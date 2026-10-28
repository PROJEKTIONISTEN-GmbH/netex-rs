//! A dataset: many documents read from a zip archive or a directory, on every
//! core.
//!
//! The German national delivery is one zip archive of 28 000 line files; a
//! [`Source`] opens it, lists its documents, and hands each one to the caller
//! inflated — in parallel, through rayon. The archive's central directory is
//! parsed once, when the source is opened; every worker gets a clone of the
//! parsed archive over a [`SharedFile`], which reads by position and so needs
//! no lock. (Opening the archive costs 0.15 s for the delivery's 29 549
//! entries, half a second when many threads do it at once; done once per
//! rayon job, as it was before, that was four fifths of the thread time on 32
//! threads.)
//!
//! Documents are handed out largest first: a worker that picks the 95 MB file
//! of the delivery last would finish long after the others, so the size order
//! is what keeps every worker busy to the end. Sizes come from the archive's
//! central directory, which is read once.
//!
//! A [`Manifest`] fingerprints every entry from the archive's central
//! directory, and its [`diff`](Manifest::diff) against the manifest of a
//! previous delivery says which files are new, changed or gone — without
//! inflating any of them — so that an importer reads only those. How much that
//! saves depends on the publisher: the German delivery is regenerated every
//! week, down to renumbered identifiers inside the files, and no file of one
//! week is the same as in the week before.

mod manifest;

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::os::unix::fs::FileExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

pub use manifest::{Changes, Fingerprint, Manifest};
use rayon::prelude::*;

use crate::error::{Error, Kind, Result};
use crate::{Document, Limits};

/// Where a dataset's documents come from.
#[derive(Debug, Clone)]
pub struct Source {
    kind: SourceKind,
}

#[derive(Debug, Clone)]
enum SourceKind {
    /// A zip archive; the documents are its `.xml` members. The central
    /// directory is parsed once; each worker clones the parsed archive.
    Zip {
        path: PathBuf,
        archive: zip::ZipArchive<SharedFile>,
    },
    /// A directory; the documents are the `.xml` files under it, at any depth.
    Directory(PathBuf),
}

/// A file read by position, so that clones — one per worker — share one
/// open file without a lock or a shared cursor.
#[derive(Debug, Clone)]
pub struct SharedFile {
    file: Arc<File>,
    position: u64,
}

impl SharedFile {
    fn open(path: &Path) -> Result<Self> {
        let file = File::open(path)
            .map_err(|e| Error::new(Kind::Io(format!("{}: {e}", path.display()))))?;
        Ok(SharedFile {
            file: Arc::new(file),
            position: 0,
        })
    }
}

impl Read for SharedFile {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let read = self.file.read_at(buf, self.position)?;
        self.position += read as u64;
        Ok(read)
    }
}

impl Seek for SharedFile {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        let position = match pos {
            SeekFrom::Start(offset) => Some(offset),
            SeekFrom::End(offset) => self.file.metadata()?.len().checked_add_signed(offset),
            SeekFrom::Current(offset) => self.position.checked_add_signed(offset),
        };
        self.position = position.ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "seek before the start of the file",
            )
        })?;
        Ok(self.position)
    }
}

/// One document of a dataset, not yet read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// The document's name: its path in the archive or under the directory.
    pub name: String,
    /// Its size in bytes, inflated.
    pub size: u64,
    /// The CRC-32 of its bytes: the archive's, or computed for a file.
    pub crc32: u32,
    /// Its position in the archive, or among the files.
    index: usize,
}

impl Entry {
    /// The entry's fingerprint.
    pub fn fingerprint(&self) -> Fingerprint {
        Fingerprint {
            name: self.name.clone(),
            size: self.size,
            crc32: self.crc32,
        }
    }
}

/// One document of a dataset, loaded.
#[derive(Debug)]
pub struct Loaded {
    /// The document's entry.
    pub entry: Entry,
    /// Its text, or why it could not be loaded.
    pub xml: Result<String>,
}

impl Loaded {
    /// Reads the document within `limits`.
    pub fn read(&self, limits: &Limits) -> Result<Document> {
        let xml = self.xml.as_ref().map_err(Clone::clone)?;
        crate::read_with(xml, limits)
    }
}

impl Source {
    /// A source at `path`: a zip archive if `path` is a file, else a directory.
    ///
    /// An archive's central directory is read here, once.
    pub fn open(path: impl AsRef<Path>) -> Result<Source> {
        let path = path.as_ref();
        let metadata = std::fs::metadata(path)
            .map_err(|e| Error::new(Kind::Io(format!("{}: {e}", path.display()))))?;
        let kind = if metadata.is_dir() {
            SourceKind::Directory(path.to_path_buf())
        } else {
            let archive = zip::ZipArchive::new(SharedFile::open(path)?)
                .map_err(|e| Error::new(Kind::Io(format!("{}: {e}", path.display()))))?;
            SourceKind::Zip {
                path: path.to_path_buf(),
                archive,
            }
        };
        Ok(Source { kind })
    }

