//! The conversion: a delivery's index and gathered facts written as the files
//! of a GTFS feed, every decision counted in the report.

use std::collections::BTreeMap;
use std::fmt;
use std::io;

use netex_rs::crs;
use netex_rs::geometry::{reading_order, AxisOrder};
use netex_rs::index::{
    CalendarId, Concern, Index, JourneyId, LineId, PassingTime, StopId, StopKind, StopPointId,
};
use netex_rs::model::Date;
use rustc_hash::{FxHashMap, FxHashSet};

use crate::delivery::{Agency, Delivery};
use crate::gather::{Direction, Wheelchair};
use crate::names::Name;
use crate::output::{Output, Table};
use crate::report::{Decisions, NotCarried, Profile, Report};

/// Which identifiers a feed's stops get.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StopIds {
    /// The NeTEx identifier of the stop place, quay or stop point.
    #[default]
    Netex,
    /// The `GlobalID` — the German DHID, the Austrian ones — where a stop
    /// carries one, else its Swiss `SLOID`, else the NeTEx identifier; the
    /// second and each further stop of the feed carrying the same global
    /// identifier numbered `<id>#<n>`.
    Global,
}

/// Who publishes the feed: `feed_info.txt`, and the agency of lines that
/// name none.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Publisher {
    /// The publisher's name.
    pub name: String,
    /// The publisher's web address.
    pub url: String,
    /// The feed's language, an IETF tag; the delivery's where none is given.
    pub language: Option<String>,
}

/// How a delivery is converted.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Options {
    /// Which identifiers the stops get.
    pub stop_ids: StopIds,
    /// Who publishes the feed. Without one, `feed_info.txt` is written only
    /// where the delivery names a data source with a web address.
    pub publisher: Option<Publisher>,
    /// The time zone of agencies whose documents name none, where no
    /// document of the delivery names one.
    pub time_zone: Option<String>,
    /// Whether to write `shapes.txt` from the service links' geometry.
    pub shapes: bool,
}

/// Why a conversion failed.
#[derive(Debug)]
#[non_exhaustive]
pub enum Error {
    /// A file of the feed could not be written.
    Write(io::Error),
    /// No document names a time zone, the options give none, and the
    /// delivery's profile has none: GTFS needs one for every agency.
    NoTimeZone,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Write(error) => write!(f, "writing the feed: {error}"),
            Error::NoTimeZone => f.write_str(
                "no document of the delivery names a time zone (FrameDefaults/DefaultLocale/TimeZone), nor is it of a profile whose country has one; give one",
            ),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Write(error) => Some(error),
            Error::NoTimeZone => None,
        }
    }
}

impl From<io::Error> for Error {
    fn from(error: io::Error) -> Self {
        Error::Write(error)
    }
}

/// Writes the feed of `delivery` to `output` and reports what it did.
pub fn convert(
    delivery: &Delivery,
    options: &Options,
    output: Box<dyn Output>,
) -> Result<Report, Error> {
    let mut conversion = Conversion::new(delivery, options)?;
    let mut output = output;
    let mut rows = BTreeMap::new();
    conversion.plan();
    rows.insert("agency.txt".to_owned(), conversion.agencies(&mut *output)?);
    rows.insert("levels.txt".to_owned(), conversion.levels(&mut *output)?);
    rows.insert("stops.txt".to_owned(), conversion.stops(&mut *output)?);
    rows.insert("routes.txt".to_owned(), conversion.routes(&mut *output)?);
    let (trips, stop_times, frequencies) = conversion.trips(&mut *output)?;
    rows.insert("trips.txt".to_owned(), trips);
    rows.insert("stop_times.txt".to_owned(), stop_times);
    rows.insert("frequencies.txt".to_owned(), frequencies);
    let (calendar, dates) = conversion.calendars(&mut *output)?;
    rows.insert("calendar.txt".to_owned(), calendar);
    rows.insert("calendar_dates.txt".to_owned(), dates);
    rows.insert(
        "transfers.txt".to_owned(),
        conversion.transfers(&mut *output)?,
    );
    if options.shapes {
        rows.insert("shapes.txt".to_owned(), conversion.shapes(&mut *output)?);
    }
    for (file, count) in conversion.feed_info(&mut *output)? {
        rows.insert(file.to_owned(), count);
    }
    output.finish()?;
    Ok(conversion.report(rows))
}

/// Where a call takes place in the feed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Place {
    /// At a stop place or quay of the index.
    Stop(StopId),
    /// At a stop point no stop place or quay is assigned, at its own position.
    Point(StopPointId),
}

/// A row of `stops.txt`.
#[derive(Debug, Clone)]
struct StopRow {
    id: String,
    name: String,
    position: (f64, f64),
    /// 0 a stop or platform, 1 a station.
    location_type: u8,
    parent: Option<String>,
    wheelchair: Option<Wheelchair>,
    platform: Option<String>,
    level: Option<String>,
}

/// A trip as planned: the journey, its identifier and its route; its calls
/// are found again as they are written ([`Conversion::calls`]).
#[derive(Debug)]
struct TripPlan {
    journey: JourneyId,
    id: String,
    route: u32,
}

struct Conversion<'d> {
    delivery: &'d Delivery,
    index: &'d Index,
    options: &'d Options,
    decisions: Decisions,
    profile: Profile,
    /// The time zone of agencies whose documents name none.
    time_zone: String,
    /// The feed stop of each place calls are made at, by its row.
    place_rows: FxHashMap<Place, u32>,
    /// The station row of each stop place that is a parent.
    station_rows: FxHashMap<StopId, u32>,
    stop_rows: Vec<StopRow>,
    /// The routes: the line each is, its identifier, its agency, the
    /// document of the journey the line was first met with.
    routes: Vec<(LineId, String, Option<u32>, Name)>,
    route_of_line: FxHashMap<LineId, u32>,
    /// The agencies: the organisation each is, or the publisher.
    agencies: Vec<AgencyRow>,
    trips: Vec<TripPlan>,
    trip_of_journey: FxHashMap<JourneyId, u32>,
    calendars_used: Vec<CalendarId>,
    shape_of_trip: FxHashMap<u32, u32>,
    shapes: Vec<Vec<(f64, f64)>>,
    levels_used: Vec<Name>,
    /// The sum of the positions of the quays of each stop place, and how many.
    quay_positions: FxHashMap<StopId, (f64, f64, u32)>,
}

#[derive(Debug, Clone)]
struct AgencyRow {
    id: String,
    name: String,
    url: String,
    time_zone: String,
    phone: Option<String>,
}

