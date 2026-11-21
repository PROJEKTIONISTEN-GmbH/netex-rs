//! Where a feed is written: a directory of CSV files, or a zip archive of them.

use std::fs::File;
use std::io::{self, BufWriter, Write};
use std::path::{Path, PathBuf};

mod archive;

pub use archive::Archive;

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
    /// The names of the files written.
    written: Vec<String>,
}

/// The files the GTFS reference defines: a consumer reads each it finds in
/// a feed's directory.
const FILES_OF_A_FEED: [&str; 32] = [
    "agency.txt",
    "stops.txt",
    "routes.txt",
    "trips.txt",
    "stop_times.txt",
    "calendar.txt",
    "calendar_dates.txt",
    "fare_attributes.txt",
    "fare_rules.txt",
    "timeframes.txt",
    "rider_categories.txt",
    "fare_media.txt",
    "fare_products.txt",
    "fare_leg_rules.txt",
    "fare_leg_join_rules.txt",
    "fare_transfer_rules.txt",
    "areas.txt",
    "stop_areas.txt",
    "networks.txt",
    "route_networks.txt",
    "shapes.txt",
    "frequencies.txt",
    "transfers.txt",
    "pathways.txt",
    "levels.txt",
    "location_groups.txt",
    "location_group_stops.txt",
    "locations.geojson",
    "booking_rules.txt",
    "translations.txt",
    "feed_info.txt",
    "attributions.txt",
];

impl Directory {
    /// A feed written into `path`. It replaces a feed there: when it is
    /// finished, each file the GTFS reference defines that it did not write
    /// is removed — a `shapes.txt` or `feed_info.txt` of an earlier feed, a
    /// `fare_rules.txt` of another program's —, so that the directory holds
    /// this feed alone; files of other names stay.
    pub fn create(path: impl AsRef<Path>) -> io::Result<Directory> {
        std::fs::create_dir_all(path.as_ref())?;
        Ok(Directory {
            path: path.as_ref().to_path_buf(),
            file: None,
            written: Vec::new(),
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
        self.written.push(name.to_owned());
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
        for name in FILES_OF_A_FEED {
            if self.written.iter().any(|written| written == name) {
                continue;
            }
            match std::fs::remove_file(self.path.join(name)) {
                Err(error) if error.kind() != io::ErrorKind::NotFound => return Err(error),
                _ => {}
            }
        }
        Ok(())
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
