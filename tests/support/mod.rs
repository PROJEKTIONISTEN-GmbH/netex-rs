//! Shared machinery for the conformance tests: the official fixtures of both
//! schema releases, semantic XML comparison, and batched schema validation.
//!
//! Cargo compiles this module separately into each integration-test binary, and no
//! single binary uses all of it.
//!
//! The comparison and the validator wrapper follow the `siri-rs` harness (its
//! `tests/support/mod.rs`); they are adapted here because NeTEx has two schema
//! releases to validate against, and because compiling a NeTEx schema costs a
//! validator run about twenty seconds, so documents are validated in batches.
#![allow(dead_code)]

pub mod delfi;
pub mod fragments;
pub mod render;
pub mod timetable;

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::Command;

use quick_xml::events::Event;
use quick_xml::name::ResolveResult;
use quick_xml::NsReader;
use quick_xml::XmlVersion;

/// The NeTEx namespace, which both releases share.
pub const NETEX_NS: &str = "http://www.netex.org.uk/netex";

pub use netex_rs::Release;

/// Where a release's fixtures are and what to call it.
pub trait ReleaseFixtures {
    /// The fixture directory of this release.
    fn dir(self) -> PathBuf;
    /// The root schema every document of this release is validated against.
    fn schema_path(self) -> PathBuf;
    /// The label the reports use.
    fn label(self) -> &'static str;
}

impl ReleaseFixtures for Release {
    fn dir(self) -> PathBuf {
        fixtures_dir().join(match self {
            // The `v1.3` branch: the 1.x line, whose publication schema is
            // versioned `1.1` and which the DELFI delivery declares as `ntx:1.1`.
            Release::V1 => "netex-1.3",
            Release::V2 => "netex-2.0",
        })
    }

    fn schema_path(self) -> PathBuf {
        self.dir().join("xsd/NeTEx_publication.xsd")
    }

    fn label(self) -> &'static str {
        match self {
            Release::V1 => "1.3",
            Release::V2 => "2.0",
        }
    }
}

/// Directory holding the official example documents and schemas.
pub fn fixtures_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

/// Whether the official schemas and examples of `release` are there.
///
/// They are published under GPL-3.0 and belong to the repository, not to the
/// crate's package; a test that needs them reads them where they are and says
/// so where they are not.
pub fn official_fixtures(release: Release) -> bool {
    let dir = release.dir();
    let present = dir.join("xsd").is_dir() && dir.join("examples").is_dir();
    if !present {
        println!(
            "SKIPPED: the official NeTEx {} schemas and examples are not at {} (see tests/fixtures/README.md)",
            release.label(),
            dir.display()
        );
    }
    present
}

/// One example document.
#[derive(Debug, Clone)]
pub struct Fixture {
    /// Path relative to the release's example directory, used as the test's label.
    pub name: String,
    /// Absolute path on disk.
    pub path: PathBuf,
    /// The document itself.
    pub xml: String,
}

impl Fixture {
    /// Name of the document's root element, without its namespace prefix.
    pub fn root(&self) -> String {
        root_element(&self.xml)
    }

    /// Whether the root element is in the NeTEx namespace.
    ///
    /// The example directories also hold SIRI request envelopes, TransXChange and
    /// NaPTAN documents; those are not NeTEx and the harness lists them as skipped.
    pub fn is_netex(&self) -> bool {
        root_namespace(&self.xml).as_deref() == Some(NETEX_NS)
    }
}

/// Every official example document of `release`, in a stable order.
pub fn fixtures(release: Release) -> Vec<Fixture> {
    fixtures_under(&release.dir().join("examples"))
}

/// The `should-fail` examples of the 2.0 release: documents the schema must reject.
pub fn should_fail_fixtures() -> Vec<Fixture> {
    fixtures_under(&Release::V2.dir().join("examples/should-fail"))
}

/// The DELFI line files kept as fixtures.
pub fn delfi_fixtures() -> Vec<Fixture> {
    fixtures_under(&fixtures_dir().join("delfi"))
}

/// The crate's own examples under `tests/fixtures/own/`, of one profile's
/// directory (`epip`, `at`, `ch`) or, given `""`, of all.
pub fn own_fixtures(profile: &str) -> Vec<Fixture> {
    fixtures_under(&fixtures_dir().join("own").join(profile))
}