impl<'d> Conversion<'d> {
    fn new(delivery: &'d Delivery, options: &'d Options) -> Result<Self, Error> {
        let index = delivery.index();
        let mut documents: Vec<(&str, Name)> = delivery
            .gather
            .documents
            .keys()
            .map(|&document| (delivery.text(document), document))
            .collect();
        documents.sort_unstable();
        let epip = documents
            .iter()
            .any(|&(_, document)| delivery.document_facts(document).epip);
        let profile = profile(index, epip);
        let mut decisions = Decisions::default();
        let named = documents
            .iter()
            .find_map(|&(_, document)| delivery.document_facts(document).time_zone)
            .map(|zone| delivery.text(zone).to_owned());
        let time_zone = match named.or_else(|| options.time_zone.clone()) {
            Some(zone) => zone,
            None => {
                // The Swiss and Austrian documents state offsets from UTC, no
                // zone: their profile's country is the zone.
                let zone = profile.time_zone().ok_or(Error::NoTimeZone)?;
                decisions.take(
                    "time-zone-of-profile",
                    "no document names a time zone and none is given: the time zone of the delivery's profile",
                    || zone.to_owned(),
                );
                zone.to_owned()
            }
        };
        Ok(Conversion {
            delivery,
            index,
            options,
            decisions,
            profile,
            time_zone,
            place_rows: FxHashMap::default(),
            station_rows: FxHashMap::default(),
            stop_rows: Vec::new(),
            routes: Vec::new(),
            route_of_line: FxHashMap::default(),
            agencies: Vec::new(),
            trips: Vec::new(),
            trip_of_journey: FxHashMap::default(),
            calendars_used: Vec::new(),
            shape_of_trip: FxHashMap::default(),
            shapes: Vec::new(),
            levels_used: Vec::new(),
            quay_positions: FxHashMap::default(),
        })
    }

    /// Decides every row before any is written: which journeys become trips
    /// and on which routes, which places their calls are at, the stops and
    /// stations those are, the agencies of the routes, the shapes.
    fn plan(&mut self) {
        let mut used: Vec<Place> = Vec::new();
        let mut seen: FxHashSet<Place> = FxHashSet::default();
        let mut candidates: Vec<(JourneyId, LineId)> = Vec::new();
        let mut placed: Vec<usize> = Vec::new();
        for journey in (0..self.index.journey_count() as u32).map(JourneyId) {
            let record = self.index.journey(journey);
            let Some(line) = record.line else {
                self.decisions.take(
                    "journey-without-line",
                    "a journey naming no line, nor a pattern naming one: left out, GTFS has no trip without a route",
                    || record.id.to_owned(),
                );
                continue;
            };
            if self.index.days_of(journey).next().is_none() {
                self.decisions.take(
                    "journey-without-days",
                    "a journey running on no day: left out",
                    || record.id.to_owned(),
                );
                continue;
            }
            placed.clear();
            for (position, &point) in record.stop_points.iter().enumerate() {
                let Some(place) = self.place_of(point) else {
                    let stop_point = self.index.stop_point(point);
                    self.decisions.take(
                        "call-without-place",
                        "a call at a stop point assigned to no stop place or quay and without a position of its own: left out of its trip",
                        || format!("{} in {}", stop_point.id, record.id),
                    );
                    continue;
                };
                if seen.insert(place) {
                    used.push(place);
                }
                placed.push(position);
            }
            if placed.len() < 2 {
                self.decisions.take(
                    "trip-without-two-calls",
                    "a journey left with fewer than two calls at a place: left out",
                    || record.id.to_owned(),
                );
                continue;
            }
            if goes_back(record.passing_times, placed.iter().copied()) {
                self.decisions.take(
                    "trip-time-goes-back",
                    "a journey whose passing times go back in time from one call to the next: left out, GTFS has no such trip",
                    || record.id.to_owned(),
                );
                continue;
            }
            candidates.push((journey, line));
        }
        self.plan_stops(&used);
        for (journey, line) in candidates {
            if self.calls(journey).len() < 2 {
                let record = self.index.journey(journey);
                self.decisions.take(
                    "trip-at-unplaced-stops",
                    "a journey calling at a stop the feed cannot place: left out",
                    || record.id.to_owned(),
                );
                continue;
            }
            let route = self.route_of(line, journey);
            self.calendars_used.push(self.index.calendar_of(journey));
            let trip = u32::try_from(self.trips.len()).expect("fewer than 2^32 trips");
            self.trip_of_journey.insert(journey, trip);
            self.trips.push(TripPlan {
                journey,
                id: String::new(),
                route,
            });
        }
        self.calendars_used.sort_unstable();
        self.calendars_used.dedup();
        self.name_trips();
        if self.options.shapes {
            self.plan_shapes();
        }
    }

    /// Where a call at a stop point takes place: at its stop, else — for a
    /// stop point placed at none — at its own position, if it has one.
    fn place_of(&self, point: StopPointId) -> Option<Place> {
        match self.index.stop_of_stop_point(point) {
            Some(stop) => Some(Place::Stop(stop)),
            None => {
                let record = self.index.stop_point(point);
                record
                    .latitude
                    .zip(record.longitude)
                    .map(|_| Place::Point(point))
            }
        }
    }

    /// The calls of a journey the feed writes: each by its position among the
    /// journey's calls, with the row of its stop.
    fn calls(&self, journey: JourneyId) -> Vec<(usize, u32)> {
        self.index
            .journey(journey)
            .stop_points
            .iter()
            .enumerate()
            .filter_map(|(position, &point)| {
                let row = self.place_rows.get(&self.place_of(point)?)?;
                Some((position, *row))
            })
            .collect()
    }

    /// The rows of `stops.txt`: every place a call is made at — quays under
    /// their stop place as a station, a stop place called at itself as a stop
    /// (under a stop of its own where its quays make it a station), a stop
    /// point no stop is assigned at its own position.
    fn plan_stops(&mut self, used: &[Place]) {
        let index = self.index;
        for stop in index.stops() {
            let quay = index.stop(stop);
            if let (Some(parent), Some((latitude, longitude))) =
                (quay.parent, quay.latitude.zip(quay.longitude))
            {
                let sum = self.quay_positions.entry(parent).or_insert((0.0, 0.0, 0));
                *sum = (sum.0 + latitude, sum.1 + longitude, sum.2 + 1);
            }
        }
        let mut parents: Vec<StopId> = used
            .iter()
            .filter_map(|place| match place {
                Place::Stop(stop) => self.station_of(*stop),
                Place::Point(_) => None,
            })
            .collect();
        parents.sort_unstable();
        parents.dedup();
        let mut ids = UniqueIds::default();
        for &parent in &parents {
            let Some(row) = self.stop_row(parent, &mut ids, 1, None, "") else {
                continue;
            };
            let at = self.push_row(row);
            self.station_rows.insert(parent, at);
        }
        let mut ordered: Vec<Place> = used.to_vec();
        ordered.sort_unstable_by_key(|place| match place {
            Place::Stop(stop) => (0, stop.0),
            Place::Point(point) => (1, point.0),
        });
        for place in ordered {
            let row = match place {
                Place::Stop(stop) => {
                    let record = index.stop(stop);
                    let parent_row = self
                        .station_of(stop)
                        .and_then(|p| self.station_rows.get(&p).copied());
                    if record.kind == StopKind::StopPlace && self.station_rows.contains_key(&stop) {
                        // Called at itself and the parent of quays: a stop of its own
                        // under itself as a station.
                        let station = self.station_rows[&stop];
                        self.decisions.take(
                            "stop-place-called-at-beside-its-quays",
                            "a stop place called at itself and at its quays: the calls at the place go to a stop of its own, `<id>#place`, under the place as a station",
                            || record.id.to_owned(),
                        );
                        self.stop_row(stop, &mut ids, 0, Some(station), "#place")
                    } else {
                        self.stop_row(stop, &mut ids, 0, parent_row, "")
                    }
                }
                Place::Point(point) => {
                    let record = index.stop_point(point);
                    self.decisions.take(
                        "stop-point-as-stop",
                        "a stop point assigned to no stop place or quay: a stop of its own, at its own position",
                        || record.id.to_owned(),
                    );
                    Some(StopRow {
                        id: ids.unique(record.id),
                        name: record.name.to_owned(),
                        position: (
                            record
                                .latitude
                                .expect("a stop point placed only with a position"),
                            record
                                .longitude
                                .expect("a stop point placed only with a position"),
                        ),
                        location_type: 0,
                        parent: None,
                        wheelchair: None,
                        platform: None,
                        level: None,
                    })
                }
            };
            match row {
                Some(row) => {
                    let at = self.push_row(row);
                    self.place_rows.insert(place, at);
                }
                None => self.decisions.take(
                    "stop-without-position",
                    "a stop place or quay called at, without a position of its own, its station's or its quays': not in the feed, nor the calls at it",
                    || match place {
                        Place::Stop(stop) => index.stop(stop).id.to_owned(),
                        Place::Point(point) => index.stop_point(point).id.to_owned(),
                    },
                ),
            }
        }
    }

