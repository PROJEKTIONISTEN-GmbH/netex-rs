//! An inventory of a dataset: which elements and attributes occur, how often,
//! under which parents, and how much of it the typed model carries through
//! untyped.
//!
//! ```sh
//! cargo run --release --example inventory -- /path/to/netex.zip > docs/DELFI-PROFILE.md
//! ```
//!
//! Every document is tokenised once for the counts and read once into the
//! typed model for the untyped share; both run on every core.

use std::collections::BTreeMap;
use std::time::Instant;

use netex_rs::dataset::Source;
use netex_rs::Limits;
use quick_xml::events::Event;
use quick_xml::Reader;
use rayon::prelude::*;

#[derive(Default)]
struct Counts {
    files: usize,
    bytes: u64,
    /// Element name → occurrences, files it occurs in.
    elements: BTreeMap<String, (u64, u64)>,
    /// (parent, child) → occurrences.
    children: BTreeMap<(String, String), u64>,
    /// (element, attribute) → occurrences.
    attributes: BTreeMap<(String, String), u64>,
    /// (parent, untyped child) → occurrences after the typed read.
    unprocessed: BTreeMap<(String, String), u64>,
    unreadable: Vec<String>,
}

impl Counts {
    fn merge(mut self, other: Counts) -> Counts {
        self.files += other.files;
        self.bytes += other.bytes;
        for (name, (count, files)) in other.elements {
            let entry = self.elements.entry(name).or_default();
            entry.0 += count;
            entry.1 += files;
        }
        for (key, count) in other.children {
            *self.children.entry(key).or_default() += count;
        }
        for (key, count) in other.attributes {
            *self.attributes.entry(key).or_default() += count;
        }
        for (key, count) in other.unprocessed {
            *self.unprocessed.entry(key).or_default() += count;
        }
        self.unreadable.extend(other.unreadable);
        self
    }
}

fn count_tokens(xml: &str, counts: &mut Counts) {
    let mut reader = Reader::from_str(xml);
    let mut stack: Vec<String> = Vec::new();
    let mut seen_in_file: BTreeMap<String, ()> = BTreeMap::new();
    while let Ok(event) = reader.read_event() {
        match event {
            Event::Start(ref start) | Event::Empty(ref start) => {
                let local = start.local_name().as_ref().to_owned();
                let parent = stack.last().cloned().unwrap_or_default();
                counts.elements.entry(local.clone()).or_default().0 += 1;
                seen_in_file.insert(local.clone(), ());
                *counts.children.entry((parent, local.clone())).or_default() += 1;
                for attribute in start.attributes().flatten() {
                    let key = attribute.key.as_ref().to_owned();
                    *counts.attributes.entry((local.clone(), key)).or_default() += 1;
                }
                if matches!(event, Event::Start(_)) {
                    stack.push(local);
                }
            }
            Event::End(_) => {
                stack.pop();
            }
            Event::Eof => break,
            _ => {}
        }
    }
    for (name, ()) in seen_in_file {
        counts.elements.entry(name).or_default().1 += 1;
    }
}

fn main() {
    let path = std::env::args()
        .nth(1)
        .expect("usage: inventory <zip or directory>");
    let source = Source::open(&path).expect("the source opens");
    let limits = Limits::default();
    let start = Instant::now();
    let counts = source
        .documents()
        .expect("the source lists its documents")
        .map(|loaded| {
            let mut counts = Counts {
                files: 1,
                bytes: loaded.entry.size,
                ..Counts::default()
            };
            let Ok(xml) = &loaded.xml else {
                counts.unreadable.push(loaded.entry.name.clone());
                return counts;
            };
            count_tokens(xml, &mut counts);
            match netex_rs::read_with(xml, &limits) {
                Ok(document) => document.for_each_unprocessed(&mut |parent, fragment| {
                    *counts
                        .unprocessed
                        .entry((parent.to_owned(), fragment.name.to_string()))
                        .or_default() += 1;
                }),
                Err(error) => counts
                    .unreadable
                    .push(format!("{}: {error}", loaded.entry.name)),
            }
            counts
        })
        .reduce(Counts::default, Counts::merge);
    let elapsed = start.elapsed();

    let total_elements: u64 = counts.elements.values().map(|(count, _)| count).sum();
    let total_unprocessed: u64 = counts.unprocessed.values().sum();
    println!("# The profile of {path}\n");
    println!(
        "{} files, {:.1} GB inflated, inventoried in {:.0} s on {} threads.\n",
        counts.files,
        counts.bytes as f64 / 1e9,
        elapsed.as_secs_f64(),
        rayon::current_num_threads()
    );
    println!(
        "{} distinct elements, {} occurrences; {} distinct (parent, child) pairs; {} distinct (element, attribute) pairs.\n",
        counts.elements.len(),
        total_elements,
        counts.children.len(),
        counts.attributes.len()
    );
    if !counts.unreadable.is_empty() {
        println!("## Files the typed model could not read\n");
        for name in &counts.unreadable {
            println!("- `{name}`");
        }
        println!();
    }
    println!("## Carried through untyped after the typed read\n");
    println!(
        "{} fragments in all, {:.3} % of the {} element occurrences (a fragment counts once, whatever it holds).\n",
        total_unprocessed,
        100.0 * total_unprocessed as f64 / total_elements.max(1) as f64,
        total_elements
    );
    println!("| Parent | Untyped child | Occurrences |\n|---|---|---|");
    let mut unprocessed: Vec<_> = counts.unprocessed.iter().collect();
    unprocessed.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
    for ((parent, child), count) in unprocessed {
        println!("| `{parent}` | `{child}` | {count} |");
    }
    println!("\n## Elements\n\n| Element | Occurrences | Files |\n|---|---|---|");
    let mut elements: Vec<_> = counts.elements.iter().collect();
    elements.sort_by(|a, b| b.1 .0.cmp(&a.1 .0).then(a.0.cmp(b.0)));
    for (name, (count, files)) in elements {
        println!("| `{name}` | {count} | {files} |");
    }
    println!("\n## Attributes\n\n| Element | Attribute | Occurrences |\n|---|---|---|");
    let mut attributes: Vec<_> = counts.attributes.iter().collect();
    attributes.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
    for ((element, attribute), count) in attributes {
        println!("| `{element}` | `{attribute}` | {count} |");
    }
    println!("\n## Parents and children\n\n| Parent | Child | Occurrences |\n|---|---|---|");
    let mut children: Vec<_> = counts.children.iter().collect();
    children.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
    for ((parent, child), count) in children {
        println!("| `{parent}` | `{child}` | {count} |");
    }
}
