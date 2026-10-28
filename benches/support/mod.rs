//! The documents the benchmarks read and write: a small and a large DELFI line
//! file from the fixtures.

use std::path::{Path, PathBuf};

/// One benchmark input.
pub struct Case {
    /// A short label.
    pub name: &'static str,
    /// The document.
    pub xml: String,
}

impl Case {
    /// The document's size, for throughput.
    pub fn bytes(&self) -> criterion::Throughput {
        criterion::Throughput::Bytes(self.xml.len() as u64)
    }
}

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/delfi")
        .join(name)
}

/// The smallest line file of the delivery (16 KB) and a large one (1.1 MB).
pub fn cases() -> Vec<Case> {
    [
        (
            "small-16KB",
            "NX-PI-01_DE_NAP_LINE_vgm_d-VKU_50-12816_20260919.xml",
        ),
        (
            "large-1.1MB",
            "NX-PI-01_DE_NAP_LINE_DBDB-87____-TER_20260919.xml",
        ),
    ]
    .into_iter()
    .map(|(name, file)| Case {
        name,
        xml: std::fs::read_to_string(fixture(file)).expect("the fixture is readable"),
    })
    .collect()
}