    /// The station a stop stands under in the feed: a quay's stop place, or
    /// the parent site that stop place is within; a stop place's parent site.
    /// A stop place within a parent site (`ParentSiteRef`) — as the Hessian
    /// and RMV stop places of the German delivery are, the parent carrying
    /// the DHID — hands its quays to the parent site: GTFS stations do not nest.
    fn station_of(&self, stop: StopId) -> Option<StopId> {
        let record = self.index.stop(stop);
        match record.kind {
            StopKind::Quay => {
                let place = record.parent?;
                Some(self.parent_site_of(place).unwrap_or(place))
            }
            StopKind::StopPlace => self.parent_site_of(stop),
        }
    }

    /// The outermost parent site of a stop place, if it is within one the
    /// delivery defines.
    fn parent_site_of(&self, place: StopId) -> Option<StopId> {
        let mut site = None;
        let mut current = place;
        // A parent site within a parent site, to the outermost; where the
        // delivery states a cycle, it ends where it closes.
        let mut seen = vec![place];
        loop {
            let id = self.delivery.name(self.index.stop(current).id)?;
            let parent = self
                .delivery
                .gather
                .stops
                .get(&id)
                .and_then(|facts| facts.parent_site)
                .and_then(|parent| self.index.stop_by_id(self.delivery.text(parent)))
                .filter(|parent| !seen.contains(parent));
            let Some(parent) = parent else {
                return site;
            };
            seen.push(parent);
            site = Some(parent);
            current = parent;
        }
    }

    fn push_row(&mut self, row: StopRow) -> u32 {
        self.stop_rows.push(row);
        u32::try_from(self.stop_rows.len() - 1).expect("fewer than 2^32 stops")
    }

    /// The row of a stop place or quay as a stop (0) or a station (1), under
    /// the station row `parent`; none where it has no position — a quay
    /// takes its stop place's, a station the mean of its quays'.
    fn stop_row(
        &mut self,
        stop: StopId,
        ids: &mut UniqueIds,
        location_type: u8,
        parent: Option<u32>,
        suffix: &str,
    ) -> Option<StopRow> {
        let index = self.index;
        let record = index.stop(stop);
        let own = record.latitude.zip(record.longitude);
        let position = own.or_else(|| match record.kind {
            StopKind::Quay => {
                let place = index.stop(record.parent?);
                let at = place.latitude.zip(place.longitude)?;
                self.decisions.take(
                    "quay-at-its-stop-place",
                    "a quay without a position: placed at its stop place",
                    || record.id.to_owned(),
                );
                Some(at)
            }
            StopKind::StopPlace => {
                let &(latitude, longitude, count) = self.quay_positions.get(&stop)?;
                self.decisions.take(
                    "stop-place-at-its-quays",
                    "a stop place without a position: placed at the mean of its quays'",
                    || record.id.to_owned(),
                );
                Some((latitude / f64::from(count), longitude / f64::from(count)))
            }
        })?;
        let parent_station = parent.map(|row| self.stop_rows[row as usize].id.clone());
        let name = if record.name.is_empty() {
            let parent_name = record.parent.map(|p| index.stop(p).name).unwrap_or("");
            self.decisions.take(
                "stop-named-by-its-station",
                "a stop without a name: named as its stop place",
                || record.id.to_owned(),
            );
            parent_name.to_owned()
        } else {
            record.name.to_owned()
        };
        let facts = self
            .delivery
            .name(record.id)
            .and_then(|id| self.delivery.gather.stops.get(&id).copied())
            .unwrap_or_default();
        let level = facts.level.filter(|level| {
            let known = self
                .delivery
                .gather
                .levels
                .get(level)
                .is_some_and(|facts| facts.index.is_some());
            if !known {
                self.decisions.take(
                    "level-without-number",
                    "a level stating no number (PublicCode, RelativeLevelOrder): the quay is written without it",
                    || record.id.to_owned(),
                );
            }
            known
        });
        if let Some(level) = level {
            self.levels_used.push(level);
        }
        // With global stop identifiers, a stop's GlobalID where it carries
        // one, else its SLOID.
        let global = record
            .dhid
            .or_else(|| facts.sloid.map(|sloid| self.delivery.text(sloid)));
        let base = match (self.options.stop_ids, global) {
            (StopIds::Global, Some(global)) => format!("{global}{suffix}"),
            _ => format!("{}{suffix}", record.id),
        };
        let id = ids.unique(&base);
        if id != base {
            self.decisions.take(
                "stop-made-unique",
                "a global identifier several stops of the feed carry: the second and each after it numbered, `<id>#<n>`",
                || base.clone(),
            );
        }
        Some(StopRow {
            id,
            name,
            position,
            location_type,
            parent: parent_station,
            wheelchair: facts.wheelchair,
            platform: facts.platform.map(|p| self.delivery.text(p).to_owned()),
            level: level.map(|l| self.delivery.text(l).to_owned()),
        })
    }