    /// Where the source is.
    pub fn path(&self) -> &Path {
        match &self.kind {
            SourceKind::Zip { path, .. } | SourceKind::Directory(path) => path,
        }
    }

    /// The documents, in the archive's or the directory's order.
    ///
    /// Size and fingerprint come from the archive's central directory, which
    /// is read once; for a directory, each file is read once to fingerprint it.
    pub fn entries(&self) -> Result<Vec<Entry>> {
        match &self.kind {
            SourceKind::Zip { path, archive } => {
                let mut archive = archive.clone();
                let mut entries = Vec::new();
                for index in 0..archive.len() {
                    let file = archive
                        .by_index_raw(index)
                        .map_err(|e| Error::new(Kind::Io(format!("{}: {e}", path.display()))))?;
                    if !file.name().to_ascii_lowercase().ends_with(".xml") {
                        continue;
                    }
                    entries.push(Entry {
                        name: file.name().to_owned(),
                        size: file.size(),
                        crc32: file.crc32(),
                        index,
                    });
                }
                Ok(entries)
            }
            SourceKind::Directory(path) => {
                let mut files = Vec::new();
                collect_xml(path, &mut files)?;
                files.sort();
                files
                    .into_iter()
                    .enumerate()
                    .map(|(index, file)| {
                        let bytes = std::fs::read(&file).map_err(|e| {
                            Error::new(Kind::Io(format!("{}: {e}", file.display())))
                        })?;
                        Ok(Entry {
                            name: file
                                .strip_prefix(path)
                                .unwrap_or(&file)
                                .to_string_lossy()
                                .into_owned(),
                            size: bytes.len() as u64,
                            crc32: crc32fast::hash(&bytes),
                            index,
                        })
                    })
                    .collect()
            }
        }
    }

    /// The fingerprints of every document, from the central directory alone:
    /// nothing is inflated.
    pub fn manifest(&self) -> Result<Manifest> {
        Ok(Manifest {
            entries: self.entries()?.iter().map(Entry::fingerprint).collect(),
        })
    }

    /// Loads one document.
    pub fn load(&self, entry: &Entry) -> Loaded {
        Loaded {
            entry: entry.clone(),
            xml: self.reader().load(entry),
        }
    }

    /// The documents, largest first.
    fn entries_largest_first(&self) -> Result<Vec<Entry>> {
        let mut entries = self.entries()?;
        largest_first(&mut entries);
        Ok(entries)
    }

    /// Every document, loaded in parallel, largest first.
    ///
    /// Nothing is re-opened: each rayon job clones the archive parsed when the
    /// source was opened. Documents are loaded as they are consumed, so at most a few are in
    /// memory at a time. Each is handed out as its own [`String`]; a caller
    /// that reads document after document is better served by
    /// [`read_each`](Self::read_each), which reuses one buffer per worker.
    pub fn documents(&self) -> Result<impl ParallelIterator<Item = Loaded> + '_> {
        let entries = self.entries_largest_first()?;
        Ok(entries.into_par_iter().map_init(
            move || self.reader(),
            move |reader, entry| Loaded {
                xml: reader.load(&entry),
                entry,
            },
        ))
    }

    /// Calls `each` with every document, on every thread of the current rayon
    /// pool, largest first.
    ///
    /// The document's text borrows a buffer the worker keeps from one document
    /// to the next, so reading the whole delivery allocates one buffer per
    /// worker rather than one per document. A document that fails to load is
    /// handed over as its error.
    pub fn read_each<F>(&self, each: F) -> Result<()>
    where
        F: Fn(&Entry, Result<&str>) + Sync,
    {
        self.read_each_with(|| (), |(), entry, text| each(entry, text))
            .map(|_| ())
    }

    /// Like [`read_each`](Self::read_each), with a state per worker: `init`
    /// makes one on each thread of the pool, `each` gets it with every
    /// document that thread reads, and the states are returned once every
    /// document is read — a builder per worker, merged by the caller.
    ///
    /// Documents are pulled from one shared queue, largest first, so the work
    /// balances itself: a worker that finishes a small file takes the next
    /// largest one left.
    pub fn read_each_with<S, I, F>(&self, init: I, each: F) -> Result<Vec<S>>
    where
        S: Send,
        I: Fn() -> S + Sync,
        F: Fn(&mut S, &Entry, Result<&str>) + Sync,
    {
        self.read_entries_with(self.entries()?, init, each)
    }

    /// Like [`read_each_with`](Self::read_each_with), over `entries` only —
    /// those a [`Manifest::diff`] says are new or changed, say — in any order
    /// they are given: they are read largest first.
    pub fn read_entries_with<S, I, F>(
        &self,
        mut entries: Vec<Entry>,
        init: I,
        each: F,
    ) -> Result<Vec<S>>
    where
        S: Send,
        I: Fn() -> S + Sync,
        F: Fn(&mut S, &Entry, Result<&str>) + Sync,
    {
        largest_first(&mut entries);
        let next = AtomicUsize::new(0);
        let states = rayon::broadcast(|_| {
            let mut state = init();
            let mut reader = self.reader();
            let mut buffer = Vec::new();
            while let Some(entry) = entries.get(next.fetch_add(1, Ordering::Relaxed)) {
                each(&mut state, entry, reader.load_into(entry, &mut buffer));
            }
            state
        });
        Ok(states)
    }

    fn reader(&self) -> Reader {
        match &self.kind {
            SourceKind::Zip { archive, .. } => Reader::Archive(archive.clone()),
            SourceKind::Directory(path) => Reader::Directory(path.clone()),
        }
    }
}

