//! `netex`: check, count, convert, cut and merge NeTEx deliveries.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser, Subcommand, ValueEnum};
use netex2gtfs::{Delivery, Options, Publisher, Reading, StopIds};
use netex_cli::extract::{self, Area, Leaving};
use netex_cli::{stats, validate};
use netex_rs::dataset::Source;
use netex_rs::index::IndexBuilder;
use netex_rs::merge::{self, Input};
use netex_rs::model::Date;
use netex_rs::stream::{ReadOptions, Selection};

/// Exit status of a run that worked and found nothing refused.
const ACCEPTED: u8 = 0;
/// Exit status of `validate` when the policy refuses the delivery.
const REFUSED: u8 = 1;
/// Exit status of a run that could not do its work.
const FAILED: u8 = 2;

#[derive(Debug, Parser)]
#[command(
    name = "netex",
    version,
    about = "Check, count, convert, cut and merge NeTEx deliveries"
)]
struct Cli {
    /// Threads to read with; every core where not given.
    #[arg(long, global = true)]
    threads: Option<usize>,
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// The checking report of a delivery: every finding by class, with
    /// examples and the journeys it affects. Exits 0 when the policy accepts
    /// the delivery, 1 when it refuses it, 2 when the delivery cannot be read.
    Validate {
        /// A zip archive or a directory of NeTEx documents.
        source: PathBuf,
        /// A zip archive or a directory of documents read along with the
        /// source: a stop register published apart, as Norway's; may be given
        /// more than once.
        #[arg(long, value_name = "PATH")]
        with: Vec<PathBuf>,
        /// How the report is printed.
        #[arg(long, value_enum, default_value_t = Format::Human)]
        format: Format,
        /// Writes the report as JSON to this file as well.
        #[arg(long)]
        report: Option<PathBuf>,
        /// Refuses any finding of any class.
        #[arg(long)]
        strict: bool,
        /// Refuses any finding of this class (`unresolved`, `journey-without-travel-time`, …).
        #[arg(long, value_name = "CLASS")]
        refuse: Vec<String>,
        /// Tolerates at most so many findings of a class: `CLASS=COUNT`.
        #[arg(long, value_name = "CLASS=COUNT")]
        at_most: Vec<String>,
        /// How many examples each class lists.
        #[arg(long, default_value_t = validate::EXAMPLES)]
        examples: usize,
    },
    /// Stops, lines, journeys and journey days of a delivery, by operator.
    Stats {
        /// A zip archive or a directory of NeTEx documents.
        source: PathBuf,
        /// A zip archive or a directory of documents read along with the
        /// source: a stop register published apart, as Norway's; may be given
        /// more than once.
        #[arg(long, value_name = "PATH")]
        with: Vec<PathBuf>,
        /// The first day counted, `YYYY-MM-DD`.
        #[arg(long)]
        from: Option<String>,
        /// The last day counted, `YYYY-MM-DD`.
        #[arg(long)]
        to: Option<String>,
        /// How the statistics are printed.
        #[arg(long, value_enum, default_value_t = Format::Human)]
        format: Format,
    },
    /// Converts a delivery into another format: GTFS.
    Convert {
        /// A zip archive or a directory of NeTEx documents.
        source: PathBuf,
        /// A zip archive or a directory of documents read along with the
        /// source: a stop register published apart, as Norway's; may be given
        /// more than once.
        #[arg(long, value_name = "PATH")]
        with: Vec<PathBuf>,
        /// The format written.
        #[arg(long, value_enum)]
        to: Target,
        /// Where the feed is written: a zip archive if it ends in `.zip`, else a directory.
        output: PathBuf,
        /// Writes shapes from the service links' geometry.
        #[arg(long)]
        shapes: bool,
        /// Which identifiers the stops get.
        #[arg(long, value_enum, default_value_t = StopIdChoice::Netex)]
        stop_ids: StopIdChoice,
        /// The publisher's name, for feed_info.txt.
        #[arg(long, requires = "publisher_url")]
        publisher_name: Option<String>,
        /// The publisher's web address, for feed_info.txt.
        #[arg(long, requires = "publisher_name")]
        publisher_url: Option<String>,
        /// The feed's language, an IETF tag.
        #[arg(long)]
        language: Option<String>,
        /// The time zone of agencies where no document names one.
        #[arg(long)]
        time_zone: Option<String>,
        /// Writes the conversion's report as JSON to this file.
        #[arg(long)]
        report: Option<PathBuf>,
    },
    /// A closed part of a delivery: the journeys of the operators, lines, area
    /// and period given, with everything they refer to, written document by
    /// document as NeTEx of the source's release.
    Extract {
        /// A zip archive or a directory of NeTEx documents.
        source: PathBuf,
        /// A zip archive or a directory of documents read along with the
        /// source: a stop register published apart, as Norway's; may be given
        /// more than once.
        #[arg(long, value_name = "PATH")]
        with: Vec<PathBuf>,
        /// The directory the part is written to, new or empty.
        #[arg(long)]
        to: PathBuf,
        /// Journeys run by this operator; may be given more than once.
        #[arg(long, value_name = "ID")]
        operator: Vec<String>,
        /// Journeys of this line; may be given more than once.
        #[arg(long, value_name = "ID")]
        line: Vec<String>,
        /// Journeys calling in this box: west,south,east,north in degrees (WGS 84).
        #[arg(long, value_name = "W,S,E,N", conflicts_with = "area")]
        bbox: Option<String>,
        /// Journeys calling in the polygons of this GeoJSON file.
        #[arg(long, value_name = "FILE")]
        area: Option<PathBuf>,
        /// Journeys running on a day of this period, `YYYY-MM-DD..YYYY-MM-DD`.
        #[arg(long, value_name = "FIRST..LAST")]
        period: Option<String>,
        /// What becomes of a journey calling outside the area as well.
        #[arg(long, value_enum, default_value_t = LeavingChoice::Whole)]
        leaving: LeavingChoice,
        /// How the report is printed.
        #[arg(long, value_enum, default_value_t = Format::Human)]
        format: Format,
        /// Writes the report as JSON to this file as well.
        #[arg(long)]
        report: Option<PathBuf>,
    },
    /// Several deliveries into one: stops paired by identifier and global
    /// identifier, objects stated alike kept once, identifiers taken renamed
    /// into the delivery's codespace, journeys stated twice kept once — each
    /// delivery written under its rank and label.
    Merge {
        /// The deliveries, the home source of what they share first: `LABEL=PATH`
        /// or a path, labelled by its file name.
        #[arg(required = true, num_args = 2..)]
        deliveries: Vec<String>,
        /// The directory the merge is written to, new or empty.
        #[arg(long)]
        to: PathBuf,
        /// How the report is printed.
        #[arg(long, value_enum, default_value_t = Format::Human)]
        format: Format,
        /// Writes the report as JSON to this file as well.
        #[arg(long)]
        report: Option<PathBuf>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum LeavingChoice {
    /// Kept whole.
    Whole,
    /// Cut to its calls from the first to the last in the area.
    Cut,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum Format {
    Human,
    Json,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum Target {
    Gtfs,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum StopIdChoice {
    /// The NeTEx identifiers.
    Netex,
    /// The GlobalID (DHID, …) where a stop carries one, else its SLOID,
    /// numbered where several stops carry the same; else the NeTEx identifier.
    Global,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    if let Some(threads) = cli.threads {
        if let Err(error) = rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build_global()
        {
            eprintln!("netex: {error}");
            return ExitCode::from(FAILED);
        }
    }
    match run(cli.command) {
        Ok(status) => ExitCode::from(status),
        Err(message) => {
            eprintln!("netex: {message}");
            ExitCode::from(FAILED)
        }
    }
}

fn run(command: Command) -> Result<u8, String> {
    match command {
        Command::Validate {
            source,
            with,
            format,
            report,
            strict,
            refuse,
            at_most,
            examples,
        } => {
            let limits = limits(strict, &refuse, &at_most)?;
            let opened = open_with(&source, &with)?;
            let index = build_with_referrers(&opened).map_err(|e| e.to_string())?;
            let checked = validate::check(&index, &name_of(&source), &limits, examples);
            let json = serde_json::to_string_pretty(&checked).map_err(|e| e.to_string())?;
            if let Some(path) = report {
                std::fs::write(&path, &json).map_err(|e| format!("{}: {e}", path.display()))?;
            }
            match format {
                Format::Human => print!("{}", checked.human()),
                Format::Json => println!("{json}"),
            }
            Ok(if checked.refused { REFUSED } else { ACCEPTED })
        }
        Command::Stats {
            source,
            with,
            from,
            to,
            format,
        } => {
            let period = match (from, to) {
                (None, None) => None,
                (from, to) => Some((
                    date(from.as_deref().unwrap_or("0001-01-01"))?,
                    date(to.as_deref().unwrap_or("9999-12-31"))?,
                )),
            };
            let delivery = Delivery::read(&open_with(&source, &with)?, Reading::default())
                .map_err(|e| e.to_string())?;
            let figures = stats::stats(
                &delivery,
                &name_of(&source),
                period.as_ref().map(|(first, last)| (first, last)),
            );
            match format {
                Format::Human => print!("{}", figures.human()),
                Format::Json => println!(
                    "{}",
                    serde_json::to_string_pretty(&figures).map_err(|e| e.to_string())?
                ),
            }
            Ok(ACCEPTED)
        }
        Command::Convert {
            source,
            with,
            to: Target::Gtfs,
            output,
            shapes,
            stop_ids,
            publisher_name,
            publisher_url,
            language,
            time_zone,
            report,
        } => {
            let reading = Reading {
                geometry: shapes,
                ..Reading::default()
            };
            let delivery =
                Delivery::read(&open_with(&source, &with)?, reading).map_err(|e| e.to_string())?;
            let publisher = publisher_name
                .zip(publisher_url)
                .map(|(name, url)| Publisher {
                    name,
                    url,
                    language: language.clone(),
                });
            let options = Options {
                stop_ids: match stop_ids {
                    StopIdChoice::Netex => StopIds::Netex,
                    StopIdChoice::Global => StopIds::Global,
                },
                publisher,
                time_zone,
                shapes,
            };
            let out = netex2gtfs::output::at(&output)
                .map_err(|e| format!("{}: {e}", output.display()))?;
            let converted =
                netex2gtfs::convert(&delivery, &options, out).map_err(|e| e.to_string())?;
            let json = serde_json::to_string_pretty(&converted).map_err(|e| e.to_string())?;
            if let Some(path) = report {
                std::fs::write(&path, &json).map_err(|e| format!("{}: {e}", path.display()))?;
            }
            for (file, rows) in &converted.rows {
                eprintln!("{file}: {rows} rows");
            }
            for decision in &converted.decisions {
                eprintln!("{} × {}: {}", decision.count, decision.code, decision.what);
            }
            for not_carried in &converted.not_carried {
                eprintln!(
                    "not carried: {} × {} (first in {})",
                    not_carried.count, not_carried.element, not_carried.first_document
                );
            }
            Ok(ACCEPTED)
        }
        Command::Extract {
            source,
            with,
            to,
            operator,
            line,
            bbox,
            area,
            period,
            leaving,
            format,
            report,
        } => {
            let area = match (bbox, area) {
                (Some(bbox), _) => Some(Area::from_box(&bbox)?),
                (None, Some(path)) => {
                    let text = std::fs::read_to_string(&path)
                        .map_err(|e| format!("{}: {e}", path.display()))?;
                    Some(
                        Area::from_geojson(&text)
                            .map_err(|e| format!("{}: {e}", path.display()))?,
                    )
                }
                (None, None) => None,
            };
            let period = period
                .map(|period| {
                    let (first, last) = period
                        .split_once("..")
                        .ok_or_else(|| format!("`{period}` is no period: FIRST..LAST"))?;
                    Ok::<_, String>((date(first)?, date(last)?))
                })
                .transpose()?;
            let selection = extract::Selection {
                operators: operator,
                lines: line,
                area,
                period,
                leaving: match leaving {
                    LeavingChoice::Whole => Leaving::Whole,
                    LeavingChoice::Cut => Leaving::Cut,
                },
            };
            empty_directory(&to)?;
            let opened = open_with(&source, &with)?;
            let delivery =
                Delivery::read(&opened, Reading::default()).map_err(|e| e.to_string())?;
            let part =
                extract::extract(&opened, &delivery, &selection, &to).map_err(|e| e.to_string())?;
            let json = serde_json::to_string_pretty(&part).map_err(|e| e.to_string())?;
            if let Some(path) = report {
                std::fs::write(&path, &json).map_err(|e| format!("{}: {e}", path.display()))?;
            }
            match format {
                Format::Human => print!("{}", part.human()),
                Format::Json => println!("{json}"),
            }
            Ok(ACCEPTED)
        }
        Command::Merge {
            deliveries,
            to,
            format,
            report,
        } => {
            let inputs: Vec<Input> = deliveries
                .iter()
                .map(|delivery| {
                    let (label, path) = match delivery.split_once('=') {
                        Some((label, path)) => (label.to_owned(), PathBuf::from(path)),
                        None => {
                            let path = PathBuf::from(delivery);
                            (label_of(&path), path)
                        }
                    };
                    Input::new(&label, open(&path)?)
                })
                .collect::<Result<_, _>>()?;
            empty_directory(&to)?;
            let merged = merge::merge(&inputs, &to).map_err(|e| e.to_string())?;
            let json = serde_json::to_string_pretty(&merged).map_err(|e| e.to_string())?;
            if let Some(path) = report {
                std::fs::write(&path, &json).map_err(|e| format!("{}: {e}", path.display()))?;
            }
            match format {
                Format::Human => print!("{}", merged.human()),
                Format::Json => println!("{json}"),
            }
            Ok(ACCEPTED)
        }
    }
}

/// The label of a delivery named by its path alone: its file name without
/// extension, every character but letters, digits, `-` and `_` as `-`.
fn label_of(path: &Path) -> String {
    path.file_stem()
        .map(|stem| stem.to_string_lossy())
        .unwrap_or_default()
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' {
                c
            } else {
                '-'
            }
        })
        .collect()
}

/// Makes `path` a directory to write into, refusing one holding anything.
fn empty_directory(path: &Path) -> Result<(), String> {
    let failed = |e: std::io::Error| format!("{}: {e}", path.display());
    if path.exists() {
        if std::fs::read_dir(path).map_err(failed)?.next().is_some() {
            return Err(format!("{} is not empty", path.display()));
        }
        Ok(())
    } else {
        std::fs::create_dir_all(path).map_err(failed)
    }
}

fn open(path: &Path) -> Result<Source, String> {
    Source::open(path).map_err(|e| e.to_string())
}

/// The source at `path`, with the documents at each of `with` read along.
fn open_with(path: &Path, with: &[PathBuf]) -> Result<Source, String> {
    with.iter().try_fold(open(path)?, |source, other| {
        source.with(open(other)?).map_err(|e| e.to_string())
    })
}

/// The name a report gives the source: its file or directory name.
fn name_of(path: &Path) -> String {
    path.file_name().map_or_else(
        || path.display().to_string(),
        |name| name.to_string_lossy().into_owned(),
    )
}

/// The index of a source with its references kept, read on every thread.
fn build_with_referrers(source: &Source) -> netex_rs::Result<netex_rs::index::Index> {
    // The routing entities and the service links, whose geometry's axis order is checked.
    let options = ReadOptions::new(Selection::routing().with(netex_rs::stream::Kind::ServiceLink));
    let workers = source.read_each_with(
        || (IndexBuilder::new().with_referrers(), None),
        |(builder, failure): &mut (IndexBuilder, Option<netex_rs::Error>), entry, text| {
            if failure.is_some() {
                return;
            }
            if let Err(error) =
                text.and_then(|xml| builder.add_streamed(xml, &options, &entry.name))
            {
                *failure = Some(error.in_document(&entry.name));
            }
        },
    )?;
    let mut builders = Vec::with_capacity(workers.len());
    for (builder, failure) in workers {
        if let Some(error) = failure {
            return Err(error);
        }
        builders.push(builder);
    }
    Ok(IndexBuilder::merge_all(builders).finish())
}

/// The limits the command line sets.
fn limits(strict: bool, refuse: &[String], at_most: &[String]) -> Result<validate::Limits, String> {
    let class = |code: &str| {
        validate::Class::from_code(code).ok_or_else(|| {
            let known: Vec<&str> = validate::Class::all().map(validate::Class::code).collect();
            format!("no class `{code}`; the classes are {}", known.join(", "))
        })
    };
    let mut limits = if strict {
        validate::Limits::strict()
    } else {
        validate::Limits::default()
    };
    for code in refuse {
        limits = limits.at_most(class(code)?, 0);
    }
    for setting in at_most {
        let (code, count) = setting
            .split_once('=')
            .ok_or_else(|| format!("`{setting}` is no CLASS=COUNT"))?;
        let count = count
            .parse()
            .map_err(|_| format!("`{count}` in `{setting}` is no count"))?;
        limits = limits.at_most(class(code)?, count);
    }
    Ok(limits)
}

fn date(text: &str) -> Result<Date, String> {
    let date = Date::from(text);
    date.days_since_epoch()
        .map(|_| date)
        .ok_or_else(|| format!("`{text}` is no date (YYYY-MM-DD)"))
}