    /// The route of a line: one per NeTEx identifier — the versions of a
    /// line are one route —, unless the definitions of one identifier differ
    /// in name, number, mode or agency, which makes routes of their own.
    fn route_of(&mut self, line: LineId, journey: JourneyId) -> u32 {
        if let Some(&route) = self.route_of_line.get(&line) {
            return route;
        }
        let record = self.index.line(line);
        let document = self
            .delivery
            .name(self.index.journey(journey).document)
            .unwrap_or(Name::MAX);
        let agency = self
            .delivery
            .agency_of_line_in(line, document)
            .map(|organisation| self.delivery.agency(organisation))
            .or_else(|| self.delivery.agency_of_journey(journey));
        let agency = self.agency_row(agency, record.id);
        let same = self
            .routes
            .iter()
            .position(|&(other, ref id, other_agency, _)| {
                let other_record = self.index.line(other);
                id == record.id
                    && other_agency == agency
                    && (
                        other_record.name,
                        other_record.public_code,
                        other_record.mode,
                    ) == (record.name, record.public_code, record.mode)
            });
        let route = match same {
            Some(route) => u32::try_from(route).expect("fewer than 2^32 routes"),
            None => {
                let taken = self
                    .routes
                    .iter()
                    .filter(|(_, id, _, _)| id == record.id)
                    .count();
                let id = if taken == 0 {
                    record.id.to_owned()
                } else {
                    self.decisions.take(
                        "route-made-unique",
                        "a line defined again with another name, number, mode or agency: a route of its own, `<id>#<n>`",
                        || record.id.to_owned(),
                    );
                    format!("{}#{}", record.id, taken + 1)
                };
                self.routes.push((line, id, agency, document));
                u32::try_from(self.routes.len() - 1).expect("fewer than 2^32 routes")
            }
        };
        self.route_of_line.insert(line, route);
        route
    }

    /// The agency row of an organisation; the publisher's for a line naming
    /// none.
    fn agency_row(&mut self, agency: Option<Agency<'_>>, line: &str) -> Option<u32> {
        let row = match agency {
            Some(agency) => {
                if let Some(at) = self.agencies.iter().position(|row| row.id == agency.id) {
                    return Some(u32::try_from(at).expect("fewer than 2^32 agencies"));
                }
                let url = match agency.url {
                    Some(url) => url.to_owned(),
                    None => {
                        self.decisions.take(
                            "agency-without-url",
                            "an organisation stating no web address: the publisher's is written, GTFS requires one",
                            || agency.id.to_owned(),
                        );
                        self.options
                            .publisher
                            .as_ref()
                            .map_or_else(String::new, |publisher| publisher.url.clone())
                    }
                };
                let time_zone = self
                    .delivery
                    .document_facts(agency.document)
                    .time_zone
                    .map_or_else(
                        || self.time_zone.clone(),
                        |zone| self.delivery.text(zone).to_owned(),
                    );
                AgencyRow {
                    id: agency.id.to_owned(),
                    name: agency.name.to_owned(),
                    url,
                    time_zone,
                    phone: agency.phone.map(str::to_owned),
                }
            }
            None => {
                self.decisions.take(
                    "route-without-agency",
                    "a line naming no operator or authority the delivery defines: the publisher given is its agency; without one, the route names none",
                    || line.to_owned(),
                );
                let publisher = self.options.publisher.as_ref()?;
                if let Some(at) = self
                    .agencies
                    .iter()
                    .position(|row| row.id == PUBLISHER_AGENCY)
                {
                    return Some(u32::try_from(at).expect("fewer than 2^32 agencies"));
                }
                AgencyRow {
                    id: PUBLISHER_AGENCY.to_owned(),
                    name: publisher.name.clone(),
                    url: publisher.url.clone(),
                    time_zone: self.time_zone.clone(),
                    phone: None,
                }
            }
        };
        self.agencies.push(row);
        Some(u32::try_from(self.agencies.len() - 1).expect("fewer than 2^32 agencies"))
    }

    /// Each trip's identifier: its journey's, made unique where the index
    /// holds two journeys of one identifier — stated otherwise in two
    /// documents, or two runs of one template.
    fn name_trips(&mut self) {
        let mut ids = UniqueIds::default();
        for trip in &mut self.trips {
            let id = self.index.journey(trip.journey).id;
            let unique = ids.unique(id);
            if unique != id {
                self.decisions.take(
                    "trip-made-unique",
                    "a journey identifier the index holds twice — stated otherwise in two documents —: the second trip `<id>#<n>`",
                    || id.to_owned(),
                );
            }
            trip.id = unique;
        }
    }

    /// The shapes: for each trip, the geometry of the service links between
    /// its calls, one after the other, read in each link's reference system
    /// and axis order; trips of the same links share a shape. A trip with a
    /// pair of calls no link joins has none.
    fn plan_shapes(&mut self) {
        let mut shape_of_links: FxHashMap<Vec<u32>, u32> = FxHashMap::default();
        for trip_number in 0..self.trips.len() {
            let trip = &self.trips[trip_number];
            let record = self.index.journey(trip.journey);
            let Some(document) = self.delivery.name(record.document) else {
                continue;
            };
            let links: Option<Vec<u32>> = self
                .calls(trip.journey)
                .windows(2)
                .map(|pair| {
                    let from = self
                        .delivery
                        .name(self.index.stop_point(record.stop_points[pair[0].0]).id)?;
                    let to = self
                        .delivery
                        .name(self.index.stop_point(record.stop_points[pair[1].0]).id)?;
                    self.delivery.links.get(&(document, from, to)).copied()
                })
                .collect();
            let Some(links) = links else {
                if !self.delivery.links.is_empty() {
                    self.decisions.take(
                        "shape-incomplete",
                        "a trip with a pair of calls no service link with a geometry joins: no shape",
                        || record.id.to_owned(),
                    );
                }
                continue;
            };
            let next = u32::try_from(shape_of_links.len()).expect("fewer than 2^32 shapes");
            let shape = match shape_of_links.get(&links) {
                Some(&shape) => shape,
                None => {
                    let points = self.shape_points(&links, document);
                    self.shapes.push(points);
                    shape_of_links.insert(links, next);
                    next
                }
            };
            let trip_number = u32::try_from(trip_number).expect("fewer than 2^32 trips");
            self.shape_of_trip.insert(trip_number, shape);
        }
    }

    /// The positions of `links` one after the other, as latitude and
    /// longitude: each link read in its system — its own `srsName`, else its
    /// document's default — in the order its first position lies near its
    /// start, a position repeated where one link ends and the next begins
    /// written once.
    fn shape_points(&mut self, links: &[u32], document: Name) -> Vec<(f64, f64)> {
        let gather = &self.delivery.gather;
        let default_system = self
            .delivery
            .document_facts(document)
            .location_system
            .map(|system| gather.names.text(system));
        let mut points: Vec<(f64, f64)> = Vec::new();
        for &link in links {
            let link = gather.links[link as usize];
            let named = link.system.map(|system| gather.names.text(system));
            let Ok(system) = crs::system_of(named, default_system) else {
                self.decisions.take(
                    "shape-in-unknown-system",
                    "a service link geometry in a reference system the crate does not know: its stretch of the shape is left out",
                    || gather.names.text(link.from).to_owned(),
                );
                continue;
            };
            let written = &gather.positions[link.positions.0 as usize..link.positions.1 as usize];
            let start = self
                .index
                .stop_point_by_id(gather.names.text(link.from))
                .and_then(|point| self.position_of_stop_point(point));
            let first = written[0];
            let readings = [
                system.to_wgs84(first.0, first.1),
                system.to_wgs84(first.1, first.0),
            ];
            let swapped = start
                .is_some_and(|start| reading_order(readings, start) == AxisOrder::LongitudeFirst);
            for &(a, b) in written {
                let point = if swapped {
                    system.to_wgs84(b, a)
                } else {
                    system.to_wgs84(a, b)
                };
                if points.last() != Some(&point) {
                    points.push(point);
                }
            }
        }
        points
    }

