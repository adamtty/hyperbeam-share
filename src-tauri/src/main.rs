// Remux a local video, extract subtitles, serve it locally, tunnel it out,
// hand you a URL for Hyperbeam.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use regex::Regex;
use serde::Serialize;
use tauri::async_runtime::JoinHandle;
use tauri::{AppHandle, Manager, RunEvent, State};
use tauri_plugin_shell::process::{CommandChild, CommandEvent};
use tauri_plugin_shell::ShellExt;
use tower_http::services::ServeDir;

const PLAYER_HTML: &str = include_str!("../assets/player.html");

const JASSUB_ASSETS: &[(&str, &[u8])] = &[
    ("jassub.js", include_bytes!("../assets/jassub/jassub.js")),
    (
        "jassub-worker.js",
        include_bytes!("../assets/jassub/jassub-worker.js"),
    ),
    (
        "jassub-worker.wasm",
        include_bytes!("../assets/jassub/jassub-worker.wasm"),
    ),
    (
        "jassub-worker-modern.wasm",
        include_bytes!("../assets/jassub/jassub-worker-modern.wasm"),
    ),
    (
        "default.woff2",
        include_bytes!("../assets/jassub/default.woff2"),
    ),
];

/// Codecs Chromium (which Hyperbeam runs) can play from an MP4 without re-encoding.
const BROWSER_VIDEO_CODECS: &[&str] = &["h264", "hevc", "av1", "vp9"];

const TUNNEL_TIMEOUT: Duration = Duration::from_secs(45);

#[derive(Default)]
struct ShareState(Mutex<Option<ActiveShare>>);

struct ActiveShare {
    dir: PathBuf,
    server: JoinHandle<()>,
    tunnel: CommandChild,
}

impl ActiveShare {
    fn shutdown(self) {
        let _ = self.tunnel.kill();
        self.server.abort();
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

#[derive(Serialize)]
struct ShareResult {
    url: String,
    warnings: Vec<String>,
}

#[derive(Serialize, Clone)]
struct SubtitleTrack {
    index: u32,
    codec: String,
    language: Option<String>,
    title: Option<String>,
}

async fn ffprobe(app: &AppHandle, args: &[&str]) -> Result<String, String> {
    let out = app
        .shell()
        .sidecar("ffprobe")
        .map_err(|e| e.to_string())?
        .args(args)
        .output()
        .await
        .map_err(|e| format!("couldn't run ffprobe: {e}"))?;
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).trim().to_string());
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

async fn run_ffmpeg(app: &AppHandle, args: &[&str]) -> Result<(), String> {
    let out = app
        .shell()
        .sidecar("ffmpeg")
        .map_err(|e| e.to_string())?
        .args(args)
        .output()
        .await
        .map_err(|e| format!("couldn't run ffmpeg: {e}"))?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        // ffmpeg's last few lines hold the actual error; the rest is banner.
        let tail: Vec<&str> = stderr.lines().rev().take(4).collect();
        return Err(tail.into_iter().rev().collect::<Vec<_>>().join("\n"));
    }
    Ok(())
}

/// First stream's codec_name for a selector like "a:0", or None if absent.
async fn probe_codec(
    app: &AppHandle,
    path: &str,
    selector: &str,
) -> Result<Option<String>, String> {
    let codec = ffprobe(
        app,
        &[
            "-v",
            "error",
            "-select_streams",
            selector,
            "-show_entries",
            "stream=codec_name",
            "-of",
            "default=nw=1:nk=1",
            path,
        ],
    )
    .await?;
    Ok(codec
        .lines()
        .next()
        .map(str::to_string)
        .filter(|c| !c.is_empty()))
}

#[tauri::command]
async fn probe_subtitles(app: AppHandle, path: String) -> Result<Vec<SubtitleTrack>, String> {
    let json = ffprobe(
        &app,
        &[
            "-v",
            "error",
            "-select_streams",
            "s",
            "-show_entries",
            "stream=index,codec_name:stream_tags=language,title",
            "-of",
            "json",
            &path,
        ],
    )
    .await?;
    let parsed: serde_json::Value = serde_json::from_str(&json).map_err(|e| e.to_string())?;
    let streams = parsed["streams"].as_array().cloned().unwrap_or_default();
    Ok(streams
        .iter()
        .map(|s| SubtitleTrack {
            index: s["index"].as_u64().unwrap_or(0) as u32,
            codec: s["codec_name"].as_str().unwrap_or("unknown").to_string(),
            language: s["tags"]["language"].as_str().map(str::to_string),
            title: s["tags"]["title"].as_str().map(str::to_string),
        })
        .collect())
}