fn fixtures_under(root: &Path) -> Vec<Fixture> {
    let mut paths = Vec::new();
    collect_xml(root, &mut paths);
    paths.sort();
    assert!(
        !paths.is_empty(),
        "no fixtures found under {}",
        root.display()
    );
    paths
        .into_iter()
        .map(|path| Fixture {
            name: path
                .strip_prefix(root)
                .expect("fixture lives under the fixture root")
                .to_string_lossy()
                .into_owned(),
            xml: std::fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display())),
            path,
        })
        .collect()
}

fn collect_xml(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in
        std::fs::read_dir(dir).unwrap_or_else(|e| panic!("cannot read {}: {e}", dir.display()))
    {
        let path = entry.expect("readable directory entry").path();
        if path.is_dir() {
            collect_xml(&path, out);
        } else if path.extension().is_some_and(|e| e == "xml") {
            out.push(path);
        }
    }
}

/// Name of a document's root element, without its namespace prefix.
pub fn root_element(xml: &str) -> String {
    let mut reader = quick_xml::Reader::from_str(xml);
    loop {
        match reader.read_event().expect("fixture is well-formed XML") {
            Event::Start(e) | Event::Empty(e) => return e.local_name().as_ref().to_owned(),
            Event::Eof => panic!("document has no elements"),
            _ => {}
        }
    }
}

/// Namespace of a document's root element, if it is in one.
pub fn root_namespace(xml: &str) -> Option<String> {
    let mut reader = NsReader::from_str(xml);
    loop {
        match reader
            .read_resolved_event()
            .expect("fixture is well-formed XML")
        {
            (ns, Event::Start(_)) | (ns, Event::Empty(_)) => {
                return match ns {
                    ResolveResult::Bound(ns) => Some(ns.as_ref().to_owned()),
                    _ => None,
                }
            }
            (_, Event::Eof) => panic!("document has no elements"),
            _ => {}
        }
    }
}

/// An XML element reduced to what the schema treats as content.
///
/// Namespace declarations and `xsi:*` attributes are dropped: they carry no
/// content, and `xsi:schemaLocation` in particular is a hint pointing at wherever
/// the author happened to keep the schemas. Everything that does carry content —
/// the resolved namespace and local name of each element, its attributes, its
/// character data and the order of its children — is kept.
#[derive(Debug, PartialEq)]
pub struct Element {
    /// `{namespace}local-name`.
    pub name: String,
    /// Attributes by `{namespace}local-name`, namespace declarations left out.
    pub attributes: BTreeMap<String, String>,
    /// Character data directly inside the element, whitespace trimmed.
    pub text: String,
    /// Child elements in document order.
    pub children: Vec<Element>,
}

/// Parses a document into its content tree.
pub fn parse(xml: &str) -> Element {
    let mut reader = NsReader::from_str(xml);
    let mut stack: Vec<Element> = Vec::new();
    let mut root = None;

    loop {
        let (namespace, event) = reader
            .read_resolved_event()
            .unwrap_or_else(|e| panic!("document is well-formed XML: {e}"));
        match event {
            Event::Start(start) => {
                stack.push(start_element(&namespace, &start));
            }
            Event::Empty(start) => {
                let element = start_element(&namespace, &start);
                push(&mut stack, &mut root, element);
            }
            Event::End(_) => {
                let element = stack.pop().expect("end tag closes an open element");
                push(&mut stack, &mut root, element);
            }
            Event::Text(text) => {
                if let Some(open) = stack.last_mut() {
                    open.text.push_str(text.xml10_content().as_ref());
                }
            }
            Event::CData(data) => {
                if let Some(open) = stack.last_mut() {
                    open.text.push_str(data.xml10_content().as_ref());
                }
            }
            Event::GeneralRef(reference) => {
                if let Some(open) = stack.last_mut() {
                    open.text.push_str(&resolve_reference(reference.as_ref()));
                }
            }
            Event::Eof => break,
            _ => {}
        }
    }
    root.expect("document has a root element")
}