    /// Where a stop point is: its own position, else its stop's, else that
    /// stop's stop place's.
    fn position_of_stop_point(&self, point: StopPointId) -> Option<(f64, f64)> {
        let record = self.index.stop_point(point);
        record.latitude.zip(record.longitude).or_else(|| {
            let stop = self.index.stop(record.stop?);
            stop.latitude.zip(stop.longitude).or_else(|| {
                let place = self.index.stop(stop.parent?);
                place.latitude.zip(place.longitude)
            })
        })
    }

    fn agencies(&mut self, output: &mut dyn Output) -> io::Result<u64> {
        let mut table = Table::start(
            output,
            "agency.txt",
            &[
                "agency_id",
                "agency_name",
                "agency_url",
                "agency_timezone",
                "agency_phone",
            ],
        )?;
        for agency in &self.agencies {
            table.row([
                agency.id.as_str(),
                agency.name.as_str(),
                agency.url.as_str(),
                agency.time_zone.as_str(),
                agency.phone.as_deref().unwrap_or(""),
            ])?;
        }
        Ok(table.rows())
    }

    fn levels(&mut self, output: &mut dyn Output) -> io::Result<u64> {
        self.levels_used
            .sort_unstable_by_key(|&level| self.delivery.text(level));
        self.levels_used.dedup();
        let mut table = Table::start(
            output,
            "levels.txt",
            &["level_id", "level_index", "level_name"],
        )?;
        for &level in &self.levels_used {
            let facts = self.delivery.gather.levels[&level];
            let index = facts.index.expect("only levels with a number are used");
            table.row([
                self.delivery.text(level),
                &index.to_string(),
                facts.name.map_or("", |name| self.delivery.text(name)),
            ])?;
        }
        Ok(table.rows())
    }

    fn stops(&mut self, output: &mut dyn Output) -> io::Result<u64> {
        let mut table = Table::start(
            output,
            "stops.txt",
            &[
                "stop_id",
                "stop_name",
                "stop_lat",
                "stop_lon",
                "location_type",
                "parent_station",
                "wheelchair_boarding",
                "platform_code",
                "level_id",
            ],
        )?;
        for row in &self.stop_rows {
            table.row([
                row.id.as_str(),
                row.name.as_str(),
                &format!("{:.7}", row.position.0),
                &format!("{:.7}", row.position.1),
                if row.location_type == 1 { "1" } else { "0" },
                row.parent.as_deref().unwrap_or(""),
                wheelchair(row.wheelchair),
                row.platform.as_deref().unwrap_or(""),
                row.level.as_deref().unwrap_or(""),
            ])?;
        }
        Ok(table.rows())
    }

    fn routes(&mut self, output: &mut dyn Output) -> io::Result<u64> {
        let mut table = Table::start(
            output,
            "routes.txt",
            &[
                "route_id",
                "agency_id",
                "route_short_name",
                "route_long_name",
                "route_type",
                "route_color",
                "route_text_color",
            ],
        )?;
        for &(line, ref id, agency, document) in &self.routes {
            let record = self.index.line(line);
            let facts = self
                .delivery
                .name(record.id)
                .and_then(|id| self.delivery.line_facts(document, id));
            let short = record
                .public_code
                .or_else(|| {
                    facts
                        .and_then(|f| f.short_name)
                        .map(|n| self.delivery.text(n))
                })
                .unwrap_or("");
            // A long name repeating the short one says nothing more.
            let long = if record.name == short {
                ""
            } else {
                record.name
            };
            let route_type = route_type(record.mode);
            if route_type.is_none() {
                self.decisions.take(
                    "route-type-unknown",
                    "a line of a mode GTFS has no route type for, or of none: written as a bus (3)",
                    || record.id.to_owned(),
                );
            }
            let text = |name: Option<Name>| name.map_or("", |n| self.delivery.text(n));
            table.row([
                id.as_str(),
                agency.map_or("", |a| self.agencies[a as usize].id.as_str()),
                short,
                long,
                route_type.unwrap_or("3"),
                text(facts.and_then(|f| f.colour)),
                text(facts.and_then(|f| f.text_colour)),
            ])?;
        }
        Ok(table.rows())
    }

    /// `trips.txt`, `frequencies.txt` and `stop_times.txt`, trip by trip;
    /// the stop times, the large file, written as they are made.
    fn trips(&mut self, output: &mut dyn Output) -> io::Result<(u64, u64, u64)> {
        let mut table = Table::start(
            output,
            "trips.txt",
            &[
                "route_id",
                "service_id",
                "trip_id",
                "trip_headsign",
                "trip_short_name",
                "direction_id",
                "shape_id",
                "wheelchair_accessible",
            ],
        )?;
        let mut frequencies: Vec<[String; 4]> = Vec::new();
        for number in 0..self.trips.len() {
            let row = self.trip_row(number, &mut frequencies);
            table.row(row.iter().map(String::as_str))?;
        }
        let trips = table.rows();
        let mut table = Table::start(
            output,
            "frequencies.txt",
            &[
                "trip_id",
                "start_time",
                "end_time",
                "headway_secs",
                "exact_times",
            ],
        )?;
        for row in &frequencies {
            table.row(row.iter().map(String::as_str).chain(["0"]))?;
        }
        let frequency_rows = table.rows();
        let mut table = Table::start(
            output,
            "stop_times.txt",
            &[
                "trip_id",
                "arrival_time",
                "departure_time",
                "stop_id",
                "stop_sequence",
                "pickup_type",
                "drop_off_type",
                "stop_headsign",
            ],
        )?;
        for number in 0..self.trips.len() {
            self.stop_times(number, &mut table)?;
        }
        Ok((trips, table.rows(), frequency_rows))
    }

    /// The row of `trips.txt` of the trip numbered `number`; its headway
    /// groups added to `frequencies` (trip, start, end, headway).
    fn trip_row(&mut self, number: usize, frequencies: &mut Vec<[String; 4]>) -> [String; 8] {
        let trip = &self.trips[number];
        let record = self.index.journey(trip.journey);
        let document = self.delivery.name(record.document).unwrap_or(Name::MAX);
        let delivery = self.delivery;
        let facts = delivery
            .name(record.id)
            .and_then(|id| delivery.journey_facts(document, id));
        let headsign = self
            .calls(trip.journey)
            .first()
            .and_then(|&(position, _)| self.displays(trip.journey, document)[position]);
        let short_name = facts.and_then(|facts| {
            let number = facts
                .train_number
                .and_then(|number| delivery.train_number(document, number))
                .map(|number| number.advertised);
            number.or(facts.public_code)
        });
        let number_u32 = u32::try_from(number).expect("fewer than 2^32 trips");
        let row = [
            self.routes[trip.route as usize].1.clone(),
            service_id(self.index.calendar_of(trip.journey)),
            trip.id.clone(),
            headsign.map_or_else(String::new, |h| delivery.text(h).to_owned()),
            short_name.map_or_else(String::new, |n| delivery.text(n).to_owned()),
            match self.direction(trip.journey, document) {
                Some(Direction::Outbound) => "0".to_owned(),
                Some(Direction::Inbound) => "1".to_owned(),
                None => String::new(),
            },
            self.shape_of_trip
                .get(&number_u32)
                .map_or_else(String::new, |shape| format!("s{shape}")),
            wheelchair(facts.and_then(|f| f.wheelchair)).to_owned(),
        ];
        // A headway group states an interval, not departures: a frequency
        // without exact times, from the first departure to the last.
        for headway in record.headways {
            frequencies.push([
                trip.id.clone(),
                time_text(Some(headway.first_departure)),
                time_text(Some(headway.last_departure)),
                headway.interval.to_string(),
            ]);
        }
        row
    }

