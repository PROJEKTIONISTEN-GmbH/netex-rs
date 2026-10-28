//! Reads whatever the fuzzer makes up, with tight limits, and writes back
//! whatever reads: the reader must never panic, and what it reads must write.
#![no_main]

use libfuzzer_sys::fuzz_target;
use netex_rs::Limits;

fuzz_target!(|data: &[u8]| {
    let Ok(xml) = std::str::from_utf8(data) else { return };
    let limits = Limits { max_depth: 64, max_text: 1 << 16, max_attributes: 64, max_fragment: 1 << 20 };
    if let Ok(document) = netex_rs::read_with(xml, &limits) {
        let written = document.write();
        // What the crate wrote, it reads again.
        let again = netex_rs::read_with(&written, &limits).expect("the crate reads what it wrote");
        assert_eq!(again.write(), written, "writing is stable");
        let _ = netex_rs::check::duplicate_ids(&document);
    }
    let _ = netex_rs::Fragment::read_with(xml, &limits);
});