/// The character a predefined or numeric reference denotes.
fn resolve_reference(name: &str) -> String {
    match name {
        "amp" => "&".to_owned(),
        "lt" => "<".to_owned(),
        "gt" => ">".to_owned(),
        "quot" => "\"".to_owned(),
        "apos" => "'".to_owned(),
        _ => {
            let code = name
                .strip_prefix("#x")
                .map(|hex| u32::from_str_radix(hex, 16))
                .or_else(|| name.strip_prefix('#').map(str::parse))
                .unwrap_or_else(|| panic!("entity &{name}; is not predefined"))
                .expect("character reference is a number");
            char::from_u32(code)
                .expect("character reference is a scalar")
                .to_string()
        }
    }
}

fn push(stack: &mut [Element], root: &mut Option<Element>, mut element: Element) {
    element.text = element.text.trim().to_owned();
    match stack.last_mut() {
        Some(parent) => parent.children.push(element),
        None => *root = Some(element),
    }
}

fn start_element(
    namespace: &ResolveResult<'_>,
    start: &quick_xml::events::BytesStart<'_>,
) -> Element {
    let local = start.local_name().as_ref().to_owned();
    let name = match namespace {
        ResolveResult::Bound(ns) => format!("{{{}}}{local}", ns.as_ref()),
        _ => local,
    };
    let mut attributes = BTreeMap::new();
    for attribute in start.attributes() {
        let attribute = attribute.expect("attribute is well-formed");
        let (namespace, local) = namespace_of_attribute(attribute.key);
        if namespace == Some("http://www.w3.org/2000/xmlns/")
            || namespace == Some("http://www.w3.org/2001/XMLSchema-instance")
            || attribute.key.as_ref() == "xmlns"
        {
            continue;
        }
        let key = match namespace {
            Some(ns) => format!("{{{ns}}}{local}"),
            None => local,
        };
        let value = attribute
            .normalized_value(XmlVersion::Explicit1_0)
            .expect("attribute value unescapes")
            .into_owned();
        attributes.insert(key, value);
    }
    Element {
        name,
        attributes,
        text: String::new(),
        children: Vec::new(),
    }
}

/// Namespace and local name of an attribute.
///
/// Attributes are not in the default namespace; only prefixed ones are in any.
/// The prefixes that matter to the comparison — `xmlns` and `xsi` — are fixed by
/// convention in every fixture, so they are recognised by prefix.
fn namespace_of_attribute(key: quick_xml::name::QName<'_>) -> (Option<&'static str>, String) {
    let (local, prefix) = key.decompose();
    let local = local.as_ref().to_owned();
    match prefix.map(|p| p.as_ref().to_owned()) {
        Some(p) if p == "xmlns" => (Some("http://www.w3.org/2000/xmlns/"), local),
        Some(p) if p == "xsi" => (Some("http://www.w3.org/2001/XMLSchema-instance"), local),
        Some(p) if p == "xml" => (Some("http://www.w3.org/XML/1998/namespace"), local),
        Some(p) => (None, format!("{p}:{local}")),
        None => (None, local),
    }
}

/// Compares two content trees, naming the first difference by path.
pub fn compare(expected: &Element, actual: &Element) -> Result<(), String> {
    let mut path = String::new();
    compare_at(expected, actual, &mut path)
}

fn compare_at(expected: &Element, actual: &Element, path: &mut String) -> Result<(), String> {
    let _ = write!(path, "/{}", short_name(&expected.name));
    let here = path.clone();

    if expected.name != actual.name {
        return Err(format!(
            "{here}: element is <{}>, expected <{}>",
            actual.name, expected.name
        ));
    }
    if expected.attributes != actual.attributes {
        return Err(format!(
            "{here}: attributes are {:?}, expected {:?}",
            actual.attributes, expected.attributes
        ));
    }
    if !text_matches(&expected.text, &actual.text) {
        return Err(format!(
            "{here}: content is {:?}, expected {:?}",
            actual.text, expected.text
        ));
    }
    if expected.children.len() != actual.children.len() {
        return Err(format!(
            "{here}: has {} children {:?}, expected {} children {:?}",
            actual.children.len(),
            child_names(actual),
            expected.children.len(),
            child_names(expected),
        ));
    }

    for (expected_child, actual_child) in expected.children.iter().zip(&actual.children) {
        let mut child_path = here.clone();
        compare_at(expected_child, actual_child, &mut child_path)?;
    }
    Ok(())
}

