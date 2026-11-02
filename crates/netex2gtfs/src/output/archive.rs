//! A feed's zip archive, written by the crate itself so that every thread
//! deflates: each file in pieces, each piece's compressor primed with the
//! bytes before it, the pieces one deflate stream — and the archive the same
//! bytes on any number of threads.
//!
//! The archive takes the ZIP64 extensions (APPNOTE 6.3) where it must: every
//! local header carries its sizes in a ZIP64 field, as a file's sizes are
//! known only once it is written; the central directory and its end carry
//! them only where a size, an offset or the number of files does not fit.

use std::fs::File;
use std::io::{self, BufWriter, Seek, SeekFrom, Write};
use std::path::Path;

use crc32fast::Hasher;
use flate2::{Compress, Compression, FlushCompress, Status};
use rayon::prelude::*;

use super::Output;

/// How many bytes of a file are deflated as one piece.
const PIECE: usize = 1 << 20;
/// How many bytes before a piece its compressor is primed with: the deflate
/// window, so that a piece compresses as it would in one stream.
const WINDOW: usize = 32 << 10;
/// How many pieces gather, per thread of the pool, before they are deflated.
const PIECES_PER_THREAD: usize = 4;

/// The version of the format the entries need: 4.5, the ZIP64 extensions.
const VERSION: u16 = 45;
/// The compression method: deflate.
const DEFLATED: u16 = 8;
/// The date every entry carries, 1 January 1980 — the first a zip archive
/// can state —, so that the archive's bytes do not depend on the day.
const DATE: u16 = (1 << 5) | 1;
/// What a field holds whose value is in the ZIP64 field instead.
const IN_ZIP64: u32 = u32::MAX;
/// The tag of the ZIP64 extra field.
const ZIP64_FIELD: u16 = 1;

/// The files of a feed in a zip archive, deflated on every thread of the
/// current rayon pool.
#[derive(Debug)]
pub struct Archive {
    file: BufWriter<File>,
    /// Where the next byte goes.
    position: u64,
    /// The files written, for the central directory.
    entries: Vec<Entry>,
    /// The file being written.
    current: Option<Current>,
}

/// A file of the archive.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Entry {
    name: String,
    /// Where its local header is.
    offset: u64,
    crc: u32,
    compressed: u64,
    size: u64,
}

/// The file being written: its entry so far, and the bytes not yet deflated.
#[derive(Debug)]
struct Current {
    entry: Entry,
    /// Whole pieces, in order.
    pieces: Vec<Vec<u8>>,
    /// The piece being filled.
    filling: Vec<u8>,
    /// The last bytes before the first of `pieces`, at most a window.
    before: Vec<u8>,
    /// How many pieces gather before they are deflated.
    gather: usize,
}

impl Archive {
    /// A feed written as the archive `path`.
    pub fn create(path: impl AsRef<Path>) -> io::Result<Archive> {
        Ok(Archive {
            file: BufWriter::with_capacity(1 << 20, File::create(path)?),
            position: 0,
            entries: Vec::new(),
            current: None,
        })
    }

    fn put(&mut self, bytes: &[u8]) -> io::Result<()> {
        self.file.write_all(bytes)?;
        self.position += bytes.len() as u64;
        Ok(())
    }

    /// Deflates the pieces gathered, on every thread, and writes them; with
    /// `last`, the piece being filled after them, ending the file's stream.
    fn deflate(&mut self, last: bool) -> io::Result<()> {
        let current = self.current.as_mut().expect("a file is being written");
        let mut pieces = std::mem::take(&mut current.pieces);
        if last {
            pieces.push(std::mem::take(&mut current.filling));
        }
        let Some(final_piece) = pieces.last() else {
            return Ok(());
        };
        let before = std::mem::replace(&mut current.before, tail(final_piece).to_vec());
        let count = pieces.len();
        let deflated: Vec<(Vec<u8>, u32)> = (0..count)
            .into_par_iter()
            .map(|at| {
                let primer = if at == 0 {
                    &before[..]
                } else {
                    tail(&pieces[at - 1])
                };
                let piece = &pieces[at];
                let crc = crc32fast::hash(piece);
                deflate_piece(piece, primer, last && at + 1 == count).map(|bytes| (bytes, crc))
            })
            .collect::<io::Result<_>>()?;
        for (piece, (bytes, crc)) in pieces.iter().zip(deflated) {
            self.put(&bytes)?;
            let entry = &mut self.current.as_mut().expect("a file").entry;
            let mut hasher = Hasher::new_with_initial_len(entry.crc, entry.size);
            hasher.combine(&Hasher::new_with_initial_len(crc, piece.len() as u64));
            entry.crc = hasher.finalize();
            entry.size += piece.len() as u64;
            entry.compressed += bytes.len() as u64;
        }
        Ok(())
    }

