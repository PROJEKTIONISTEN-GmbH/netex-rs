//! A dataset: many documents read from a zip archive or a directory, on every
//! core.
//!
//! The German national delivery is one zip archive of 28 000 line files; a
//! [`Source`] opens it, lists its documents, and hands each one to the caller
//! inflated — in parallel, through rayon. The archive's central directory is
//! parsed once, when the source is opened; every worker gets a clone of the
//! parsed archive over a file read by position, which needs no lock.
//! (Opening the archive costs 0.15 s for the delivery's 29 549 entries, half
//! a second when many threads do it at once; done once per rayon job, as it
//! was before, that was four fifths of the thread time on 32 threads.)
//!
//! Documents are handed out largest first: a worker that picks the 95 MB file
//! of the delivery last would finish long after the others, so the size order
//! is what keeps every worker busy to the end. Sizes come from the archive's
//! central directory, which is read once, or from a directory's listing: no
//! file is read before it is read.
//!
//! A reader that fails stops the others ([`Source::try_read_entries_with`]):
//! no document is loaded once one has failed, but those being read.
//!
//! A [`Manifest`] fingerprints every entry from the archive's central
//! directory — of a directory, by reading each file through a buffer —, and
//! its [`diff`](Manifest::diff) against the manifest of a previous delivery
//! says which files are new, changed or gone — without inflating any of them
//! — so that an importer reads only those. How much that saves depends on the
//! publisher: the German delivery is regenerated every week, down to
//! renumbered identifiers inside the files, and no file of one week is the
//! same as in the week before.
//!
//! A delivery may name what another publication defines: the Nordic
//! deliveries name the quays of a stop register published apart from them.
//! [`Source::with`] reads several sources as one.
//!
//! What an archive states of a member is the publisher's claim, read with
//! care: a member's name is listed as written, and its document is refused
//! where the name is no path of plain parts within the source
//! ([`Kind::DocumentName`]); a document larger than the source's limit is
//! refused, whatever size the archive claims for it
//! ([`Source::with_max_document_bytes`]). Either is an error of that document,
//! handed over in its place; the others are read. Of members an archive gives
//! one name, the first is read and the name reported
//! ([`Source::names_given_twice`]).

mod manifest;

use std::convert::Infallible;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;

pub use manifest::{Changes, Fingerprint, Manifest};
use rayon::prelude::*;
use rustc_hash::FxHashSet;

use crate::error::{Error, Kind, Limit, Result};
use crate::{Document, Limits};

/// The largest document a source reads unless told otherwise, in bytes:
/// 4 GiB.
///
/// Well above the largest documents of the national deliveries read so far —
/// Belgium's railway delivery states its network in one document of 1.08 GB,
/// Switzerland its largest timetable in one of 744 MB, Sweden its shared data
/// in one of 711 MB —, so that no delivery is refused for its size; a member
/// claiming more is refused before anything of it is read.
pub const DEFAULT_MAX_DOCUMENT_BYTES: u64 = 4 << 30;

/// Where a dataset's documents come from.
#[derive(Debug, Clone)]
pub struct Source {
    kind: SourceKind,
    /// The largest document read, in bytes.
    max_document_bytes: u64,
}

#[derive(Debug, Clone)]
enum SourceKind {
    /// A zip archive; the documents are its `.xml` members. The central
    /// directory is parsed once; each worker clones the parsed archive.
    Zip {
        path: PathBuf,
        archive: zip::ZipArchive<SharedFile>,
        /// The first member of each name the central directory gives more
        /// than one, where it gives any ([`first_members`]).
        firsts: Option<zip::ZipArchive<Overlay>>,
    },
    /// A directory; the documents are the `.xml` files under it, at any depth.
    Directory(PathBuf),
    /// Several sources read as one ([`Source::with`]), each a zip archive or
    /// a directory; the documents of every part after the first are named
    /// under the part's name.
    Combined(Vec<Part>),
}

/// One source of several read as one.
#[derive(Debug, Clone)]
struct Part {
    /// The name its documents are named under; none for the first part.
    name: Option<String>,
    source: Source,
}

impl Part {
    /// The name of its document `name` among the documents of every part.
    fn named(&self, name: &str) -> String {
        match &self.name {
            Some(part) => format!("{part}/{name}"),
            None => name.to_owned(),
        }
    }
}