fn write_jassub_assets(dir: &Path) -> Result<(), String> {
    for (name, bytes) in JASSUB_ASSETS {
        std::fs::write(dir.join(name), bytes).map_err(|e| e.to_string())?;
    }
    Ok(())
}

async fn remux_video(
    app: &AppHandle,
    path: &str,
    out: &Path,
    warnings: &mut Vec<String>,
) -> Result<(), String> {
    let video = probe_codec(app, path, "v:0")
        .await?
        .ok_or("No video stream found in this file.")?;
    let audio = probe_codec(app, path, "a:0").await?;

    let out = out.to_string_lossy().to_string();
    let mut args = vec!["-y", "-i", path, "-map", "0:v:0", "-c:v", "copy"];
    if video == "hevc" {
        // Chromium only plays HEVC-in-MP4 tagged as hvc1.
        args.extend(["-tag:v", "hvc1"]);
    } else if !BROWSER_VIDEO_CODECS.contains(&video.as_str()) {
        warnings.push(format!(
            "Video is {video}, which browsers usually can't play. It was copied as-is; re-encode to H.264 if Hyperbeam shows a black screen."
        ));
    }
    match audio.as_deref() {
        None => warnings.push("No audio track found in this file.".into()),
        Some("aac") => args.extend(["-map", "0:a:0", "-c:a", "copy"]),
        Some(_) => args.extend(["-map", "0:a:0", "-c:a", "aac", "-b:a", "192k", "-ac", "2"]),
    }
    args.extend([
        "-sn",
        "-dn",
        "-map_chapters",
        "-1",
        "-movflags",
        "+faststart",
        out.as_str(),
    ]);
    run_ffmpeg(app, &args).await
}

/// Extracts the chosen subtitle track; returns the player's SUB_KIND.
async fn extract_subtitles(
    app: &AppHandle,
    path: &str,
    sub_index: u32,
    dir: &Path,
) -> Result<&'static str, String> {
    let tracks = probe_subtitles(app.clone(), path.to_string()).await?;
    let track = tracks
        .iter()
        .find(|t| t.index == sub_index)
        .ok_or(format!("Subtitle track #{sub_index} not found."))?;
    let (kind, file) = match track.codec.as_str() {
        "ass" | "ssa" => ("ass", "subs.ass"),
        _ => ("vtt", "subs.vtt"),
    };
    let map = format!("0:{sub_index}");
    let out = dir.join(file).to_string_lossy().to_string();
    let codec = if kind == "ass" { "copy" } else { "webvtt" };
    run_ffmpeg(app, &["-y", "-i", path, "-map", &map, "-c:s", codec, &out]).await?;
    Ok(kind)
}

async fn start_tunnel(app: &AppHandle, port: u16) -> Result<(String, CommandChild), String> {
    let (mut rx, child) = app
        .shell()
        .sidecar("cloudflared")
        .map_err(|e| e.to_string())?
        .args([
            "tunnel",
            "--no-autoupdate",
            "--url",
            &format!("http://127.0.0.1:{port}"),
        ])
        .spawn()
        .map_err(|e| format!("couldn't start cloudflared: {e}"))?;

    let url_re = Regex::new(r"https://[a-zA-Z0-9\-]+\.trycloudflare\.com").unwrap();
    let found = tokio::time::timeout(TUNNEL_TIMEOUT, async {
        while let Some(event) = rx.recv().await {
            match event {
                CommandEvent::Stdout(line) | CommandEvent::Stderr(line) => {
                    if let Some(m) = url_re.find(&String::from_utf8_lossy(&line)) {
                        return Some(m.as_str().to_string());
                    }
                }
                CommandEvent::Terminated(_) => return None,
                _ => {}
            }
        }
        None
    })
    .await
    .ok()
    .flatten();

    let Some(url) = found else {
        let _ = child.kill();
        return Err("cloudflared never reported a public URL (check your network, or that quick tunnels aren't being rate-limited).".into());
    };
    // Keep draining cloudflared's output so it never blocks on a full pipe.
    tauri::async_runtime::spawn(async move { while rx.recv().await.is_some() {} });
    Ok((url, child))
}

