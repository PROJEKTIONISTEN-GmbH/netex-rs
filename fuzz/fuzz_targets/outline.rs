//! Outlines whatever the fuzzer makes up, as `netex extract` and `netex merge`
//! read a document: no panic, and every position found stands within the
//! document on a character boundary, an element's identifier within it.
#![no_main]

use libfuzzer_sys::fuzz_target;
use netex_rs::written::outline::Outline;

fuzz_target!(|data: &[u8]| {
    let Ok(xml) = std::str::from_utf8(data) else { return };
    let Ok(outline) = Outline::of(xml) else { return };
    let within = |range: &std::ops::Range<usize>| {
        range.start <= range.end && xml.get(range.clone()).is_some()
    };
    for object in &outline.objects {
        assert!(within(&object.range) && within(&object.id_at));
        assert!(object.range.start <= object.id_at.start && object.id_at.end <= object.range.end);
        assert_eq!(&xml[object.id_at.clone()], object.id);
        for reference in &object.references {
            assert!(within(&reference.at));
            assert_eq!(&xml[reference.at.clone()], reference.id);
        }
        for call in &object.calls {
            assert!(within(&call.range));
        }
    }
    for container in &outline.containers {
        assert!(within(&container.range));
    }
});