    /// Ends the file being written: the rest of it deflated, its checksum and
    /// sizes written into its local header.
    fn end_file(&mut self) -> io::Result<()> {
        if self.current.is_none() {
            return Ok(());
        }
        self.deflate(true)?;
        let entry = self.current.take().expect("a file").entry;
        self.file.seek(SeekFrom::Start(entry.offset + 14))?;
        self.file.write_all(&entry.crc.to_le_bytes())?;
        let sizes = entry.offset + LOCAL_HEADER + entry.name.len() as u64 + 4;
        self.file.seek(SeekFrom::Start(sizes))?;
        self.file.write_all(&entry.size.to_le_bytes())?;
        self.file.write_all(&entry.compressed.to_le_bytes())?;
        self.file.seek(SeekFrom::Start(self.position))?;
        self.entries.push(entry);
        Ok(())
    }
}

impl Output for Archive {
    fn start(&mut self, name: &str) -> io::Result<()> {
        self.end_file()?;
        let entry = Entry {
            name: name.to_owned(),
            offset: self.position,
            crc: 0,
            compressed: 0,
            size: 0,
        };
        let header = local_header(&entry)?;
        self.put(&header)?;
        self.current = Some(Current {
            entry,
            pieces: Vec::new(),
            filling: Vec::with_capacity(PIECE),
            before: Vec::new(),
            gather: rayon::current_num_threads() * PIECES_PER_THREAD,
        });
        Ok(())
    }

    fn write_all(&mut self, mut bytes: &[u8]) -> io::Result<()> {
        while !bytes.is_empty() {
            let current = self
                .current
                .as_mut()
                .expect("a file is started before it is written");
            let room = PIECE - current.filling.len();
            let (now, rest) = bytes.split_at(room.min(bytes.len()));
            current.filling.extend_from_slice(now);
            bytes = rest;
            if current.filling.len() == PIECE {
                let piece = std::mem::replace(&mut current.filling, Vec::with_capacity(PIECE));
                current.pieces.push(piece);
                if current.pieces.len() >= current.gather {
                    self.deflate(false)?;
                }
            }
        }
        Ok(())
    }

    fn finish(mut self: Box<Self>) -> io::Result<()> {
        self.end_file()?;
        let start = self.position;
        for at in 0..self.entries.len() {
            let record = central_header(&self.entries[at])?;
            self.put(&record)?;
        }
        let size = self.position - start;
        let end = central_end(self.entries.len() as u64, size, start, self.position);
        self.put(&end)?;
        self.file.flush()
    }
}

/// The last window of `bytes`, all of them where they are fewer.
fn tail(bytes: &[u8]) -> &[u8] {
    &bytes[bytes.len().saturating_sub(WINDOW)..]
}

/// A piece of a file as raw deflate, its compressor primed with `primer`:
/// ended by a sync flush, so that the next piece's bytes follow it in the
/// same stream; or, the `last`, ending the stream.
fn deflate_piece(piece: &[u8], primer: &[u8], last: bool) -> io::Result<Vec<u8>> {
    let mut compress = Compress::new(Compression::default(), false);
    if !primer.is_empty() {
        compress.set_dictionary(primer).map_err(io::Error::other)?;
    }
    let flush = if last {
        FlushCompress::Finish
    } else {
        FlushCompress::Sync
    };
    let mut bytes = Vec::with_capacity(piece.len() / 4 + 64);
    loop {
        let read = usize::try_from(compress.total_in()).expect("a piece fits in memory");
        let status = compress
            .compress_vec(&piece[read..], &mut bytes, flush)
            .map_err(io::Error::other)?;
        let all_read = compress.total_in() == piece.len() as u64;
        // A flush is complete once it leaves room in the output.
        let done = match status {
            Status::StreamEnd => true,
            Status::Ok | Status::BufError => !last && all_read && bytes.len() < bytes.capacity(),
        };
        if done {
            return Ok(bytes);
        }
        bytes.reserve(bytes.capacity().max(4096));
    }
}

