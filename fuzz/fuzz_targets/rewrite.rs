//! Detects the release of whatever the fuzzer makes up and rewrites it for
//! both releases: the scanners must never panic on any input.
#![no_main]

use libfuzzer_sys::fuzz_target;
use netex_rs::version::{detect, rewrite};
use netex_rs::Release;

fuzz_target!(|data: &[u8]| {
    let Ok(xml) = std::str::from_utf8(data) else { return };
    let _ = detect(xml);
    for release in [Release::V1, Release::V2] {
        let written = rewrite(xml, release);
        assert!(written.xml.len() <= xml.len() + 32, "a rewrite adds at most a version attribute");
    }
});
