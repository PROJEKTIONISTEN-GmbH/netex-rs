//! The timetable of a NeTEx delivery on one day: every journey of every line
//! that runs that day, with its calls.
//!
//! ```sh
//! cargo run --example timetable -- tests/fixtures/own/ch 2026-10-14
//! ```
//!
//! The delivery is a zip archive or a directory of NeTEx documents. It is
//! indexed lean — only the entities a timetable needs are typed — on every
//! core.

use netex_rs::dataset::Source;
use netex_rs::index::{Index, LineId, PassingTime};
use netex_rs::model::Date;
use netex_rs::stream::{ReadOptions, Selection};

/// A time of the operating day as `hh:mm`, past 24:00 after midnight.
fn clock(seconds: u32) -> String {
    format!("{:02}:{:02}", seconds / 3600, seconds / 60 % 60)
}

/// When a journey leaves a call, or arrives where it ends.
fn time(passing: &PassingTime) -> String {
    passing
        .departure
        .or(passing.arrival)
        .map_or_else(|| "--:--".to_owned(), clock)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut arguments = std::env::args().skip(1);
    let (Some(path), Some(day)) = (arguments.next(), arguments.next()) else {
        return Err("usage: timetable <delivery.zip | directory> <YYYY-MM-DD>".into());
    };
    let day = Date::from(day.as_str());
    let source = Source::open(&path)?;
    let index = Index::build(&source, &ReadOptions::new(Selection::routing()))?;

    for line in (0..index.line_count() as u32).map(LineId) {
        let name = index
            .line(line)
            .public_code
            .unwrap_or(index.line(line).name);
        for journey in index.journeys_of_line_on(line, &day) {
            let journey = index.journey(journey);
            let calls: Vec<String> = journey
                .stop_points
                .iter()
                .zip(journey.passing_times)
                .map(|(&point, passing)| {
                    format!("{} {}", time(passing), index.stop_point(point).name)
                })
                .collect();
            println!("{name:>6}  {}", calls.join(" → "));
            for headway in journey.headways {
                println!(
                    "        from {} to {}, every {} minutes",
                    clock(headway.first_departure),
                    clock(headway.last_departure),
                    headway.interval / 60
                );
            }
        }
    }
    for finding in index.findings() {
        eprintln!("{finding}");
    }
    Ok(())
}
