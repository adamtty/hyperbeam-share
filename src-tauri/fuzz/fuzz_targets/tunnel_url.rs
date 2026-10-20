// Whatever cloudflared prints, the share URL is exactly https://<letters, digits, dashes>.trycloudflare.com
// and really appears in the line: output can never point the share at another host.
#![no_main]
use libfuzzer_sys::fuzz_target;

#[path = "../../src/parse.rs"]
#[allow(dead_code)] // each target uses part of it
mod parse;

fuzz_target!(|data: &[u8]| {
    let line = String::from_utf8_lossy(data);
    if let Some(url) = parse::tunnel_url(&line) {
        let host = url
            .strip_prefix("https://")
            .and_then(|h| h.strip_suffix(".trycloudflare.com"));
        let host = host.expect("not an https trycloudflare.com URL");
        assert!(
            !host.is_empty() && host.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-'),
            "{url}"
        );
        assert!(line.contains(&url));
    }
});