/// The length of a local header before its name.
const LOCAL_HEADER: u64 = 30;

/// The local header of a file about to be written: its checksum and sizes
/// zero, written once the file is ([`Archive::end_file`]), the sizes in the
/// ZIP64 field.
fn local_header(entry: &Entry) -> io::Result<Vec<u8>> {
    let mut header = Vec::with_capacity(LOCAL_HEADER as usize + entry.name.len() + 20);
    header.extend_from_slice(&0x0403_4b50_u32.to_le_bytes());
    header.extend_from_slice(&VERSION.to_le_bytes());
    header.extend_from_slice(&0_u16.to_le_bytes()); // flags
    header.extend_from_slice(&DEFLATED.to_le_bytes());
    header.extend_from_slice(&0_u16.to_le_bytes()); // time
    header.extend_from_slice(&DATE.to_le_bytes());
    header.extend_from_slice(&0_u32.to_le_bytes()); // checksum
    header.extend_from_slice(&IN_ZIP64.to_le_bytes()); // compressed size
    header.extend_from_slice(&IN_ZIP64.to_le_bytes()); // size
    header.extend_from_slice(&name_length(&entry.name)?.to_le_bytes());
    header.extend_from_slice(&20_u16.to_le_bytes()); // extra field length
    header.extend_from_slice(entry.name.as_bytes());
    header.extend_from_slice(&ZIP64_FIELD.to_le_bytes());
    header.extend_from_slice(&16_u16.to_le_bytes());
    header.extend_from_slice(&0_u64.to_le_bytes()); // size
    header.extend_from_slice(&0_u64.to_le_bytes()); // compressed size
    Ok(header)
}

/// The central directory's record of a file: a size or offset past 32 bits
/// in its ZIP64 field.
fn central_header(entry: &Entry) -> io::Result<Vec<u8>> {
    let mut zip64 = Vec::new();
    let mut field = |value: u64| match u32::try_from(value) {
        Ok(small) if small != IN_ZIP64 => small,
        _ => {
            zip64.extend_from_slice(&value.to_le_bytes());
            IN_ZIP64
        }
    };
    // The ZIP64 field holds the size, the compressed size and the offset,
    // those of them past 32 bits, in this order.
    let size = field(entry.size);
    let compressed = field(entry.compressed);
    let offset = field(entry.offset);
    let extra = if zip64.is_empty() {
        Vec::new()
    } else {
        let mut extra = Vec::with_capacity(4 + zip64.len());
        extra.extend_from_slice(&ZIP64_FIELD.to_le_bytes());
        extra.extend_from_slice(&(zip64.len() as u16).to_le_bytes());
        extra.extend_from_slice(&zip64);
        extra
    };
    let mut record = Vec::with_capacity(46 + entry.name.len() + extra.len());
    record.extend_from_slice(&0x0201_4b50_u32.to_le_bytes());
    record.extend_from_slice(&VERSION.to_le_bytes()); // made by
    record.extend_from_slice(&VERSION.to_le_bytes()); // needed
    record.extend_from_slice(&0_u16.to_le_bytes()); // flags
    record.extend_from_slice(&DEFLATED.to_le_bytes());
    record.extend_from_slice(&0_u16.to_le_bytes()); // time
    record.extend_from_slice(&DATE.to_le_bytes());
    record.extend_from_slice(&entry.crc.to_le_bytes());
    record.extend_from_slice(&compressed.to_le_bytes());
    record.extend_from_slice(&size.to_le_bytes());
    record.extend_from_slice(&name_length(&entry.name)?.to_le_bytes());
    record.extend_from_slice(&(extra.len() as u16).to_le_bytes());
    record.extend_from_slice(&0_u16.to_le_bytes()); // comment length
    record.extend_from_slice(&0_u16.to_le_bytes()); // disk
    record.extend_from_slice(&0_u16.to_le_bytes()); // internal attributes
    record.extend_from_slice(&0_u32.to_le_bytes()); // external attributes
    record.extend_from_slice(&offset.to_le_bytes());
    record.extend_from_slice(entry.name.as_bytes());
    record.extend_from_slice(&extra);
    Ok(record)
}