    /// The rows of `stop_times.txt` of the trip numbered `number`.
    fn stop_times(&mut self, number: usize, table: &mut Table<'_>) -> io::Result<()> {
        let trip = &self.trips[number];
        let record = self.index.journey(trip.journey);
        let document = self.delivery.name(record.document).unwrap_or(Name::MAX);
        let displays = self.displays(trip.journey, document);
        let calls = self.calls(trip.journey);
        let headsign = calls.first().and_then(|&(position, _)| displays[position]);
        for (sequence, &(position, stop_row)) in calls.iter().enumerate() {
            let time = record.passing_times[position];
            let call = record.uses[position];
            let stop_headsign = displays[position].filter(|&shown| Some(shown) != headsign);
            table.row([
                trip.id.as_str(),
                &time_text(time.arrival.or(time.departure)),
                &time_text(time.departure.or(time.arrival)),
                self.stop_rows[stop_row as usize].id.as_str(),
                &sequence.to_string(),
                if call.boarding { "0" } else { "1" },
                if call.alighting { "0" } else { "1" },
                stop_headsign.map_or("", |shown| self.delivery.text(shown)),
            ])?;
        }
        Ok(())
    }

    /// The destination display shown at each call of a journey, by position
    /// among its calls: what the point of its pattern or its call names, else
    /// the one before — a display holds from where it is named on —, else the
    /// pattern's.
    fn displays(&self, journey: JourneyId, document: Name) -> Vec<Option<Name>> {
        let record = self.index.journey(journey);
        let delivery = self.delivery;
        let facts = delivery
            .name(record.id)
            .and_then(|id| delivery.journey_facts(document, id));
        let pattern = record
            .pattern
            .and_then(|pattern| delivery.name(pattern))
            .and_then(|pattern| delivery.pattern_facts(document, pattern));
        let points = match (facts, pattern) {
            (Some(facts), _) if facts.calls.0 < facts.calls.1 => delivery.points(facts.calls),
            (_, Some(pattern)) => delivery.points(pattern.points),
            _ => &[],
        };
        let fallback = pattern
            .and_then(|pattern| pattern.display)
            .and_then(|display| delivery.display(document, display))
            .map(|display| display.text);
        // The points the vehicle stops at, in order: the calls are among them,
        // in their order.
        let stops: Vec<_> = points.iter().filter(|point| !point.passes).collect();
        let mut shown: Option<Name> = None;
        let mut next = 0;
        let mut displays = Vec::with_capacity(record.stop_points.len());
        for &stop_point in record.stop_points {
            let id = delivery.name(self.index.stop_point(stop_point).id);
            while let Some(point) = stops.get(next) {
                next += 1;
                if let Some(display) = point
                    .display
                    .and_then(|display| delivery.display(document, display))
                {
                    shown = Some(display.text);
                }
                if Some(point.stop_point) == id {
                    break;
                }
            }
            displays.push(shown.or(fallback));
        }
        displays
    }

    /// Which way a journey runs: as it says, else as its pattern says, else
    /// as the direction either names.
    fn direction(&self, journey: JourneyId, document: Name) -> Option<Direction> {
        let record = self.index.journey(journey);
        let delivery = self.delivery;
        let facts = delivery
            .name(record.id)
            .and_then(|id| delivery.journey_facts(document, id));
        let pattern = record
            .pattern
            .and_then(|pattern| delivery.name(pattern))
            .and_then(|pattern| delivery.pattern_facts(document, pattern));
        facts
            .and_then(|f| f.direction)
            .or_else(|| facts.and_then(|f| delivery.direction(document, f.direction_ref?)))
            .or_else(|| pattern.and_then(|p| p.direction))
            .or_else(|| pattern.and_then(|p| delivery.direction(document, p.direction_ref?)))
    }

    /// `calendar.txt` and `calendar_dates.txt`: for each calendar, the
    /// weekdays it runs on on most of their dates from its first day to its
    /// last, and each date it differs from them on.
    fn calendars(&mut self, output: &mut dyn Output) -> io::Result<(u64, u64)> {
        let mut calendar_rows: Vec<[String; 10]> = Vec::new();
        let mut date_rows: Vec<[String; 3]> = Vec::new();
        for &calendar in &self.calendars_used {
            let days: Vec<i64> = self.index.calendar_days(calendar).collect();
            let (first, last) = (days[0], days[days.len() - 1]);
            let mut active = [0u32; 7];
            let mut total = [0u32; 7];
            for day in first..=last {
                total[weekday(day)] += 1;
            }
            for &day in &days {
                active[weekday(day)] += 1;
            }
            let weekdays: [bool; 7] =
                std::array::from_fn(|w| active[w] > 0 && active[w] * 2 >= total[w]);
            let service = service_id(calendar);
            if weekdays.iter().any(|&on| on) {
                let mut row: [String; 10] = Default::default();
                row[0] = service.clone();
                for (w, &on) in weekdays.iter().enumerate() {
                    row[1 + w] = if on { "1" } else { "0" }.to_owned();
                }
                row[8] = date_text(first);
                row[9] = date_text(last);
                calendar_rows.push(row);
            }
            let runs: FxHashSet<i64> = days.iter().copied().collect();
            for day in first..=last {
                let regular = weekdays[weekday(day)];
                let runs_on = runs.contains(&day);
                if runs_on != regular {
                    let kind = if runs_on { "1" } else { "2" };
                    date_rows.push([service.clone(), date_text(day), kind.to_owned()]);
                }
            }
        }
        let mut table = Table::start(
            output,
            "calendar.txt",
            &[
                "service_id",
                "monday",
                "tuesday",
                "wednesday",
                "thursday",
                "friday",
                "saturday",
                "sunday",
                "start_date",
                "end_date",
            ],
        )?;
        for row in &calendar_rows {
            table.row(row.iter().map(String::as_str))?;
        }
        let calendars = table.rows();
        let mut table = Table::start(
            output,
            "calendar_dates.txt",
            &["service_id", "date", "exception_type"],
        )?;
        for row in &date_rows {
            table.row(row.iter().map(String::as_str))?;
        }
        Ok((calendars, table.rows()))
    }

