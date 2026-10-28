//! What the repository publishes stands on its own: no reference to the
//! working directories, programs, tools or units of work it was written with,
//! no half-finished intention left in it — in its files and in the messages of
//! its commits —, and nothing of the official schemas' prose in what the crate
//! derives from them.
//!
//! The words searched for are spelled in parts, so that this file can be
//! searched like every other.

mod support;

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use netex_rs::Release;
use quick_xml::events::Event;
use quick_xml::Reader;
use support::ReleaseFixtures;

/// Markers of unfinished work, matched as words and by case.
const MARKERS: [&str; 4] = [
    concat!("TO", "DO"),
    concat!("FIX", "ME"),
    concat!("XX", "X"),
    concat!("HA", "CK"),
];

/// Paths of the machines the crate was written on, matched anywhere.
const PATHS: [&str; 3] = [
    concat!("/work", "space"),
    concat!("/single", "_repos"),
    concat!("/da", "ta/"),
];

/// Names of the programs and the repository around the crate where it was
/// written, and of assistants that may have helped write it, matched as words
/// without regard to case.
const NAMES: [&str; 8] = [
    concat!("mono", "repo"),
    concat!("pa", "pi"),
    concat!("chat", "bot"),
    concat!("clau", "de"),
    concat!("anthro", "pic"),
    concat!("co", "pilot"),
    concat!("chat", "gpt"),
    concat!("co-authored", "-by"),
];

/// A numbered unit of development work, `<word> 42`, matched without regard
/// to case.
const UNIT_OF_WORK: &str = concat!("spr", "int");

/// Directories of the repository not searched: build output, the repository's
/// own store, what the fuzz targets write as they run, and the test data under
/// a licence of its own, which is its publishers' as it was published
/// (`tests/fixtures/README.md`).
const NOT_SEARCHED: [&str; 9] = [
    "target",
    ".git",
    "fuzz/target",
    "fuzz/corpus",
    "fuzz/artifacts",
    "fuzz/coverage",
    "tests/fixtures/netex-1.3",
    "tests/fixtures/netex-2.0",
    "tests/fixtures/delfi",
];

/// Each forbidden word or path `line` holds.
fn forbidden_in(line: &str) -> Vec<String> {
    let lower = line.to_lowercase();
    let mut found = Vec::new();
    for marker in MARKERS {
        if contains_word(line, marker) {
            found.push(marker.to_owned());
        }
    }
    for path in PATHS {
        if line.contains(path) {
            found.push(path.to_owned());
        }
    }
    for name in NAMES {
        if contains_word(&lower, name) {
            found.push(name.to_owned());
        }
    }
    if let Some(unit) = unit_of_work_in(&lower) {
        found.push(unit);
    }
    found
}

/// Whether `word` stands in `text` with no letter or digit right before or
/// after it.
fn contains_word(text: &str, word: &str) -> bool {
    text.match_indices(word).any(|(at, _)| {
        let before = text[..at].chars().next_back();
        let after = text[at + word.len()..].chars().next();
        !before.is_some_and(char::is_alphanumeric) && !after.is_some_and(char::is_alphanumeric)
    })
}

/// The first unit of work `lower` names by number: the word, separators, and
/// a digit.
fn unit_of_work_in(lower: &str) -> Option<String> {
    lower.match_indices(UNIT_OF_WORK).find_map(|(at, _)| {
        let number = lower[at + UNIT_OF_WORK.len()..]
            .trim_start_matches([' ', '-', '_'])
            .split(|c: char| !c.is_ascii_alphanumeric())
            .next()?;
        number
            .starts_with(|c: char| c.is_ascii_digit())
            .then(|| format!("{UNIT_OF_WORK} {number}"))
    })
}

/// Every forbidden word or path in `text`, as `<where>:<line>: <word> — <line>`.
fn findings_in(place: &str, text: &str) -> Vec<String> {
    text.lines()
        .enumerate()
        .flat_map(|(number, line)| {
            forbidden_in(line)
                .into_iter()
                .map(move |word| format!("{place}:{}: {word} — {}", number + 1, line.trim()))
        })
        .collect()
}