/// A file read by position, so that clones — one per worker — share one
/// open file without a lock or a shared cursor.
#[derive(Debug, Clone)]
pub(crate) struct SharedFile {
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

    /// Reads into `buf` from `offset`, leaving the position as it was.
    #[cfg(unix)]
    fn read_at(&self, buf: &mut [u8], offset: u64) -> std::io::Result<usize> {
        std::os::unix::fs::FileExt::read_at(&*self.file, buf, offset)
    }

    /// Reads into `buf` from `offset`. Windows moves the file's own cursor
    /// as it reads, which no reader here relies on: each read names its
    /// offset.
    #[cfg(windows)]
    fn read_at(&self, buf: &mut [u8], offset: u64) -> std::io::Result<usize> {
        std::os::windows::fs::FileExt::seek_read(&*self.file, buf, offset)
    }

    /// The bytes from `offset` to the end of the file.
    fn tail(&self, offset: u64) -> std::io::Result<Vec<u8>> {
        let length = self.file.metadata()?.len().saturating_sub(offset);
        let mut bytes = vec![0; usize::try_from(length).map_err(std::io::Error::other)?];
        let mut filled = 0;
        while filled < bytes.len() {
            match self.read_at(&mut bytes[filled..], offset + filled as u64)? {
                0 => break,
                read => filled += read,
            }
        }
        bytes.truncate(filled);
        Ok(bytes)
    }
}

impl Read for SharedFile {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let read = self.read_at(buf, self.position)?;
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

/// An archive's file read with another central directory than its own: its
/// bytes up to where its own central directory starts, then `directory`.
#[derive(Debug, Clone)]
pub(crate) struct Overlay {
    file: SharedFile,
    /// Where the archive's own central directory starts.
    cut: u64,
    directory: Arc<[u8]>,
    position: u64,
}

impl Read for Overlay {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let read = if self.position < self.cut {
            let room = usize::try_from(self.cut - self.position).unwrap_or(usize::MAX);
            let wanted = buf.len().min(room);
            self.file.read_at(&mut buf[..wanted], self.position)?
        } else {
            let at = usize::try_from(self.position - self.cut).unwrap_or(usize::MAX);
            let rest = self.directory.get(at..).unwrap_or_default();
            let read = buf.len().min(rest.len());
            buf[..read].copy_from_slice(&rest[..read]);
            read
        };
        self.position += read as u64;
        Ok(read)
    }
}