    /// `transfers.txt`: the walks between stops (`SiteConnection`), at least
    /// as long as they take; and the planned interchanges between trips —
    /// staying seated (4), guaranteed (1), else recommended (0).
    fn transfers(&mut self, output: &mut dyn Output) -> io::Result<u64> {
        let mut table = Table::start(
            output,
            "transfers.txt",
            &[
                "from_stop_id",
                "to_stop_id",
                "from_trip_id",
                "to_trip_id",
                "transfer_type",
                "min_transfer_time",
            ],
        )?;
        let row_of_stop = |stop: StopId| -> Option<&str> {
            let row = self
                .place_rows
                .get(&Place::Stop(stop))
                .or_else(|| self.station_rows.get(&stop))?;
            Some(self.stop_rows[*row as usize].id.as_str())
        };
        // GTFS keys a walk by its two stops: of walks the delivery states
        // between the same two, the longest — none is made shorter.
        let mut walks: Vec<(&str, &str, u32)> = Vec::new();
        let mut walk_of: FxHashMap<(&str, &str), usize> = FxHashMap::default();
        let mut again: Vec<String> = Vec::new();
        for stop in self.index.stops() {
            for transfer in self.index.transfers_from(stop) {
                let (Some(from), Some(to)) = (row_of_stop(transfer.from), row_of_stop(transfer.to))
                else {
                    continue;
                };
                match walk_of.get(&(from, to)) {
                    Some(&at) => {
                        walks[at].2 = walks[at].2.max(transfer.seconds);
                        again.push(format!("{from} to {to}"));
                    }
                    None => {
                        walk_of.insert((from, to), walks.len());
                        walks.push((from, to, transfer.seconds));
                    }
                }
            }
        }
        for (from, to, seconds) in walks {
            table.row([from, to, "", "", "2", &seconds.to_string()])?;
        }
        let row_of_point = |point: StopPointId| -> Option<&str> {
            let place = match self.index.stop_of_stop_point(point) {
                Some(stop) => Place::Stop(stop),
                None => Place::Point(point),
            };
            let row = self.place_rows.get(&place)?;
            Some(self.stop_rows[*row as usize].id.as_str())
        };
        // An interchange several documents state is one transfer: the
        // first the index lists, whatever kind the others say.
        let mut written: FxHashSet<(&str, &str, &str, &str)> = FxHashSet::default();
        let mut left_out: Vec<&str> = Vec::new();
        let mut repeated: Vec<&str> = Vec::new();
        for point in (0..self.index.stop_point_count() as u32).map(StopPointId) {
            for interchange in self.index.interchanges_at(point) {
                let trip = |journey: JourneyId| {
                    self.trip_of_journey
                        .get(&journey)
                        .map(|&trip| self.trips[trip as usize].id.as_str())
                };
                let ends = (
                    row_of_point(interchange.from_stop_point),
                    row_of_point(interchange.to_stop_point),
                    trip(interchange.from_journey),
                    trip(interchange.to_journey),
                );
                let (Some(from), Some(to), Some(from_trip), Some(to_trip)) = ends else {
                    left_out.push(self.index.journey(interchange.from_journey).id);
                    continue;
                };
                if !written.insert((from, to, from_trip, to_trip)) {
                    repeated.push(from_trip);
                    continue;
                }
                let kind = if interchange.stay_seated {
                    "4"
                } else if interchange.guaranteed {
                    "1"
                } else {
                    "0"
                };
                table.row([from, to, from_trip, to_trip, kind, ""])?;
            }
        }
        let rows = table.rows();
        for walk in again {
            self.decisions.take(
                "walk-stated-twice",
                "a walk between two stops stated again, with another duration or between stops the feed writes as one: one transfer, the longest",
                || walk,
            );
        }
        let left_out: Vec<String> = left_out.into_iter().map(str::to_owned).collect();
        let repeated: Vec<String> = repeated.into_iter().map(str::to_owned).collect();
        for journey in left_out {
            self.decisions.take(
                "interchange-left-out",
                "a planned interchange with a trip or stop the feed does not hold: left out",
                || journey,
            );
        }
        for journey in repeated {
            self.decisions.take(
                "interchange-repeated",
                "a planned interchange between the same trips at the same stops stated again: one transfer, the first",
                || journey,
            );
        }
        Ok(rows)
    }

    fn shapes(&mut self, output: &mut dyn Output) -> io::Result<u64> {
        let mut table = Table::start(
            output,
            "shapes.txt",
            &[
                "shape_id",
                "shape_pt_lat",
                "shape_pt_lon",
                "shape_pt_sequence",
            ],
        )?;
        for (shape, points) in self.shapes.iter().enumerate() {
            let id = format!("s{shape}");
            for (sequence, (latitude, longitude)) in points.iter().enumerate() {
                table.row([
                    id.as_str(),
                    &format!("{latitude:.7}"),
                    &format!("{longitude:.7}"),
                    &sequence.to_string(),
                ])?;
            }
        }
        Ok(table.rows())
    }

    /// `feed_info.txt` and `attributions.txt`: who publishes the feed — the
    /// publisher given, else the delivery's data source naming a web address
    /// —, the period and version; and the organisations whose data it is.
    fn feed_info(&mut self, output: &mut dyn Output) -> io::Result<Vec<(&'static str, u64)>> {
        let delivery = self.delivery;
        let mut sources: Vec<(&str, Option<&str>)> = delivery
            .gather
            .data_sources
            .iter()
            .map(|&(_, name, url)| (delivery.text(name), url.map(|u| delivery.text(u))))
            .collect();
        sources.sort_unstable();
        sources.dedup();
        let publisher = self.options.publisher.clone().or_else(|| {
            sources.iter().find_map(|&(name, url)| {
                Some(Publisher {
                    name: name.to_owned(),
                    url: url?.to_owned(),
                    language: None,
                })
            })
        });
        let mut written = Vec::new();
        let mut documents: Vec<(&str, Name)> = delivery
            .gather
            .documents
            .keys()
            .map(|&d| (delivery.text(d), d))
            .collect();
        documents.sort_unstable();
        match publisher {
            Some(publisher) => {
                let language = publisher.language.clone().or_else(|| {
                    documents
                        .iter()
                        .find_map(|&(_, d)| delivery.document_facts(d).language)
                        .map(|l| delivery.text(l).to_owned())
                });
                let version = documents
                    .iter()
                    .filter_map(|&(_, d)| delivery.document_facts(d).published)
                    .map(|p| delivery.text(p))
                    .max()
                    .unwrap_or("");
                let (start, end) = self.period().unwrap_or_default();
                let mut table = Table::start(
                    output,
                    "feed_info.txt",
                    &[
                        "feed_publisher_name",
                        "feed_publisher_url",
                        "feed_lang",
                        "feed_start_date",
                        "feed_end_date",
                        "feed_version",
                    ],
                )?;
                table.row([
                    publisher.name.as_str(),
                    publisher.url.as_str(),
                    language.as_deref().unwrap_or("mul"),
                    &start,
                    &end,
                    version,
                ])?;
                written.push(("feed_info.txt", table.rows()));
            }
            None => self.decisions.take(
                "feed-info-without-publisher",
                "neither a publisher given nor a data source with a web address in the delivery: no feed_info.txt",
                String::new,
            ),
        }
        let participants: Vec<&str> = {
            let mut list: Vec<&str> = documents
                .iter()
                .filter_map(|&(_, d)| delivery.document_facts(d).participant)
                .map(|p| delivery.text(p))
                .collect();
            list.sort_unstable();
            list.dedup();
            list
        };
        if !sources.is_empty() || !participants.is_empty() {
            let mut table = Table::start(
                output,
                "attributions.txt",
                &[
                    "attribution_id",
                    "organization_name",
                    "is_producer",
                    "attribution_url",
                ],
            )?;
            let mut number = 0;
            for &(name, url) in &sources {
                number += 1;
                table.row([&format!("a{number}"), name, "1", url.unwrap_or("")])?;
            }
            for participant in participants {
                if sources.iter().any(|&(name, _)| name == participant) {
                    continue;
                }
                number += 1;
                table.row([&format!("a{number}"), participant, "1", ""])?;
            }
            written.push(("attributions.txt", table.rows()));
        }
        Ok(written)
    }

