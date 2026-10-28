//! Converts whatever the fuzzer makes up, as the one document of a delivery,
//! to GTFS: neither reading the delivery nor converting it may panic, and a
//! delivery that reads converts.
#![no_main]

use std::io;
use std::sync::OnceLock;

use libfuzzer_sys::fuzz_target;
use netex2gtfs::output::Output;
use netex2gtfs::{convert, Delivery, Options, Reading};
use netex_rs::dataset::Source;

/// A feed thrown away as it is written.
struct Nowhere;

impl Output for Nowhere {
    fn start(&mut self, _name: &str) -> io::Result<()> {
        Ok(())
    }

    fn write_all(&mut self, _bytes: &[u8]) -> io::Result<()> {
        Ok(())
    }

    fn finish(self: Box<Self>) -> io::Result<()> {
        Ok(())
    }
}

/// The directory the document is written to, one for the whole run.
fn directory() -> &'static tempfile::TempDir {
    static DIRECTORY: OnceLock<tempfile::TempDir> = OnceLock::new();
    DIRECTORY.get_or_init(|| tempfile::tempdir().expect("a directory"))
}

fuzz_target!(|data: &[u8]| {
    let Ok(xml) = std::str::from_utf8(data) else { return };
    let path = directory().path().join("document.xml");
    std::fs::write(&path, xml).expect("written");
    let source = Source::open(directory().path()).expect("the directory opens");
    let reading = Reading { geometry: true, ..Reading::default() };
    if let Ok(delivery) = Delivery::read(&source, reading) {
        let options = Options { shapes: true, time_zone: Some("Europe/Berlin".to_owned()), ..Options::default() };
        convert(&delivery, &options, Box::new(Nowhere)).expect("a delivery that reads converts");
    }
});
