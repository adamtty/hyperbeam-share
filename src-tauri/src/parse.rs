//! Parsing of what ffprobe and cloudflared print. ffprobe's output carries the video file's own
//! metadata (track titles, languages), so it's treated as untrusted. This module uses no Tauri, so
//! the fuzz targets in `fuzz/` include it directly.

use regex::Regex;
use serde::Serialize;
use std::sync::OnceLock;

#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct SubtitleTrack {
    pub index: u32,
    pub codec: String,
    pub language: Option<String>,
    pub title: Option<String>,
}

/// Subtitle tracks from ffprobe's `-of json` output. A track without a usable stream index is
/// dropped rather than guessed: index 0 is usually the video, and a truncated index would point
/// ffmpeg at the wrong stream.
pub fn subtitle_tracks(json: &str) -> Result<Vec<SubtitleTrack>, String> {
    let parsed: serde_json::Value = serde_json::from_str(json).map_err(|e| e.to_string())?;
    let streams = parsed["streams"].as_array().cloned().unwrap_or_default();
    Ok(streams
        .iter()
        .filter_map(|s| {
            Some(SubtitleTrack {
                index: u32::try_from(s["index"].as_u64()?).ok()?,
                codec: s["codec_name"].as_str().unwrap_or("unknown").to_string(),
                language: s["tags"]["language"].as_str().map(str::to_string),
                title: s["tags"]["title"].as_str().map(str::to_string),
            })
        })
        .collect())
}

/// How a subtitle codec is served: ASS/SSA kept for JASSUB, everything else converted to WebVTT.
pub fn subtitle_output(codec: &str) -> (&'static str, &'static str) {
    match codec {
        "ass" | "ssa" => ("ass", "subs.ass"),
        _ => ("vtt", "subs.vtt"),
    }
}

/// The quick-tunnel URL in a line of cloudflared's output, if there is one. Cloudflare's own API host
/// (api.trycloudflare.com) shows up in cloudflared's error messages and is never a tunnel.
pub fn tunnel_url(line: &str) -> Option<String> {
    static RE: OnceLock<Regex> = OnceLock::new();
    let re =
        RE.get_or_init(|| Regex::new(r"https://([a-zA-Z0-9\-]+)\.trycloudflare\.com").unwrap());
    re.captures_iter(line)
        .find(|c| !c[1].eq_ignore_ascii_case("api"))
        .map(|c| c[0].to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subtitle_tracks_from_real_ffprobe_output() {
        let json = r#"{"programs":[],"stream_groups":[],"streams":[
            {"index":2,"codec_name":"subrip","tags":{"language":"eng"}},
            {"index":3,"codec_name":"ass","tags":{"language":"jpn","title":"Signs"}}]}"#;
        let tracks = subtitle_tracks(json).unwrap();
        assert_eq!(tracks.len(), 2);
        assert_eq!(
            (
                tracks[0].index,
                tracks[0].codec.as_str(),
                tracks[0].language.as_deref()
            ),
            (2, "subrip", Some("eng"))
        );
        assert_eq!(
            (tracks[1].index, tracks[1].title.as_deref()),
            (3, Some("Signs"))
        );
    }

    #[test]
    fn tracks_without_a_usable_index_are_dropped() {
        // missing, negative, fractional and too large for u32 (which `as u32` used to wrap to 1)
        let json = r#"{"streams":[{"codec_name":"ass"},{"index":-1},{"index":1.5},{"index":4294967297},{"index":7}]}"#;
        let tracks = subtitle_tracks(json).unwrap();
        assert_eq!(tracks.iter().map(|t| t.index).collect::<Vec<_>>(), vec![7]);
        assert!(subtitle_tracks("not json").is_err());
        assert!(subtitle_tracks("[]").unwrap().is_empty());
    }

    #[test]
    fn subtitle_kinds() {
        assert_eq!(subtitle_output("ass"), ("ass", "subs.ass"));
        assert_eq!(subtitle_output("ssa"), ("ass", "subs.ass"));
        assert_eq!(subtitle_output("subrip"), ("vtt", "subs.vtt"));
        assert_eq!(subtitle_output("ass\"; rm -rf ~"), ("vtt", "subs.vtt"));
    }

    #[test]
    fn tunnel_urls() {
        let line = "2026-10-20 INF |  https://wild-river-42.trycloudflare.com  |";
        assert_eq!(
            tunnel_url(line).as_deref(),
            Some("https://wild-river-42.trycloudflare.com")
        );
        assert_eq!(tunnel_url("https://evil.example/?.trycloudflare.com"), None);
        assert_eq!(tunnel_url("http://x.trycloudflare.com"), None);
    }

    #[test]
    fn cloudflare_api_errors_are_not_share_urls() {
        // what cloudflared logs when it can't get a quick tunnel: not a URL to hand out
        let err = r#"ERR failed to request quick Tunnel: Post "https://api.trycloudflare.com/tunnel": dial tcp: lookup api.trycloudflare.com: no such host"#;
        assert_eq!(tunnel_url(err), None);
        assert_eq!(
            tunnel_url(
                "retrying https://api.trycloudflare.com then https://calm-sea-7.trycloudflare.com"
            )
            .as_deref(),
            Some("https://calm-sea-7.trycloudflare.com")
        );
    }
}