/// Every file under `directory` but the directories not searched, without
/// following symbolic links.
fn files_under(root: &Path, directory: &Path, files: &mut Vec<PathBuf>) {
    let mut entries: Vec<_> = fs::read_dir(directory)
        .unwrap_or_else(|e| panic!("{}: {e}", directory.display()))
        .map(|entry| entry.expect("a directory entry"))
        .collect();
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        let kind = entry.file_type().expect("a file type");
        let path = entry.path();
        let relative = path.strip_prefix(root).expect("under the root");
        if kind.is_dir() {
            if !NOT_SEARCHED
                .iter()
                .any(|skipped| relative == Path::new(skipped))
            {
                files_under(root, &path, files);
            }
        } else if kind.is_file() {
            files.push(path);
        }
    }
}

#[test]
fn no_file_of_the_repository_refers_to_how_it_was_written() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut files = Vec::new();
    files_under(root, root, &mut files);
    for shipped in [
        "Cargo.toml",
        "README.md",
        "CHANGELOG.md",
        "LICENSE-MIT",
        "LICENSE-APACHE",
    ] {
        assert!(
            files.contains(&root.join(shipped)),
            "the search reaches {shipped}"
        );
    }
    assert!(
        files
            .iter()
            .any(|file| file.ends_with("tests/publish_hygiene.rs")),
        "the search reaches this file"
    );

    let mut findings = Vec::new();
    for file in &files {
        let relative = file.strip_prefix(root).expect("under the root");
        let place = relative.to_string_lossy();
        findings.extend(findings_in(&format!("{place} (the path)"), &place));
        // A file that is not text — an image, an archive — holds no prose.
        if let Ok(text) = fs::read_to_string(file) {
            findings.extend(findings_in(&place, &text));
        }
    }
    assert!(
        findings.is_empty(),
        "the repository refers to how it was written:\n{}",
        findings.join("\n")
    );
}

