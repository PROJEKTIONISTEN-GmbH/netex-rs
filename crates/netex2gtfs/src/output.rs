//! Where a feed is written: a directory of CSV files, or a zip archive of them.

use std::fs::File;
use std::io::{self, BufWriter, Write};
use std::path::{Path, PathBuf};

use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

/// Where the files of a feed go, one after the other.
pub trait Output {
    /// Starts the file `name` (`stops.txt`, …); what is written next goes to it.
    fn start(&mut self, name: &str) -> io::Result<()>;

    /// Writes bytes to the file started last.
    fn write_all(&mut self, bytes: &[u8]) -> io::Result<()>;

    /// Ends the feed: every file is complete and in place.
    fn finish(self: Box<Self>) -> io::Result<()>;
}

/// The files of a feed in a directory, made if it is not there.
#[derive(Debug)]
pub struct Directory {
    path: PathBuf,
    file: Option<BufWriter<File>>,
}

impl Directory {
    /// A feed written into `path`.
    pub fn create(path: impl AsRef<Path>) -> io::Result<Directory> {
        std::fs::create_dir_all(path.as_ref())?;
        Ok(Directory {
            path: path.as_ref().to_path_buf(),
            file: None,
        })
    }
}

impl Output for Directory {
    fn start(&mut self, name: &str) -> io::Result<()> {
        if let Some(mut file) = self.file.take() {
            file.flush()?;
        }
        let file = File::create(self.path.join(name))?;
        self.file = Some(BufWriter::with_capacity(1 << 20, file));
        Ok(())
    }

    fn write_all(&mut self, bytes: &[u8]) -> io::Result<()> {
        self.file
            .as_mut()
            .expect("a file is started before it is written")
            .write_all(bytes)
    }

    fn finish(mut self: Box<Self>) -> io::Result<()> {
        if let Some(mut file) = self.file.take() {
            file.flush()?;
        }
        Ok(())
    }
}

/// The files of a feed in a zip archive, deflated.
#[derive(Debug)]
pub struct Archive {
    zip: ZipWriter<BufWriter<File>>,
}

impl Archive {
    /// A feed written as the archive `path`.
    pub fn create(path: impl AsRef<Path>) -> io::Result<Archive> {
        let file = BufWriter::with_capacity(1 << 20, File::create(path)?);
        Ok(Archive {
            zip: ZipWriter::new(file),
        })
    }
}

impl Output for Archive {
    fn start(&mut self, name: &str) -> io::Result<()> {
        let options = SimpleFileOptions::default()
            .compression_method(CompressionMethod::Deflated)
            // stop_times.txt of a national delivery is several gigabytes.
            .large_file(true);
        self.zip.start_file(name, options).map_err(io::Error::other)
    }

    fn write_all(&mut self, bytes: &[u8]) -> io::Result<()> {
        self.zip.write_all(bytes)
    }

    fn finish(self: Box<Self>) -> io::Result<()> {
        self.zip.finish().map_err(io::Error::other)?.flush()
    }
}

/// The output a path names: a zip archive if it ends in `.zip`, else a
/// directory.
pub fn at(path: impl AsRef<Path>) -> io::Result<Box<dyn Output>> {
    let path = path.as_ref();
    if path
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("zip"))
    {
        Ok(Box::new(Archive::create(path)?))
    } else {
        Ok(Box::new(Directory::create(path)?))
    }
}

/// One CSV file being written: a header, then rows of fields, each quoted
/// where it must be (RFC 4180, as GTFS requires).
pub(crate) struct Table<'o> {
    output: &'o mut dyn Output,
    line: Vec<u8>,
    rows: u64,
}

impl<'o> Table<'o> {
    /// Starts the file `name` with its header.
    pub(crate) fn start(
        output: &'o mut dyn Output,
        name: &str,
        header: &[&str],
    ) -> io::Result<Table<'o>> {
        output.start(name)?;
        let mut table = Table {
            output,
            line: Vec::with_capacity(256),
            rows: 0,
        };
        table.write_line(header.iter().copied())?;
        table.rows = 0;
        Ok(table)
    }

    /// Writes one row.
    pub(crate) fn row<'f>(&mut self, fields: impl IntoIterator<Item = &'f str>) -> io::Result<()> {
        self.write_line(fields)?;
        self.rows += 1;
        Ok(())
    }

    fn write_line<'f>(&mut self, fields: impl IntoIterator<Item = &'f str>) -> io::Result<()> {
        self.line.clear();
        for (position, field) in fields.into_iter().enumerate() {
            if position > 0 {
                self.line.push(b',');
            }
            if field
                .bytes()
                .any(|b| matches!(b, b',' | b'"' | b'\n' | b'\r'))
            {
                self.line.push(b'"');
                for byte in field.bytes() {
                    if byte == b'"' {
                        self.line.push(b'"');
                    }
                    self.line.push(byte);
                }
                self.line.push(b'"');
            } else {
                self.line.extend_from_slice(field.as_bytes());
            }
        }
        self.line.push(b'\n');
        self.output.write_all(&self.line)
    }

    /// How many rows were written.
    pub(crate) fn rows(&self) -> u64 {
        self.rows
    }
}