/// Largest first; equal sizes in the source's order.
fn largest_first(entries: &mut [Entry]) {
    entries.sort_by(|a, b| b.size.cmp(&a.size).then_with(|| a.index.cmp(&b.index)));
}

fn collect_xml(dir: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
    let entries = std::fs::read_dir(dir)
        .map_err(|e| Error::new(Kind::Io(format!("{}: {e}", dir.display()))))?;
    for entry in entries {
        let path = entry
            .map_err(|e| Error::new(Kind::Io(format!("{}: {e}", dir.display()))))?
            .path();
        if path.is_dir() {
            collect_xml(&path, out)?;
        } else if path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("xml"))
        {
            out.push(path);
        }
    }
    Ok(())
}

/// One worker's way of loading documents.
enum Reader {
    Archive(zip::ZipArchive<SharedFile>),
    Directory(PathBuf),
}

impl Reader {
    /// Loads a document into a `String` of its own.
    fn load(&mut self, entry: &Entry) -> Result<String> {
        let mut bytes = Vec::with_capacity(usize::try_from(entry.size).unwrap_or(0));
        self.load_bytes(entry, &mut bytes)?;
        String::from_utf8(bytes).map_err(|e| not_utf8(&e.utf8_error()))
    }

    /// Loads a document into `buffer`, which it empties first, and hands out
    /// the text as a borrow of it: the buffer keeps its capacity for the next
    /// document, but for what it holds beyond twice the document's size.
    ///
    /// The documents come largest first, so a worker would otherwise hold the
    /// size of the largest document it read until the read ends: in the Swiss
    /// delivery the eight largest, 390–720 MB, held by eight workers for the
    /// whole read, while half of its documents are under 60 MB.
    fn load_into<'b>(&mut self, entry: &Entry, buffer: &'b mut Vec<u8>) -> Result<&'b str> {
        make_room(buffer, usize::try_from(entry.size).unwrap_or(0));
        self.load_bytes(entry, buffer)?;
        std::str::from_utf8(buffer).map_err(|e| not_utf8(&e))
    }

    fn load_bytes(&mut self, entry: &Entry, bytes: &mut Vec<u8>) -> Result<()> {
        match self {
            Reader::Archive(archive) => {
                let mut file = archive
                    .by_index(entry.index)
                    .map_err(|e| Error::new(Kind::Io(format!("{}: {e}", entry.name))))?;
                file.read_to_end(bytes)
                    .map_err(|e| Error::new(Kind::Io(format!("{}: {e}", entry.name))))?;
            }
            Reader::Directory(root) => {
                let path = root.join(&entry.name);
                let mut file = File::open(&path)
                    .map_err(|e| Error::new(Kind::Io(format!("{}: {e}", path.display()))))?;
                file.read_to_end(bytes)
                    .map_err(|e| Error::new(Kind::Io(format!("{}: {e}", path.display()))))?;
            }
        }
        Ok(())
    }
}

/// Empties `buffer` for a document of `size` bytes: room for it, and no more
/// than twice that kept.
fn make_room(buffer: &mut Vec<u8>, size: usize) {
    buffer.clear();
    if buffer.capacity() / 2 > size {
        buffer.shrink_to(size);
    }
    buffer.reserve(size);
}

fn not_utf8(error: &std::str::Utf8Error) -> Error {
    Error::new(Kind::Encoding {
        declared: format!("bytes: {error}"),
    })
}

#[cfg(test)]
mod tests {
    use super::make_room;

    #[test]
    fn a_buffer_gives_back_what_a_much_smaller_document_does_not_need() {
        let mut buffer = Vec::with_capacity(1_000);
        buffer.extend_from_slice(&[1; 700]);
        let room = buffer.capacity();
        make_room(&mut buffer, 600);
        assert!(buffer.is_empty());
        assert_eq!(
            buffer.capacity(),
            room,
            "more than half the room needed: kept"
        );
        make_room(&mut buffer, 100);
        let room = buffer.capacity();
        assert!((100..=200).contains(&room), "{room}");
        make_room(&mut buffer, 5_000);
        assert!(
            buffer.capacity() >= 5_000,
            "a larger document gets its room"
        );
    }
}
