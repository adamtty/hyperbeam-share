// Uses the globals exposed by `withGlobalTauri` so this page needs no bundler.
const { invoke } = window.__TAURI__.core;
const { open } = window.__TAURI__.dialog;

let selectedPath = null;

const pickBtn = document.getElementById("pick");
const pathEl = document.getElementById("path");
const subsSelect = document.getElementById("subs");
const goBtn = document.getElementById("go");
const stopBtn = document.getElementById("stop");
const logEl = document.getElementById("log");
const resultEl = document.getElementById("result");
const urlEl = document.getElementById("url");
const copyBtn = document.getElementById("copy");

pickBtn.addEventListener("click", async () => {
  const file = await open({
    multiple: false,
    filters: [{ name: "Video", extensions: ["mkv", "mp4", "mov", "avi", "webm"] }],
  });
  if (!file) return;

  selectedPath = file;
  pathEl.textContent = file;
  logEl.textContent = "Probing subtitle tracks…";

  try {
    const tracks = await invoke("probe_subtitles", { path: file });
    subsSelect.innerHTML = '<option value="">None</option>';
    for (const t of tracks) {
      const opt = document.createElement("option");
      opt.value = t.index;
      opt.textContent = `#${t.index} — ${t.codec}${t.language ? " [" + t.language + "]" : ""}${t.title ? " — " + t.title : ""}`;
      subsSelect.appendChild(opt);
    }
    subsSelect.disabled = false;
    goBtn.disabled = false;
    logEl.textContent = tracks.length
      ? `Found ${tracks.length} subtitle track(s).`
      : "No subtitle tracks found in this file.";
  } catch (e) {
    logEl.textContent = `Probe failed: ${e}`;
  }
});

goBtn.addEventListener("click", async () => {
  goBtn.disabled = true;
  pickBtn.disabled = true;
  logEl.textContent = "Remuxing and starting the tunnel — this can take a little while for large files…";
  resultEl.style.display = "none";

  const subIndex = subsSelect.value === "" ? null : parseInt(subsSelect.value, 10);

  try {
    const result = await invoke("prepare_and_share", { path: selectedPath, subIndex });
    urlEl.textContent = result.url;
    resultEl.style.display = "block";
    logEl.textContent = result.warnings.length ? result.warnings.join("\n") : "";
    stopBtn.style.display = "inline-block";
  } catch (e) {
    logEl.textContent = `Failed: ${e}`;
    goBtn.disabled = false;
    pickBtn.disabled = false;
  }
});

stopBtn.addEventListener("click", async () => {
  await invoke("stop_sharing");
  resultEl.style.display = "none";
  stopBtn.style.display = "none";
  goBtn.disabled = false;
  pickBtn.disabled = false;
  logEl.textContent = "Stopped — tunnel closed and temp files removed.";
});

copyBtn.addEventListener("click", () => {
  navigator.clipboard.writeText(urlEl.textContent);
  copyBtn.textContent = "Copied!";
  setTimeout(() => (copyBtn.textContent = "Copy URL"), 1200);
});