/// The end of the central directory of `files` files, `size` bytes from
/// `start`, written at `position`: preceded by its ZIP64 record and locator
/// where a number does not fit its field.
fn central_end(files: u64, size: u64, start: u64, position: u64) -> Vec<u8> {
    let files_16 = u16::try_from(files).ok().filter(|&n| n != u16::MAX);
    let size_32 = u32::try_from(size).ok().filter(|&n| n != IN_ZIP64);
    let start_32 = u32::try_from(start).ok().filter(|&n| n != IN_ZIP64);
    let mut end = Vec::with_capacity(98);
    if files_16.is_none() || size_32.is_none() || start_32.is_none() {
        end.extend_from_slice(&0x0606_4b50_u32.to_le_bytes());
        end.extend_from_slice(&44_u64.to_le_bytes()); // the size of the rest of the record
        end.extend_from_slice(&VERSION.to_le_bytes()); // made by
        end.extend_from_slice(&VERSION.to_le_bytes()); // needed
        end.extend_from_slice(&0_u32.to_le_bytes()); // this disk
        end.extend_from_slice(&0_u32.to_le_bytes()); // the disk of the directory
        end.extend_from_slice(&files.to_le_bytes()); // on this disk
        end.extend_from_slice(&files.to_le_bytes()); // in all
        end.extend_from_slice(&size.to_le_bytes());
        end.extend_from_slice(&start.to_le_bytes());
        end.extend_from_slice(&0x0706_4b50_u32.to_le_bytes());
        end.extend_from_slice(&0_u32.to_le_bytes()); // the disk of the record
        end.extend_from_slice(&position.to_le_bytes()); // where the record is
        end.extend_from_slice(&1_u32.to_le_bytes()); // disks
    }
    let files_16 = files_16.unwrap_or(u16::MAX);
    end.extend_from_slice(&0x0605_4b50_u32.to_le_bytes());
    end.extend_from_slice(&0_u16.to_le_bytes()); // this disk
    end.extend_from_slice(&0_u16.to_le_bytes()); // the disk of the directory
    end.extend_from_slice(&files_16.to_le_bytes()); // on this disk
    end.extend_from_slice(&files_16.to_le_bytes()); // in all
    end.extend_from_slice(&size_32.unwrap_or(IN_ZIP64).to_le_bytes());
    end.extend_from_slice(&start_32.unwrap_or(IN_ZIP64).to_le_bytes());
    end.extend_from_slice(&0_u16.to_le_bytes()); // comment length
    end
}

fn name_length(name: &str) -> io::Result<u16> {
    u16::try_from(name.len())
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "a file name of 64 KiB or more"))
}

#[cfg(test)]
mod tests {
    use std::io::Read;

    use super::*;

    /// Text as a feed's files are: rows of identifiers, times and numbers,
    /// `rows` of them.
    fn rows(rows: usize) -> Vec<u8> {
        let mut text = String::new();
        for row in 0..rows {
            text.push_str(&format!(
                "ch:1:ServiceJourney:{}:{},{:02}:{:02}:00,ch:1:sloid:{}:0:{},{}\n",
                row / 40,
                row % 7,
                row / 60 % 24,
                row % 60,
                8500 + row % 911,
                row % 3,
                row % 40
            ));
        }
        text.into_bytes()
    }

    /// The files of the archive written at `path` on `threads` threads: each
    /// name with its bytes, written as given.
    fn write(path: &Path, threads: usize, files: &[(&str, &[u8])]) {
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build()
            .expect("a pool");
        pool.install(|| {
            let mut archive: Box<dyn Output> = Box::new(Archive::create(path).expect("created"));
            for (name, bytes) in files {
                archive.start(name).expect("started");
                // Written as a table writes, a row at a time.
                for row in bytes.split_inclusive(|&b| b == b'\n') {
                    archive.write_all(row).expect("written");
                }
            }
            archive.finish().expect("finished");
        });
    }