fn short_name(name: &str) -> &str {
    name.rsplit('}').next().unwrap_or(name)
}

fn child_names(element: &Element) -> Vec<&str> {
    element
        .children
        .iter()
        .map(|c| short_name(&c.name))
        .collect()
}

/// Whether two lexical values denote the same content.
///
/// Equal text is equal content. Beyond that, the two spellings of a boolean and
/// numerically equal decimals are the same content: `1` and `true`, `7.7660340`
/// and `7.766034`. Nothing else is folded — a duration or a date written another
/// way is a difference the harness reports.
fn text_matches(expected: &str, actual: &str) -> bool {
    if expected == actual {
        return true;
    }
    if let (Some(a), Some(b)) = (boolean(expected), boolean(actual)) {
        return a == b;
    }
    match (expected.parse::<f64>(), actual.parse::<f64>()) {
        (Ok(a), Ok(b)) => a == b,
        _ => false,
    }
}

fn boolean(text: &str) -> Option<bool> {
    match text {
        "true" | "1" => Some(true),
        "false" | "0" => Some(false),
        _ => None,
    }
}

/// Whether an XML Schema validator is available.
pub fn validator_available() -> bool {
    Command::new("xmllint")
        .arg("--version")
        .output()
        .is_ok_and(|output| output.status.success())
}

/// Why the schema-validation tests cannot run.
pub const VALIDATOR_MISSING: &str = "\
`xmllint` is not on PATH, so the schema-validation tests cannot run.
Install libxml2's command-line tools (Debian: apt install libxml2-utils).";

/// One document handed to the validator, with the label its verdict is reported under.
#[derive(Debug, Clone)]
pub struct Candidate {
    /// The label.
    pub name: String,
    /// The document.
    pub xml: String,
}

/// Validates every candidate against the schema of `release` and returns the
/// complaints, by label, for those the validator rejects.
///
/// The validator compiles the schema once per invocation, which for NeTEx takes
/// longer than validating a hundred documents, so the candidates are validated
/// in a few invocations of many files each, run side by side. libxml2 carries
/// identity-constraint state from one document of an invocation into the next,
/// so a keyref complaint can depend on a document's neighbours; every
/// complaint about an identity constraint is therefore checked again with the
/// document validated on its own, and only kept if it stands alone.
pub fn validate_all(release: Release, candidates: &[Candidate]) -> BTreeMap<String, String> {
    let batched = validate_batched(release, candidates);
    let complaints: Vec<(String, String)> = batched.into_iter().collect();
    // Only identity-constraint complaints can be the neighbours' doing. Each
    // single re-check compiles the schema again and holds a few hundred
    // megabytes, so at most four run at a time.
    let mut confirmed = Vec::with_capacity(complaints.len());
    for chunk in complaints.chunks(4) {
        let results: Vec<bool> = std::thread::scope(|scope| {
            let handles: Vec<_> = chunk
                .iter()
                .map(|(name, complaint)| {
                    let candidate = candidates
                        .iter()
                        .find(|c| &c.name == name)
                        .expect("a complaint names a candidate");
                    let about_keys = complaint.contains("identity-constraint")
                        || complaint.contains("key-sequence");
                    scope.spawn(move || {
                        !about_keys
                            || validate_batched(release, std::slice::from_ref(candidate))
                                .contains_key(name)
                    })
                })
                .collect();
            handles
                .into_iter()
                .map(|h| h.join().expect("validator thread"))
                .collect()
        });
        confirmed.extend(results);
    }
    complaints
        .into_iter()
        .zip(confirmed)
        .filter(|(_, stands)| *stands)
        .map(|(c, _)| c)
        .collect()
}

/// Validates every candidate in batches only, without the single re-check of
/// identity-constraint complaints: for counts that are reported, not asserted.
pub fn validate_all_batched(
    release: Release,
    candidates: &[Candidate],
) -> BTreeMap<String, String> {
    validate_batched(release, candidates)
}

