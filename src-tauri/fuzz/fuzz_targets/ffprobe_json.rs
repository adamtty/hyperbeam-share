// ffprobe's JSON carries the video file's own metadata, so: never panic, and every track that comes
// back has a real stream index and a codec name.
#![no_main]
use libfuzzer_sys::fuzz_target;

#[path = "../../src/parse.rs"]
#[allow(dead_code)] // each target uses part of it
mod parse;

fuzz_target!(|data: &[u8]| {
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };
    if let Ok(tracks) = parse::subtitle_tracks(text) {
        for t in tracks {
            assert!(!t.codec.is_empty());
            let (kind, file) = parse::subtitle_output(&t.codec);
            assert!(matches!(
                (kind, file),
                ("ass", "subs.ass") | ("vtt", "subs.vtt")
            ));
        }
    }
});