    /// The first and last day the feed's trips run on, `YYYYMMDD`.
    fn period(&self) -> Option<(String, String)> {
        let mut first = i64::MAX;
        let mut last = i64::MIN;
        for &calendar in &self.calendars_used {
            let mut days = self.index.calendar_days(calendar);
            if let Some(day) = days.next() {
                first = first.min(day);
                last = last.max(days.last().unwrap_or(day));
            }
        }
        (first <= last).then(|| (date_text(first), date_text(last)))
    }

    fn report(self, rows: BTreeMap<String, u64>) -> Report {
        let delivery = self.delivery;
        let mut index_findings = BTreeMap::new();
        for &concern in Concern::ALL {
            let count = self.index.finding_count(concern);
            if count > 0 {
                index_findings.insert(concern.code().to_owned(), count);
            }
        }
        let mut not_carried: Vec<NotCarried> = delivery
            .gather
            .not_carried
            .iter()
            .map(|(&element, entry)| NotCarried {
                element: delivery.text(element).to_owned(),
                count: entry.count,
                first_document: delivery.text(entry.first_document).to_owned(),
            })
            .collect();
        not_carried.sort_by(|a, b| a.element.cmp(&b.element));
        Report {
            profile: self.profile,
            documents: self.index.document_count(),
            rows,
            period: self.period(),
            index_findings,
            decisions: self.decisions.into_list(),
            not_carried,
        }
    }
}

/// Whether the times a journey states at the calls at `positions` go back
/// from one to the next.
fn goes_back(times: &[PassingTime], positions: impl Iterator<Item = usize>) -> bool {
    let mut last = 0u32;
    for position in positions {
        let time = times[position];
        for at in [time.arrival, time.departure].into_iter().flatten() {
            if at < last {
                return true;
            }
            last = at;
        }
    }
    false
}

/// The agency of lines naming none, from the publisher.
const PUBLISHER_AGENCY: &str = "publisher";

/// Identifiers made unique: the second of one text gets `#2`, the third `#3`.
#[derive(Debug, Default)]
struct UniqueIds {
    seen: FxHashMap<String, u32>,
}

impl UniqueIds {
    fn unique(&mut self, id: &str) -> String {
        let count = self.seen.entry(id.to_owned()).or_insert(0);
        *count += 1;
        if *count == 1 {
            id.to_owned()
        } else {
            let unique = format!("{id}#{count}");
            self.seen.insert(unique.clone(), 1);
            unique
        }
    }
}

impl Profile {
    /// The time zone of the profile's country, where it has one.
    fn time_zone(self) -> Option<&'static str> {
        match self {
            Profile::Delfi => Some("Europe/Berlin"),
            Profile::Austria => Some("Europe/Vienna"),
            Profile::Switzerland => Some("Europe/Zurich"),
            Profile::Epip | Profile::Other => None,
        }
    }
}

/// The profile a delivery's identifiers show: the prefix most of its
/// journeys' identifiers carry.
fn profile(index: &Index, epip: bool) -> Profile {
    if epip {
        return Profile::Epip;
    }
    let mut counts = [0u32; 3];
    for journey in (0..index.journey_count() as u32).map(JourneyId) {
        let id = index.journey(journey).id;
        let slot = if id.starts_with("DE::") {
            0
        } else if id.starts_with("at:") {
            1
        } else if id.starts_with("ch:") {
            2
        } else {
            continue;
        };
        counts[slot] += 1;
    }
    let most = counts.iter().copied().max().unwrap_or(0);
    if most * 2 <= index.journey_count() as u32 || most == 0 {
        return Profile::Other;
    }
    match counts.iter().position(|&count| count == most) {
        Some(0) => Profile::Delfi,
        Some(1) => Profile::Austria,
        _ => Profile::Switzerland,
    }
}

/// The GTFS route type of a NeTEx mode; none for a mode GTFS has none for.
fn route_type(mode: Option<&str>) -> Option<&'static str> {
    Some(match mode? {
        "tram" => "0",
        "metro" => "1",
        "rail" => "2",
        "bus" | "coach" | "trolleyBus" => "3",
        "water" | "ferry" => "4",
        "cableway" | "telecabin" => "6",
        "funicular" => "7",
        "air" => "1100",
        "taxi" => "1500",
        _ => return None,
    })
}

fn wheelchair(value: Option<Wheelchair>) -> &'static str {
    match value {
        Some(Wheelchair::Accessible) => "1",
        Some(Wheelchair::Not) => "2",
        None => "0",
    }
}

fn service_id(calendar: CalendarId) -> String {
    format!("c{}", calendar.0)
}

/// A time of day in seconds since midnight as GTFS writes it, `HH:MM:SS`,
/// past 24 hours where it is.
fn time_text(seconds: Option<u32>) -> String {
    match seconds {
        Some(s) => format!("{:02}:{:02}:{:02}", s / 3600, s / 60 % 60, s % 60),
        None => String::new(),
    }
}

/// The weekday of a day since 1970-01-01, Monday 0 (1970-01-01 was a Thursday).
fn weekday(day: i64) -> usize {
    (day + 3).rem_euclid(7) as usize
}

/// A day since 1970-01-01 as GTFS writes a date, `YYYYMMDD`.
fn date_text(day: i64) -> String {
    Date::from_days_since_epoch(day).as_str().replace('-', "")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn days_are_written_as_dates_and_weekdays() {
        assert_eq!(date_text(0), "19700101");
        assert_eq!(weekday(0), 3, "a Thursday");
        let day = Date::from("2026-10-12").days_since_epoch().expect("a date");
        assert_eq!(date_text(day), "20261012");
        assert_eq!(weekday(day), 0, "a Monday");
        assert_eq!(date_text(day - 365 * 30 - 7), "19961012");
        assert_eq!(date_text(-1), "19691231");
    }

    #[test]
    fn times_past_midnight_stay_past_24_hours() {
        assert_eq!(time_text(Some(0)), "00:00:00");
        assert_eq!(time_text(Some(86_400 + 30 * 60 + 5)), "24:30:05");
        assert_eq!(time_text(None), "");
    }

    #[test]
    fn a_second_identifier_of_one_text_is_numbered() {
        let mut ids = UniqueIds::default();
        assert_eq!(ids.unique("a"), "a");
        assert_eq!(ids.unique("a"), "a#2");
        assert_eq!(ids.unique("a#2"), "a#2#2");
        assert_eq!(ids.unique("b"), "b");
    }
}