/// The messages of the commits `NETEX_COMMITS` names, a revision range as
/// `git log` takes it (`HEAD` for the whole history), are searched like the
/// files. Without the variable the history is not searched and the test says
/// so; with it, a range git cannot read fails rather than passes.
#[test]
fn no_commit_message_refers_to_how_it_was_written() {
    let Ok(range) = std::env::var("NETEX_COMMITS") else {
        println!(
            "SKIPPED: no commit messages searched; set NETEX_COMMITS to a revision range (e.g. HEAD)"
        );
        return;
    };
    let output = Command::new("git")
        .args(["log", "-z", "--format=%H%n%B", &range, "--"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("git runs");
    assert!(
        output.status.success(),
        "git log {range}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let log = String::from_utf8(output.stdout).expect("commit messages in UTF-8");
    let commits: Vec<(&str, &str)> = log
        .split('\0')
        .filter(|commit| !commit.is_empty())
        .map(|commit| commit.split_once('\n').unwrap_or((commit, "")))
        .collect();
    assert!(!commits.is_empty(), "the range {range} holds no commit");
    let findings: Vec<String> = commits
        .iter()
        .flat_map(|(hash, message)| findings_in(&format!("commit {hash}"), message))
        .collect();
    println!("{} commit messages searched in {range}", commits.len());
    assert!(
        findings.is_empty(),
        "a commit message refers to how it was written:\n{}",
        findings.join("\n")
    );
}

#[test]
fn a_planted_word_is_found_and_ordinary_prose_is_not() {
    let unit = UNIT_OF_WORK.to_uppercase();
    for planted in [
        format!("// {} read the rest", MARKERS[0]),
        format!("see {}/netex-rs/README.md", PATHS[0]),
        format!("as {}-netex orders them", NAMES[1]),
        format!("Read the {}'s rules", NAMES[1].to_uppercase()),
        format!("written with {}", NAMES[3].to_uppercase()),
        format!("{}: Someone <someone@example.org>", NAMES[7]),
        format!("added in {unit} 42"),
        format!("the {UNIT_OF_WORK}-1558 follow-up"),
    ] {
        assert_eq!(forbidden_in(&planted).len(), 1, "{planted}");
    }
    for prose in [
        "a sprinter's timetable, a sprint finish, a sprint of 4 km",
        "Papier, papillon, Hackney, todos, the frequency is 0xXXXL",
        "tests/fixtures/data_and_more, data/ without its slash before",
        "the journey planner's importer and its build",
    ] {
        assert_eq!(forbidden_in(prose), Vec::<String>::new(), "{prose}");
    }
    assert_eq!(
        findings_in("README.md", &format!("first\nsecond {}", NAMES[2])),
        [format!("README.md:2: {} — second {}", NAMES[2], NAMES[2])]
    );
}

/// The release tables hold what the schemas declare — names of elements and
/// attributes, enumeration tokens and which release declares them — and
/// nothing of what the schemas say about them: no `xsd:documentation` text,
/// nor five words in a row of one, stands in the generated table or in its
/// page.
#[test]
fn the_release_tables_hold_names_not_the_schemas_prose() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let table = fs::read_to_string(root.join("src/version/table.rs")).expect("the table");
    let page = fs::read_to_string(root.join("docs/VERSIONS.md")).expect("the page");

    let literals = string_literals(&table);
    assert!(literals.len() > 1_000, "the table lists the names");
    let not_names: Vec<&str> = literals
        .iter()
        .copied()
        .filter(|literal| !is_name_or_token(literal))
        .collect();
    assert!(
        not_names.is_empty(),
        "the table holds more than names: {not_names:?}"
    );

    for release in [Release::V1, Release::V2] {
        if !support::official_fixtures(release) {
            continue;
        }
        let documentation = schema_documentation(&release.dir().join("xsd"));
        assert!(
            documentation.len() > 1_000,
            "the {} schemas document their types",
            release.label()
        );
        let runs: HashSet<String> = documentation
            .iter()
            .flat_map(|text| word_runs(text))
            .collect();
        for (name, text) in [
            ("src/version/table.rs", &table),
            ("docs/VERSIONS.md", &page),
        ] {
            let taken: Vec<String> = word_runs(text).filter(|run| runs.contains(run)).collect();
            assert!(
                taken.is_empty(),
                "{name} repeats the {} schemas' documentation: {taken:?}",
                release.label()
            );
        }
    }
}

/// The contents of the string literals of a Rust source without escapes.
fn string_literals(source: &str) -> Vec<&str> {
    source
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .flat_map(|line| line.split('"').skip(1).step_by(2))
        .collect()
}

/// Whether `text` is an XML name or an enumeration token: no white space, no
/// sentence punctuation.
fn is_name_or_token(text: &str) -> bool {
    !text.is_empty()
        && text
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.' | ':' | '+'))
}

/// The runs of five words in a row of `text`, white space normalised.
fn word_runs(text: &str) -> impl Iterator<Item = String> + '_ {
    let words: Vec<&str> = text.split_whitespace().collect();
    (0..words.len().saturating_sub(4)).map(move |at| words[at..at + 5].join(" "))
}

/// The text of every `xsd:documentation` of the schemas under `directory`,
/// one string per element.
fn schema_documentation(directory: &Path) -> Vec<String> {
    let mut files = Vec::new();
    files_under(directory, directory, &mut files);
    let mut texts = Vec::new();
    for file in files
        .iter()
        .filter(|file| file.extension().is_some_and(|e| e == "xsd"))
    {
        let xml = fs::read_to_string(file).unwrap_or_else(|e| panic!("{}: {e}", file.display()));
        let mut reader = Reader::from_str(&xml);
        let mut depth = 0usize;
        let mut text = String::new();
        loop {
            match reader.read_event() {
                Ok(Event::Start(start)) if start.local_name().as_ref() == "documentation" => {
                    depth += 1;
                }
                Ok(Event::Start(_)) if depth > 0 => depth += 1,
                Ok(Event::End(_)) if depth > 0 => {
                    depth -= 1;
                    if depth == 0 {
                        texts.push(std::mem::take(&mut text));
                    }
                }
                Ok(Event::Text(chunk)) if depth > 0 => {
                    text.push_str(&chunk);
                    text.push(' ');
                }
                Ok(Event::CData(chunk)) if depth > 0 => {
                    text.push_str(&chunk);
                    text.push(' ');
                }
                Ok(Event::Eof) => break,
                Ok(_) => {}
                Err(e) => panic!("{}: {e}", file.display()),
            }
        }
    }
    texts
}