    #[test]
    fn every_file_reads_back_as_written_its_checksum_checked() {
        let large = rows(120_000);
        assert!(large.len() > 5 * PIECE, "several pieces: {}", large.len());
        let whole_pieces = &large[..2 * PIECE];
        let files: [(&str, &[u8]); 4] = [
            ("agency.txt", b"agency_id,agency_name\na,Seeweg\n"),
            ("stop_times.txt", &large),
            ("empty.txt", b""),
            ("pieces.txt", whole_pieces),
        ];
        let directory = tempfile::tempdir().expect("a directory");
        let path = directory.path().join("feed.zip");
        write(&path, 4, &files);
        let mut archive =
            zip::ZipArchive::new(File::open(&path).expect("the archive")).expect("a zip archive");
        assert_eq!(archive.len(), files.len());
        for (at, (name, bytes)) in files.iter().enumerate() {
            let mut file = archive.by_index(at).expect("a file");
            assert_eq!(file.name(), *name);
            assert_eq!(file.compression(), zip::CompressionMethod::Deflated);
            let mut read = Vec::new();
            // Reading to the end checks the checksum.
            file.read_to_end(&mut read).expect("the file reads");
            assert!(read == *bytes, "{name} reads back otherwise");
        }
    }

    #[test]
    fn the_archive_is_the_same_bytes_on_any_number_of_threads() {
        let large = rows(60_000);
        let files: [(&str, &[u8]); 2] = [("stops.txt", b"stop_id\n"), ("stop_times.txt", &large)];
        let directory = tempfile::tempdir().expect("a directory");
        let written: Vec<Vec<u8>> = [1, 3, 8]
            .into_iter()
            .map(|threads| {
                let path = directory.path().join(format!("{threads}.zip"));
                write(&path, threads, &files);
                std::fs::read(&path).expect("the archive")
            })
            .collect();
        assert!(written[1] == written[0] && written[2] == written[0]);
    }

    #[test]
    fn the_pieces_deflate_about_as_well_as_one_stream() {
        let large = rows(120_000);
        let mut one = flate2::write::DeflateEncoder::new(Vec::new(), Compression::default());
        one.write_all(&large).expect("deflated");
        let one = one.finish().expect("deflated").len();
        let directory = tempfile::tempdir().expect("a directory");
        let path = directory.path().join("feed.zip");
        write(&path, 2, &[("stop_times.txt", &large)]);
        let mut archive =
            zip::ZipArchive::new(File::open(&path).expect("the archive")).expect("a zip archive");
        let pieces = archive
            .by_name("stop_times.txt")
            .expect("the file is there")
            .compressed_size();
        assert!(
            (pieces as f64) < one as f64 * 1.01,
            "{pieces} bytes in pieces, {one} in one stream"
        );
    }

    #[test]
    fn a_size_or_offset_past_32_bits_goes_into_the_zip64_field() {
        let entry = Entry {
            name: "stop_times.txt".to_owned(),
            offset: 5 << 30,
            crc: 0x1234_5678,
            compressed: 700 << 20,
            size: 6 << 30,
        };
        let record = central_header(&entry).expect("a record");
        let u32_at =
            |at: usize| u32::from_le_bytes(record[at..at + 4].try_into().expect("4 bytes"));
        assert_eq!(u32_at(16), 0x1234_5678);
        assert_eq!(u32_at(20), 700 << 20, "the compressed size fits");
        assert_eq!(u32_at(24), IN_ZIP64, "the size does not");
        assert_eq!(u32_at(42), IN_ZIP64, "nor the offset");
        let extra = &record[46 + entry.name.len()..];
        let mut expected = vec![1, 0, 16, 0];
        expected.extend_from_slice(&(6_u64 << 30).to_le_bytes());
        expected.extend_from_slice(&(5_u64 << 30).to_le_bytes());
        assert_eq!(extra, expected);
        let small = Entry {
            offset: 0,
            size: 1 << 20,
            ..entry
        };
        let record = central_header(&small).expect("a record");
        assert_eq!(record.len(), 46 + small.name.len(), "no ZIP64 field");
        // The end of a directory past 4 GiB: a ZIP64 record, its locator,
        // and the end proper pointing to them.
        let end = central_end(13, 1_500, 6 << 30, (6 << 30) + 1_500);
        assert_eq!(&end[..4], &0x0606_4b50_u32.to_le_bytes());
        assert_eq!(&end[56..60], &0x0706_4b50_u32.to_le_bytes());
        assert_eq!(&end[76..80], &0x0605_4b50_u32.to_le_bytes());
        assert_eq!(end.len(), 98);
        assert_eq!(central_end(13, 1_500, 1 << 20, (1 << 20) + 1_500).len(), 22);
    }
}