fn validate_batched(release: Release, candidates: &[Candidate]) -> BTreeMap<String, String> {
    let dir = temp_dir();
    let mut files = Vec::new();
    for (index, candidate) in candidates.iter().enumerate() {
        let path = dir.join(format!("{index}.xml"));
        std::fs::write(&path, &candidate.xml).expect("temporary file is writable");
        files.push(path);
    }

    let batches = std::thread::available_parallelism().map_or(1, |n| n.get().min(8));
    let chunk = files.len().div_ceil(batches).max(1);
    let outputs: Vec<String> = std::thread::scope(|scope| {
        let handles: Vec<_> = files
            .chunks(chunk)
            .map(|chunk| {
                let schema = release.schema_path();
                scope.spawn(move || {
                    let output = Command::new("xmllint")
                        .arg("--noout")
                        .arg("--schema")
                        .arg(schema)
                        .args(chunk)
                        .output()
                        .expect("xmllint runs");
                    String::from_utf8_lossy(&output.stderr).into_owned()
                })
            })
            .collect();
        handles
            .into_iter()
            .map(|h| h.join().expect("validator thread"))
            .collect()
    });

    let mut complaints: BTreeMap<String, String> = BTreeMap::new();
    for output in outputs {
        for line in output.lines() {
            let Some((file, rest)) = line.split_once(':').or_else(|| line.split_once(' ')) else {
                continue;
            };
            let Some(index) = Path::new(file)
                .file_stem()
                .and_then(|s| s.to_str())
                .and_then(|s| s.parse::<usize>().ok())
            else {
                continue;
            };
            let Some(candidate) = candidates.get(index) else {
                continue;
            };
            if rest.trim() == "validates" {
                continue;
            }
            let entry = complaints.entry(candidate.name.clone()).or_default();
            entry.push_str(rest.trim());
            entry.push('\n');
        }
    }
    for file in &files {
        let _ = std::fs::remove_file(file);
    }
    let _ = std::fs::remove_dir(&dir);
    complaints
}

/// Validates one document against the schema of `release`.
pub fn validate(release: Release, xml: &str) -> Result<(), String> {
    let candidate = Candidate {
        name: "document".to_owned(),
        xml: xml.to_owned(),
    };
    let complaints = validate_all(release, std::slice::from_ref(&candidate));
    match complaints.into_values().next() {
        None => Ok(()),
        Some(complaint) => Err(complaint),
    }
}

/// A uniquely named directory next to the other test temporaries.
fn temp_dir() -> PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);

    let path = std::env::temp_dir().join(format!(
        "netex-conformance-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&path).expect("temporary directory is creatable");
    path
}

/// Renders a failure list for an assertion message: the count, then each entry.
pub fn report(what: &str, failures: &[String]) -> String {
    let mut out = format!("{} {what}:\n", failures.len());
    for failure in failures {
        out.push_str(failure);
        out.push('\n');
    }
    out
}

/// The DELFI fixtures written into a zip archive under `dir`, each entry
/// stored as it is, the way the national delivery packs its line files.
pub fn zip_of_delfi_fixtures(dir: &Path) -> PathBuf {
    zip_of(dir, "delfi.zip", &delfi_fixtures())
}

/// `fixtures` written into a zip archive at `dir/name`, entries stored.
pub fn zip_of(dir: &Path, name: &str, fixtures: &[Fixture]) -> PathBuf {
    std::fs::create_dir_all(dir).expect("the directory is created");
    let path = dir.join(name);
    let file = std::fs::File::create(&path).expect("the archive is created");
    let mut writer = zip::ZipWriter::new(file);
    let options =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    for fixture in fixtures {
        writer
            .start_file(fixture.name.as_str(), options)
            .expect("the entry starts");
        std::io::Write::write_all(&mut writer, fixture.xml.as_bytes())
            .expect("the entry is written");
    }
    writer.finish().expect("the archive is finished");
    path
}

/// A fresh directory under the target directory for a test's scratch files.
pub fn scratch_dir(test: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target/test-scratch")
        .join(test);
    if dir.exists() {
        std::fs::remove_dir_all(&dir).expect("the old scratch directory is removed");
    }
    std::fs::create_dir_all(&dir).expect("the scratch directory is created");
    dir
}