/// A new temp directory for one share: an unguessable name, owner-only, and creation fails if the path
/// already exists, so on a shared /tmp another user can't pre-create it (or a symlink) to see or swap
/// what gets served.
fn fresh_share_dir() -> Result<PathBuf, String> {
    use std::collections::hash_map::RandomState;
    use std::hash::{BuildHasher, Hasher};
    for _ in 0..8 {
        let mut h = RandomState::new().build_hasher(); // randomly keyed per process
        h.write_u128(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
        );
        h.write_u32(std::process::id());
        let dir = std::env::temp_dir().join(format!("hb-share-{:016x}", h.finish()));
        let mut builder = std::fs::DirBuilder::new();
        #[cfg(unix)]
        std::os::unix::fs::DirBuilderExt::mode(&mut builder, 0o700);
        match builder.create(&dir) {
            Ok(()) => return Ok(dir),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(e.to_string()),
        }
    }
    Err("couldn't create a temporary directory".into())
}

#[tauri::command]
async fn prepare_and_share(
    app: AppHandle,
    state: State<'_, ShareState>,
    path: String,
    sub_index: Option<u32>,
) -> Result<ShareResult, String> {
    if let Some(old) = state.0.lock().unwrap().take() {
        old.shutdown();
    }

    let dir = fresh_share_dir()?;

    let result = prepare_dir(&app, &path, sub_index, &dir).await;
    let warnings = match result {
        Ok(w) => w,
        Err(e) => {
            let _ = std::fs::remove_dir_all(&dir);
            return Err(e);
        }
    };

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .map_err(|e| e.to_string())?;
    let port = listener.local_addr().map_err(|e| e.to_string())?.port();
    // ServeDir handles Range requests, which <video> seeking relies on.
    let router = axum::Router::new().fallback_service(ServeDir::new(&dir));
    let server = tauri::async_runtime::spawn(async move {
        let _ = axum::serve(listener, router).await;
    });

    let (url, tunnel) = match start_tunnel(&app, port).await {
        Ok(t) => t,
        Err(e) => {
            server.abort();
            let _ = std::fs::remove_dir_all(&dir);
            return Err(e);
        }
    };

    *state.0.lock().unwrap() = Some(ActiveShare {
        dir,
        server,
        tunnel,
    });
    Ok(ShareResult {
        url: format!("{url}/"),
        warnings,
    })
}

/// Fills `dir` with video.mp4, optional subs, index.html and any player assets.
async fn prepare_dir(
    app: &AppHandle,
    path: &str,
    sub_index: Option<u32>,
    dir: &Path,
) -> Result<Vec<String>, String> {
    let mut warnings = Vec::new();
    remux_video(app, path, &dir.join("video.mp4"), &mut warnings).await?;

    let mut sub_kind = "none";
    if let Some(index) = sub_index {
        match extract_subtitles(app, path, index, dir).await {
            Ok(kind) => sub_kind = kind,
            Err(e) => warnings.push(format!(
                "Subtitle extraction failed — the track is probably image-based (PGS/VobSub), which needs OCR rather than a straight extract. ({e})"
            )),
        }
    }
    if sub_kind == "ass" {
        write_jassub_assets(dir)?;
    }

    let html = PLAYER_HTML.replace("__SUB_KIND__", sub_kind);
    std::fs::write(dir.join("index.html"), html).map_err(|e| e.to_string())?;
    Ok(warnings)
}

#[tauri::command]
fn stop_sharing(state: State<'_, ShareState>) {
    if let Some(share) = state.0.lock().unwrap().take() {
        share.shutdown();
    }
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .manage(ShareState::default())
        .invoke_handler(tauri::generate_handler![
            probe_subtitles,
            prepare_and_share,
            stop_sharing
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| {
            // Don't leave a tunnel or temp files behind when the app quits.
            if let RunEvent::Exit = event {
                if let Some(share) = app.state::<ShareState>().0.lock().unwrap().take() {
                    share.shutdown();
                }
            }
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn share_dirs_are_new_private_and_never_reused() {
        let a = fresh_share_dir().unwrap();
        let b = fresh_share_dir().unwrap();
        assert_ne!(a, b);
        assert!(a
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .starts_with("hb-share-"));
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                std::fs::metadata(&a).unwrap().permissions().mode() & 0o777,
                0o700
            );
        }
        // the exclusive create that fresh_share_dir relies on refuses an existing path
        assert_eq!(
            std::fs::DirBuilder::new().create(&a).unwrap_err().kind(),
            std::io::ErrorKind::AlreadyExists
        );
        std::fs::remove_dir(&a).unwrap();
        std::fs::remove_dir(&b).unwrap();
    }
}