impl Seek for Overlay {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        let end = self.cut + self.directory.len() as u64;
        let position = match pos {
            SeekFrom::Start(offset) => Some(offset),
            SeekFrom::End(offset) => end.checked_add_signed(offset),
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
    /// The document's name: its path in the archive or under the directory,
    /// `/`-separated, as the source states it.
    ///
    /// A name is the publisher's text, and is listed as it is written; its
    /// document loads only where the name is a path of plain parts within
    /// the source — none empty, `.` or `..`, not rooted, no backslash, no
    /// control character — and is refused otherwise
    /// ([`Kind::DocumentName`]). A caller writing documents under their names
    /// joins the name of a document that loaded, never that of an entry
    /// alone.
    pub name: String,
    /// Its size in bytes, inflated: as the archive's central directory or
    /// the directory's listing states it. Its fingerprint, with the CRC-32
    /// of its bytes, is the [`Source::manifest`]'s.
    pub size: u64,
    /// Its position in the archive, or among the files.
    index: usize,
    /// The part of a combined source it is from; 0 for any other source.
    part: usize,
    /// Why the document is refused rather than loaded, if it is.
    refused: Option<&'static str>,
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
    /// An archive's central directory is read here, once. A directory's
    /// documents are the `.xml` files under it at any depth, a directory
    /// reached by more than one path — through a link — read once.
    pub fn open(path: impl AsRef<Path>) -> Result<Source> {
        let path = path.as_ref();
        let failed =
            |e: &dyn std::fmt::Display| Error::new(Kind::Io(format!("{}: {e}", path.display())));
        let metadata = std::fs::metadata(path).map_err(|e| failed(&e))?;
        let kind = if metadata.is_dir() {
            SourceKind::Directory(path.to_path_buf())
        } else {
            let file = SharedFile::open(path)?;
            let archive = zip::ZipArchive::new(file.clone()).map_err(|e| failed(&e))?;
            let firsts = first_members(&file, &archive).map_err(|e| failed(&e))?;
            SourceKind::Zip {
                path: path.to_path_buf(),
                archive,
                firsts,
            }
        };
        Ok(Source {
            kind,
            max_document_bytes: DEFAULT_MAX_DOCUMENT_BYTES,
        })
    }

    /// This source, reading no document larger than `bytes`
    /// ([`DEFAULT_MAX_DOCUMENT_BYTES`] unless set): a document whose archive
    /// claims more, or that inflates to more, is refused as
    /// [`Limit::Document`], its bytes read no further than the limit. Set on
    /// several sources read as one, the limit holds for every part.
    pub fn with_max_document_bytes(mut self, bytes: u64) -> Source {
        self.max_document_bytes = bytes;
        self
    }

    /// This source with the documents of `other` read along with its own:
    /// a delivery with the stop register it names its quays in, published
    /// apart from it, as Norway's. The documents of `other` are named
    /// `<its file or directory name>/<document>`, so that no two documents
    /// of the parts share a name; a source whose name another part after
    /// the first already has is refused. The sources read as one read
    /// within this source's limit of a document's size.
    ///
    /// ```no_run
    /// # fn main() -> netex_rs::Result<()> {
    /// use netex_rs::dataset::Source;
    /// let source = Source::open("rb_rut-aggregated-netex.zip")?
    ///     .with(Source::open("Current_latest.zip")?)?;
    /// # Ok(()) }
    /// ```
    pub fn with(self, other: Source) -> Result<Source> {
        let max_document_bytes = self.max_document_bytes;
        let mut parts = match self.kind {
            SourceKind::Combined(parts) => parts,
            kind => vec![Part {
                name: None,
                source: Source {
                    kind,
                    max_document_bytes,
                },
            }],
        };
        let added = match other.kind {
            SourceKind::Combined(parts) => parts.into_iter().map(|part| part.source).collect(),
            kind => vec![Source {
                kind,
                max_document_bytes: other.max_document_bytes,
            }],
        };
        for source in added {
            let path = source.path();
            let name = path.file_name().map_or_else(
                || path.display().to_string(),
                |name| name.to_string_lossy().into_owned(),
            );
            if parts.iter().any(|part| part.name.as_deref() == Some(&name)) {
                return Err(Error::new(Kind::Io(format!(
                    "{}: another source read along is named {name} too",
                    path.display()
                ))));
            }
            parts.push(Part {
                name: Some(name),
                source,
            });
        }
        Ok(Source {
            kind: SourceKind::Combined(parts),
            max_document_bytes,
        })
    }

    /// Where the source is: for several read as one, where the first is.
    pub fn path(&self) -> &Path {
        match &self.kind {
            SourceKind::Zip { path, .. } | SourceKind::Directory(path) => path,
            SourceKind::Combined(parts) => parts[0].source.path(),
        }
    }

    /// The documents, in the archive's or the directory's order.
    ///
    /// Sizes come from the archive's central directory, which is read once,
    /// or from the directory's listing: no document is read. Every document
    /// is listed, under its name as the source states it, those refused when
    /// loaded too ([`Entry::name`]); a name an archive gives several members
    /// once, for the first ([`names_given_twice`](Self::names_given_twice)).
    pub fn entries(&self) -> Result<Vec<Entry>> {
        match &self.kind {
            SourceKind::Zip {
                path,
                archive,
                firsts,
            } => Ok(archive_members(path, archive, firsts.as_ref())?
                .into_iter()
                .map(|(entry, _)| entry)
                .collect()),
            SourceKind::Directory(path) => Ok(directory_files(path)?
                .into_iter()
                .map(|(_, entry)| entry)
                .collect()),
            SourceKind::Combined(parts) => {
                let mut entries = Vec::new();
                for (position, part) in parts.iter().enumerate() {
                    for mut entry in part.source.entries()? {
                        entry.name = part.named(&entry.name);
                        entry.part = position;
                        entries.push(entry);
                    }
                }
                Ok(entries)
            }
        }
    }

    /// The names an archive gives more than one of its members, as
    /// [`entries`](Self::entries) lists them: of each, the first member is
    /// read and the others left out — a finding the caller may count. Names
    /// alike only once read count too: `ä` written in UTF-8 and in code page
    /// 437. None for a directory, whose files each have a name of their own.
    pub fn names_given_twice(&self) -> Vec<String> {
        match &self.kind {
            SourceKind::Zip {
                archive, firsts, ..
            } => match firsts {
                // In the order the archive lists its documents.
                Some(firsts) => archive
                    .file_names()
                    .filter(|name| name.to_ascii_lowercase().ends_with(".xml"))
                    .filter(|name| firsts.index_for_name(name).is_some())
                    .map(str::to_owned)
                    .collect(),
                None => Vec::new(),
            },
            SourceKind::Directory(_) => Vec::new(),
            SourceKind::Combined(parts) => parts
                .iter()
                .flat_map(|part| {
                    part.source
                        .names_given_twice()
                        .into_iter()
                        .map(|name| part.named(&name))
                })
                .collect(),
        }
    }

    /// The fingerprints of every document: of an archive, from its central
    /// directory alone — nothing is inflated —; of a directory, each file read
    /// through a buffer of its own, on every thread of the current rayon pool.
    pub fn manifest(&self) -> Result<Manifest> {
        let entries = match &self.kind {
            SourceKind::Zip {
                path,
                archive,
                firsts,
            } => archive_members(path, archive, firsts.as_ref())?
                .into_iter()
                .map(|(entry, crc32)| Fingerprint {
                    name: entry.name,
                    size: entry.size,
                    crc32,
                })
                .collect(),
            SourceKind::Directory(path) => directory_files(path)?
                .into_par_iter()
                .map(|(file, entry)| {
                    let (size, crc32) = fingerprint(&file)
                        .map_err(|e| Error::new(Kind::Io(format!("{}: {e}", file.display()))))?;
                    Ok(Fingerprint {
                        name: entry.name,
                        size,
                        crc32,
                    })
                })
                .collect::<Result<_>>()?,
            SourceKind::Combined(parts) => {
                let mut entries = Vec::new();
                for part in parts {
                    for mut fingerprint in part.source.manifest()?.entries {
                        fingerprint.name = part.named(&fingerprint.name);
                        entries.push(fingerprint);
                    }
                }
                entries
            }
        };
        Ok(Manifest { entries })
    }

    /// Loads one document.
    ///
    /// The entry must be one this source listed: one of another source is
    /// refused where this source holds no document of its name at its place.
    pub fn load(&self, entry: &Entry) -> Loaded {
        Loaded {
            entry: entry.clone(),
            xml: self.reader().load(entry, self.max_document_bytes),
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
        let max = self.max_document_bytes;
        Ok(entries.into_par_iter().map_init(
            move || self.reader(),
            move |reader, entry| Loaded {
                xml: reader.load(&entry, max),
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
        entries: Vec<Entry>,
        init: I,
        each: F,
    ) -> Result<Vec<S>>
    where
        S: Send,
        I: Fn() -> S + Sync,
        F: Fn(&mut S, &Entry, Result<&str>) + Sync,
    {
        let read = self.try_read_entries_with(entries, init, |state, entry, text| {
            each(state, entry, text);
            Ok::<(), Infallible>(())
        });
        match read {
            Ok(states) => Ok(states),
            Err(never) => match never {},
        }
    }

    /// Like [`read_each_with`](Self::read_each_with), for a reader that may
    /// fail: once `each` returns an error, no worker loads another document,
    /// and that error is returned ([`try_read_entries_with`](Self::try_read_entries_with)).
    pub fn try_read_each_with<S, E, I, F>(&self, init: I, each: F) -> std::result::Result<Vec<S>, E>
    where
        S: Send,
        E: Send + From<Error>,
        I: Fn() -> S + Sync,
        F: Fn(&mut S, &Entry, Result<&str>) -> std::result::Result<(), E> + Sync,
    {
        self.try_read_entries_with(self.entries()?, init, each)
    }

    /// Like [`read_entries_with`](Self::read_entries_with), for a reader that
    /// may fail: once `each` returns an error, no worker loads another
    /// document — those being read are read to their end —, and the states
    /// are dropped.
    ///
    /// The error returned is that of the document first in the reading order
    /// (largest first) whose reading failed: every document before it was
    /// taken from the queue before it, and is read to its end. Where `each`
    /// fails on a document whatever worker reads it, that is the same
    /// document on any number of threads.
    pub fn try_read_entries_with<S, E, I, F>(
        &self,
        mut entries: Vec<Entry>,
        init: I,
        each: F,
    ) -> std::result::Result<Vec<S>, E>
    where
        S: Send,
        E: Send,
        I: Fn() -> S + Sync,
        F: Fn(&mut S, &Entry, Result<&str>) -> std::result::Result<(), E> + Sync,
    {
        largest_first(&mut entries);
        let next = AtomicUsize::new(0);
        let stopped = AtomicBool::new(false);
        let max = self.max_document_bytes;
        let workers = rayon::broadcast(|_| {
            let mut state = init();
            let mut reader = self.reader();
            let mut buffer = Vec::new();
            // A document once taken is read whatever happens meanwhile, so
            // that every document before a failed one is read.
            while !stopped.load(Ordering::Relaxed) {
                let at = next.fetch_add(1, Ordering::Relaxed);
                let Some(entry) = entries.get(at) else {
                    break;
                };
                if let Err(error) =
                    each(&mut state, entry, reader.load_into(entry, &mut buffer, max))
                {
                    stopped.store(true, Ordering::Relaxed);
                    return Err((at, error));
                }
            }
            Ok(state)
        });
        let mut states = Vec::with_capacity(workers.len());
        let mut failed: Option<(usize, E)> = None;
        for worker in workers {
            match worker {
                Ok(state) => states.push(state),
                Err((at, error)) => {
                    if failed.as_ref().is_none_or(|(first, _)| at < *first) {
                        failed = Some((at, error));
                    }
                }
            }
        }
        match failed {
            Some((_, error)) => Err(error),
            None => Ok(states),
        }
    }

    fn reader(&self) -> Reader {
        match &self.kind {
            SourceKind::Zip {
                archive, firsts, ..
            } => Reader::Archive {
                archive: archive.clone(),
                firsts: firsts.clone(),
            },
            SourceKind::Directory(path) => Reader::Directory(path.clone()),
            SourceKind::Combined(parts) => {
                let parts = parts.iter().map(|part| {
                    let prefix = part
                        .name
                        .as_ref()
                        .map_or_else(String::new, |name| format!("{name}/"));
                    (prefix, part.source.reader())
                });
                Reader::Combined(parts.collect())
            }
        }
    }
}

/// Largest first; equal sizes in the source's order.
fn largest_first(entries: &mut [Entry]) {
    entries.sort_by(|a, b| {
        b.size
            .cmp(&a.size)
            .then_with(|| (a.part, a.index).cmp(&(b.part, b.index)))
    });
}

/// Why `name` is no document name, if it is not: a document name is a path
/// of plain parts within its source, `/`-separated — none of them empty, `.`
/// or `..` —, holding no backslash, which some systems read as a separator,
/// and no control character.
fn refusal(name: &str) -> Option<&'static str> {
    if name.chars().any(char::is_control) {
        Some("it holds a control character")
    } else if name.contains('\\') {
        Some("it holds a backslash, which some systems read as a separator")
    } else if name.starts_with('/') {
        Some("it is rooted, not within its source")
    } else if name.split('/').any(|part| part == "..") {
        Some("it names a parent directory")
    } else if name.split('/').any(|part| part.is_empty() || part == ".") {
        Some("it names an empty part, or `.`")
    } else {
        None
    }
}

/// The `.xml` members of `archive`, at `path`, each with the CRC-32 the
/// central directory states: of a name given several members, the first
/// member's, from `firsts`.
fn archive_members(
    path: &Path,
    archive: &zip::ZipArchive<SharedFile>,
    firsts: Option<&zip::ZipArchive<Overlay>>,
) -> Result<Vec<(Entry, u32)>> {
    let failed =
        |e: zip::result::ZipError| Error::new(Kind::Io(format!("{}: {e}", path.display())));
    let mut archive = archive.clone();
    let mut firsts = firsts.cloned();
    let mut members = Vec::new();
    for index in 0..archive.len() {
        let file = archive.by_index_raw(index).map_err(failed)?;
        if !file.name().to_ascii_lowercase().ends_with(".xml") {
            continue;
        }
        let first = match firsts.as_mut() {
            Some(firsts) => match firsts.index_for_name(file.name()) {
                Some(at) => Some(firsts.by_index_raw(at).map_err(failed)?),
                None => None,
            },
            None => None,
        };
        let (size, crc32) = match &first {
            Some(first) => (first.size(), first.crc32()),
            None => (file.size(), file.crc32()),
        };
        let entry = Entry {
            name: file.name().to_owned(),
            size,
            index,
            part: 0,
            refused: refusal(file.name()),
        };
        members.push((entry, crc32));
    }
    Ok(members)
}

/// The first member of each name the central directory of `archive` gives
/// more than one, as an archive of their own; none where every member has a
/// name of its own.
///
/// The zip reader keeps the last member of a name, and reads names as the
/// archive flags them — UTF-8 or code page 437 —, so that names alike only
/// once read are one name too. The members it leaves out are found in the
/// central directory itself, and their headers written, the last first, into
/// a central directory of their own after the archive's members: read with
/// it, the zip reader keeps the first member of each name.
fn first_members(
    file: &SharedFile,
    archive: &zip::ZipArchive<SharedFile>,
) -> zip::result::ZipResult<Option<zip::ZipArchive<Overlay>>> {
    const CENTRAL_HEADER: [u8; 4] = 0x0201_4b50_u32.to_le_bytes();
    // The fixed part of a central directory header, before its name.
    const FIXED: usize = 46;
    let mut kept = FxHashSet::default();
    let mut listed = archive.clone();
    for index in 0..listed.len() {
        kept.insert(listed.by_index_raw(index)?.central_header_start());
    }
    let cut = archive.central_directory_start();
    let directory = file.tail(cut)?;
    let length = |at: usize| usize::from(u16::from_le_bytes([directory[at], directory[at + 1]]));
    let mut left_out = Vec::new();
    let mut at = 0;
    while directory.get(at..at + 4) == Some(&CENTRAL_HEADER) && at + FIXED <= directory.len() {
        let end = at + FIXED + length(at + 28) + length(at + 30) + length(at + 32);
        let Some(header) = directory.get(at..end) else {
            break;
        };
        if !kept.contains(&(cut + at as u64)) {
            left_out.push(header);
        }
        at = end;
    }
    if left_out.is_empty() {
        return Ok(None);
    }
    left_out.reverse();
    let headers = left_out.concat();
    let overlay = Overlay {
        file: file.clone(),
        cut,
        directory: end_of_central_directory(headers, left_out.len(), cut - archive.offset()).into(),
        position: 0,
    };
    let config = zip::read::Config {
        archive_offset: zip::read::ArchiveOffset::Known(archive.offset()),
    };
    zip::ZipArchive::with_config(config, overlay).map(Some)
}

/// `headers` — `count` central directory headers to be read from `start`,
/// relative to the archive — ended as a central directory is: in the zip64
/// form, which holds any count and offset.
fn end_of_central_directory(mut headers: Vec<u8>, count: usize, start: u64) -> Vec<u8> {
    let size = headers.len() as u64;
    let count = count as u64;
    // Where the zip64 end record begins, relative to the archive.
    let end = start + size;
    // The zip64 end of central directory record: its size after the first
    // twelve bytes, versions, disks, counts, size and start of the directory.
    headers.extend_from_slice(&0x0606_4b50_u32.to_le_bytes());
    headers.extend_from_slice(&44_u64.to_le_bytes());
    headers.extend_from_slice(&45_u16.to_le_bytes());
    headers.extend_from_slice(&45_u16.to_le_bytes());
    headers.extend_from_slice(&[0; 8]);
    headers.extend_from_slice(&count.to_le_bytes());
    headers.extend_from_slice(&count.to_le_bytes());
    headers.extend_from_slice(&size.to_le_bytes());
    headers.extend_from_slice(&start.to_le_bytes());
    // Its locator: the disk, where the record begins, the disks.
    headers.extend_from_slice(&0x0706_4b50_u32.to_le_bytes());
    headers.extend_from_slice(&0_u32.to_le_bytes());
    headers.extend_from_slice(&end.to_le_bytes());
    headers.extend_from_slice(&1_u32.to_le_bytes());
    // The end of central directory record, its counts and offset deferring
    // to the zip64 record's.
    headers.extend_from_slice(&0x0605_4b50_u32.to_le_bytes());
    headers.extend_from_slice(&[0; 4]);
    headers.extend_from_slice(&u16::MAX.to_le_bytes());
    headers.extend_from_slice(&u16::MAX.to_le_bytes());
    headers.extend_from_slice(&u32::MAX.to_le_bytes());
    headers.extend_from_slice(&u32::MAX.to_le_bytes());
    headers.extend_from_slice(&0_u16.to_le_bytes());
    headers
}

/// The `.xml` files under the directory at `root`, in the order of their
/// paths, each with its entry: its name, and its size as the directory
/// lists it. No file is read.
fn directory_files(root: &Path) -> Result<Vec<(PathBuf, Entry)>> {
    let mut files = Vec::new();
    collect_xml(root, &mut FxHashSet::default(), &mut files)?;
    files.sort();
    files
        .into_iter()
        .enumerate()
        .map(|(index, file)| {
            let size = std::fs::metadata(&file)
                .map_err(|e| Error::new(Kind::Io(format!("{}: {e}", file.display()))))?
                .len();
            let relative = file.strip_prefix(root).unwrap_or(&file);
            let (name, refused) = match relative.to_str() {
                Some(name) => {
                    let name = name.replace(std::path::MAIN_SEPARATOR, "/");
                    let refused = refusal(&name);
                    (name, refused)
                }
                None => (
                    relative.to_string_lossy().into_owned(),
                    Some("its file name is not UTF-8"),
                ),
            };
            let entry = Entry {
                name,
                size,
                index,
                part: 0,
                refused,
            };
            Ok((file, entry))
        })
        .collect()
}

/// The size and CRC-32 of a file, read through a buffer of its own.
fn fingerprint(path: &Path) -> std::io::Result<(u64, u32)> {
    let mut file = File::open(path)?;
    let mut hasher = crc32fast::Hasher::new();
    let mut buffer = vec![0; 1 << 16];
    let mut size = 0;
    loop {
        match file.read(&mut buffer)? {
            0 => return Ok((size, hasher.finalize())),
            read => {
                hasher.update(&buffer[..read]);
                size += read as u64;
            }
        }
    }
}

/// The `.xml` files under `dir`, at any depth, each directory read once
/// however many paths — links — reach it: the first, in the order of the
/// names.
fn collect_xml(dir: &Path, read: &mut FxHashSet<PathBuf>, out: &mut Vec<PathBuf>) -> Result<()> {
    let failed = |e: std::io::Error| Error::new(Kind::Io(format!("{}: {e}", dir.display())));
    if !read.insert(dir.canonicalize().map_err(failed)?) {
        return Ok(());
    }
    let mut paths = std::fs::read_dir(dir)
        .map_err(failed)?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<std::io::Result<Vec<_>>>()
        .map_err(failed)?;
    paths.sort();
    for path in paths {
        if path.is_dir() {
            collect_xml(&path, read, out)?;
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
    /// An archive, with the first member of each name it gives several.
    Archive {
        archive: zip::ZipArchive<SharedFile>,
        firsts: Option<zip::ZipArchive<Overlay>>,
    },
    Directory(PathBuf),
    /// One reader for each part of a combined source, with the name its
    /// documents are named under, `/` and all; none for the first part.
    Combined(Vec<(String, Reader)>),
}

impl Reader {
    /// Loads a document of at most `max` bytes into a `String` of its own.
    fn load(&mut self, entry: &Entry, max: u64) -> Result<String> {
        let mut bytes = Vec::new();
        held(entry, max)?;
        make_room(&mut bytes, room(entry)).map_err(|e| cannot_hold(entry, &e))?;
        self.load_bytes(entry, &mut bytes, max)?;
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
    fn load_into<'b>(
        &mut self,
        entry: &Entry,
        buffer: &'b mut Vec<u8>,
        max: u64,
    ) -> Result<&'b str> {
        held(entry, max)?;
        make_room(buffer, room(entry)).map_err(|e| cannot_hold(entry, &e))?;
        self.load_bytes(entry, buffer, max)?;
        std::str::from_utf8(buffer).map_err(|e| not_utf8(&e))
    }

    /// Appends the bytes of the document of `entry` to `bytes`, reading no
    /// further than one byte past `max`: one more is a document larger than
    /// the limit.
    fn load_bytes(&mut self, entry: &Entry, bytes: &mut Vec<u8>, max: u64) -> Result<()> {
        let read = self.read_named(entry, &entry.name, bytes, max)?;
        if read as u64 > max {
            return Err(too_large(entry, max));
        }
        Ok(())
    }

    /// Reads the document named `name` within this reader's source, where
    /// `entry` says it is, and how many bytes; an entry that does not name
    /// the document at its place is one of another source.
    fn read_named(
        &mut self,
        entry: &Entry,
        name: &str,
        bytes: &mut Vec<u8>,
        max: u64,
    ) -> Result<usize> {
        let another = || {
            Error::new(Kind::Io(format!(
                "{}: not a document of this source",
                entry.name
            )))
        };
        match self {
            Reader::Archive { archive, firsts } => {
                let failed = |e: &dyn std::fmt::Display| {
                    Error::new(Kind::Io(format!("{}: {e}", entry.name)))
                };
                let named = archive.name_for_index(entry.index);
                if named != Some(name) {
                    return Err(another());
                }
                let first = firsts
                    .as_mut()
                    .and_then(|firsts| Some((firsts.index_for_name(name)?, firsts)));
                let read = match first {
                    Some((at, firsts)) => firsts
                        .by_index(at)
                        .map_err(|e| failed(&e))?
                        .take(max.saturating_add(1))
                        .read_to_end(bytes),
                    None => archive
                        .by_index(entry.index)
                        .map_err(|e| failed(&e))?
                        .take(max.saturating_add(1))
                        .read_to_end(bytes),
                };
                read.map_err(|e| failed(&e))
            }
            Reader::Directory(root) => {
                let path = root.join(name);
                let failed =
                    |e: std::io::Error| Error::new(Kind::Io(format!("{}: {e}", path.display())));
                File::open(&path)
                    .map_err(failed)?
                    .take(max.saturating_add(1))
                    .read_to_end(bytes)
                    .map_err(failed)
            }
            Reader::Combined(parts) => {
                let (prefix, reader) = parts.get_mut(entry.part).ok_or_else(another)?;
                let name = name.strip_prefix(prefix.as_str()).ok_or_else(another)?;
                reader.read_named(entry, name, bytes, max)
            }
        }
    }
}

/// Whether the document of `entry` may be read within `max` bytes: refused
/// for its name, or for the size its source states, before anything of it
/// is read.
fn held(entry: &Entry, max: u64) -> Result<()> {
    if let Some(reason) = entry.refused {
        return Err(Error::new(Kind::DocumentName {
            name: entry.name.clone(),
            reason,
        }));
    }
    if entry.size > max {
        return Err(too_large(entry, max));
    }
    Ok(())
}

/// The room to make for the document of `entry` before reading it: the size
/// its source states, which is within the limit once [`held`] — and only a
/// claim, so that the reservation is a hint the read may outgrow.
fn room(entry: &Entry) -> usize {
    usize::try_from(entry.size).unwrap_or(usize::MAX)
}

fn too_large(entry: &Entry, max: u64) -> Error {
    Error::new(Kind::Limit(Limit::Document(max))).in_document(&entry.name)
}

fn cannot_hold(entry: &Entry, error: &std::collections::TryReserveError) -> Error {
    Error::new(Kind::Io(format!(
        "{}: no room for the {} bytes its source states: {error}",
        entry.name, entry.size
    )))
}

/// Empties `buffer` for a document of `size` bytes: room for it, and no more
/// than twice that kept. Room that cannot be had is an error, not an abort.
fn make_room(
    buffer: &mut Vec<u8>,
    size: usize,
) -> std::result::Result<(), std::collections::TryReserveError> {
    buffer.clear();
    if buffer.capacity() / 2 > size {
        buffer.shrink_to(size);
    }
    buffer.try_reserve(size)
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
        make_room(&mut buffer, 600).expect("room");
        assert!(buffer.is_empty());
        assert_eq!(
            buffer.capacity(),
            room,
            "more than half the room needed: kept"
        );
        make_room(&mut buffer, 100).expect("room");
        let room = buffer.capacity();
        assert!((100..=200).contains(&room), "{room}");
        make_room(&mut buffer, 5_000).expect("room");
        assert!(
            buffer.capacity() >= 5_000,
            "a larger document gets its room"
        );
        assert!(
            make_room(&mut buffer, usize::MAX).is_err(),
            "room no machine has is an error"
        );
    }
}
