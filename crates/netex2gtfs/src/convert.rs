//! The conversion: a delivery's index and gathered facts written as the files
//! of a GTFS feed, every decision counted in the report.

use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::fmt;
use std::io;

use jiff::tz::TimeZone;
use netex_rs::crs;
use netex_rs::geometry::{reading_order, AxisOrder};
use netex_rs::index::{
    CalendarId, Concern, Headway, Index, JourneyId, LineId, PassingTime, StopId, StopKind,
    StopPointId,
};
use netex_rs::model::{Date, DateTime};
use rustc_hash::{FxHashMap, FxHashSet};

use crate::country;
use crate::delivery::{Agency, Delivery};
use crate::gather::{Direction, LinkGeometry, Wheelchair};
use crate::names::Name;
use crate::output::{Output, Table};
use crate::report::{Decisions, NotCarried, Profile, Report};

/// Which identifiers a feed's stops get.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
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
///
/// Options are made from their default, a field set at a time, so that a
/// later option is no breaking change.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct Options {
    /// Which identifiers the stops get.
    pub stop_ids: StopIds,
    /// Who publishes the feed. Without one, the delivery's data source with
    /// a web address does, where it names one.
    pub publisher: Option<Publisher>,
    /// The time zone of the feed — of all its agencies, as GTFS has one for
    /// them — where no document of the delivery names one by its name in
    /// the time zone database (`Europe/Zurich`); a name the database does
    /// not hold fails the conversion ([`Error::UnknownTimeZone`]).
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
    /// No document names a time zone by its name in the time zone database,
    /// the options give none, and neither the delivery's identifiers nor the
    /// positions of its stops show its country: GTFS needs one for its
    /// agencies.
    NoTimeZone,
    /// The time zone given for the feed, or the one of the country it takes,
    /// is no name the time zone database holds.
    UnknownTimeZone(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Write(error) => write!(f, "writing the feed: {error}"),
            Error::NoTimeZone => f.write_str(
                "no document of the delivery names a time zone by its name in the time zone database (FrameDefaults/DefaultLocale/TimeZone), nor do its identifiers or the positions of its stops show its country; give one",
            ),
            Error::UnknownTimeZone(zone) => {
                write!(f, "`{zone}` is no time zone of the time zone database")
            }
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Write(error) => Some(error),
            Error::NoTimeZone | Error::UnknownTimeZone(_) => None,
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
    /// The row of its station.
    parent: Option<u32>,
    wheelchair: Option<Wheelchair>,
    platform: Option<String>,
    level: Option<String>,
}

/// A trip as planned: the journey, its identifier, its route, the days it
/// runs on, and how much later its times are written than the journey
/// states them; its calls are found again as they are written
/// ([`Conversion::calls`]).
#[derive(Debug)]
struct TripPlan {
    journey: JourneyId,
    id: String,
    route: u32,
    service: Service,
    /// Seconds added to every time of the journey: the difference of its
    /// agency's time zone from the feed's ([`Conversion::plan_in_zone`]).
    shift: i64,
    /// Its place among the trips of its journey: 0 for the first or only.
    part: u32,
}

/// The days a trip runs on: its journey's calendar, or days of its own —
/// for a trip of an agency in another time zone than the feed's, on the day
/// before or on some of its days only ([`Conversion::plan_in_zone`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Service {
    Calendar(CalendarId),
    /// A service of [`Conversion::own_services`], by its number.
    Own(u32),
}

/// A service of days of its own: its identifier and its days, in order.
#[derive(Debug)]
struct OwnService {
    id: String,
    days: Vec<i64>,
}

struct Conversion<'d> {
    delivery: &'d Delivery,
    index: &'d Index,
    options: &'d Options,
    decisions: Decisions,
    profile: Profile,
    /// The time zone of the feed, of all its agencies.
    feed_zone: Zone,
    /// The data sources the delivery names, each by its name and its web
    /// address, where it states one with its scheme ([`data_sources`]).
    sources: Vec<(&'d str, Option<&'d str>)>,
    /// Who publishes the feed: the publisher given, else the delivery's
    /// data source with a web address.
    publisher: Option<Publisher>,
    /// The feed stop of each place calls are made at, by its row.
    place_rows: FxHashMap<Place, u32>,
    /// The station row of each stop place that is a parent.
    station_rows: FxHashMap<StopId, u32>,
    stop_rows: Vec<StopRow>,
    /// The routes: the line each is, its identifier, its agency, the
    /// document of the journey the line was first met with.
    routes: Vec<(LineId, String, Option<u32>, Name)>,
    route_of_line: FxHashMap<LineId, u32>,
    /// The route of each definition of a line: its identifier, agency, name,
    /// number and mode.
    route_of_definition: FxHashMap<RouteKey<'d>, u32>,
    route_ids: UniqueIds,
    /// The agencies: the organisation each is, or the publisher.
    agencies: Vec<AgencyRow>,
    /// The agency row of each organisation, by its identifier.
    agency_of_organisation: FxHashMap<&'d str, u32>,
    /// The agency row of the publisher, once a line naming none needs it.
    publisher_agency: Option<u32>,
    trips: Vec<TripPlan>,
    /// The trips of each journey: the first's number and how many, one
    /// after the other.
    trips_of_journey: FxHashMap<JourneyId, (u32, u32)>,
    calendars_used: Vec<CalendarId>,
    own_services: Vec<OwnService>,
    /// The service of each calendar's days of its own.
    own_service_of: FxHashMap<(CalendarId, Vec<i64>), u32>,
    shape_of_trip: FxHashMap<u32, u32>,
    shapes: Vec<Vec<(f64, f64)>>,
    levels_used: Vec<Name>,
    /// The sum of the positions of the quays of each stop place, and how many.
    quay_positions: FxHashMap<StopId, (f64, f64, u32)>,
    /// Each stop's place in the order of the stops' identifiers, by its
    /// number in the index: the order rows of stops are written in.
    stop_rank: Vec<u32>,
    /// The parent sites that are no station of the feed, having no position.
    unplaced_sites: FxHashSet<StopId>,
    /// The time zone of each stop row in another than the feed's: a station,
    /// or a stop standing under none.
    stop_zones: FxHashMap<u32, String>,
}

/// What makes a definition of a line a route of its own: its identifier,
/// agency, name, number and mode.
type RouteKey<'d> = (
    &'d str,
    Option<u32>,
    &'d str,
    Option<&'d str>,
    Option<&'d str>,
);

#[derive(Debug, Clone)]
struct AgencyRow {
    id: String,
    name: String,
    url: String,
    phone: Option<String>,
    /// The time zone its document names, where it is another than the
    /// feed's.
    zone: Option<Zone>,
}

/// A time zone the time zone database holds: its name, as GTFS writes it,
/// and its rules.
#[derive(Debug, Clone)]
struct Zone {
    name: String,
    rules: TimeZone,
}

/// What `stop_times.txt` found of a trip that `trips.txt` and
/// `frequencies.txt` write: the display at its first call, and how long
/// after the journey's first call its first call written is.
#[derive(Debug, Clone, Copy)]
struct TripStart {
    headsign: Option<Name>,
    delay: u32,
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
        let mut named = None;
        for &(document_name, document) in &documents {
            let Some(zone) = delivery.document_facts(document).time_zone else {
                continue;
            };
            let zone = delivery.text(zone).trim();
            match time_zone_named(zone) {
                Some(rules) => {
                    named.get_or_insert(rules);
                }
                None => decisions.take(
                    "time-zone-not-a-name",
                    "a document naming its time zone otherwise than by a name the time zone database holds (`CET`, `CEST`, `BST`): taken as naming none, as GTFS requires such a name",
                    || format!("{document_name}: {zone}"),
                ),
            }
        }
        let feed_zone = match (named, &options.time_zone) {
            (Some(rules), _) => rules,
            (None, Some(given)) => {
                time_zone_named(given).ok_or_else(|| Error::UnknownTimeZone(given.clone()))?
            }
            (None, None) => {
                let zone = time_zone_of_country(index, &mut decisions)?;
                time_zone_named(zone).ok_or_else(|| Error::UnknownTimeZone(zone.to_owned()))?
            }
        };
        note_positions_off_the_earth(index, &mut decisions);
        let sources = data_sources(delivery, &mut decisions);
        let publisher = options.publisher.clone().or_else(|| {
            sources.iter().find_map(|&(name, url)| {
                Some(Publisher {
                    name: name.to_owned(),
                    url: url?.to_owned(),
                    language: None,
                })
            })
        });
        Ok(Conversion {
            delivery,
            index,
            options,
            decisions,
            profile,
            feed_zone,
            sources,
            publisher,
            place_rows: FxHashMap::default(),
            station_rows: FxHashMap::default(),
            stop_rows: Vec::new(),
            routes: Vec::new(),
            route_of_line: FxHashMap::default(),
            route_of_definition: FxHashMap::default(),
            route_ids: UniqueIds::default(),
            agencies: Vec::new(),
            agency_of_organisation: FxHashMap::default(),
            publisher_agency: None,
            trips: Vec::new(),
            trips_of_journey: FxHashMap::default(),
            calendars_used: Vec::new(),
            own_services: Vec::new(),
            own_service_of: FxHashMap::default(),
            shape_of_trip: FxHashMap::default(),
            shapes: Vec::new(),
            levels_used: Vec::new(),
            quay_positions: FxHashMap::default(),
            stop_rank: stop_ranks(index),
            unplaced_sites: FxHashSet::default(),
            stop_zones: FxHashMap::default(),
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
                let (code, what) = TRIP_WITHOUT_TWO_CALLS;
                self.decisions.take(code, what, || record.id.to_owned());
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
            let record = self.index.journey(journey);
            let placed = self.placed_calls(journey);
            if placed.len() < 2 {
                self.decisions.take(
                    "trip-at-unplaced-stops",
                    "a journey left with fewer than two calls once those at stops the feed cannot place are left out: left out",
                    || record.id.to_owned(),
                );
                continue;
            }
            let placed_count = placed.len();
            let calls = timed_ends(placed, record.passing_times);
            if calls.len() < placed_count {
                self.decisions.take(
                    "trip-untimed-end-cut",
                    "a journey whose first or last call at a placed stop has no time — the calls before or after it left out —: cut to the first and last with a time, as GTFS requires times there",
                    || record.id.to_owned(),
                );
            }
            if calls.len() < 2 {
                let (code, what) = TRIP_WITHOUT_TWO_CALLS;
                self.decisions.take(code, what, || record.id.to_owned());
                continue;
            }
            let route = self.route_of(line, journey);
            let first = trip_number(self.trips.len());
            let zone = self.routes[route as usize]
                .2
                .and_then(|agency| self.agencies[agency as usize].zone.clone());
            match zone {
                Some(zone) => self.plan_in_zone(journey, route, &zone, &calls),
                None => {
                    let calendar = self.index.calendar_of(journey);
                    self.calendars_used.push(calendar);
                    self.trips.push(TripPlan {
                        journey,
                        id: String::new(),
                        route,
                        service: Service::Calendar(calendar),
                        shift: 0,
                        part: 0,
                    });
                }
            }
            let count = trip_number(self.trips.len()) - first;
            self.trips_of_journey.insert(journey, (first, count));
        }
        self.calendars_used.sort_unstable();
        self.calendars_used.dedup();
        self.name_trips();
        self.plan_stop_zones();
        if self.options.shapes {
            self.plan_shapes();
        }
    }

    /// The trips of a journey of an agency in another time zone than the
    /// feed's: each runs at the instants the journey's times state in the
    /// agency's zone, written in the feed's — as much later as noon of its
    /// day is later in the agency's zone than in the feed's, a GTFS time
    /// counting from twelve hours before noon —, on the day before where it
    /// would begin before its day. Where the difference changes over the
    /// journey's days — the zones keeping summer time on other days —, the
    /// journey is a trip for each set of days of one difference, `<id>#<n>`
    /// ([`Conversion::name_trips`]), each with days of its own.
    fn plan_in_zone(
        &mut self,
        journey: JourneyId,
        route: u32,
        zone: &Zone,
        calls: &[(usize, u32)],
    ) {
        let record = self.index.journey(journey);
        let calendar = self.index.calendar_of(journey);
        let earliest = earliest_time(record.passing_times, record.headways, calls);
        // The days, in the feed, of each way of writing the journey: how many
        // days earlier, and how many seconds later; in the order of the
        // first day of each.
        let mut ways: Vec<((i64, i64), Vec<i64>)> = Vec::new();
        let mut beyond = false;
        for day in self.index.calendar_days(calendar) {
            let Some((moved, shift)) =
                in_feed_zone(&zone.rules, &self.feed_zone.rules, day, earliest)
            else {
                beyond = true;
                continue;
            };
            match ways.iter_mut().find(|(way, _)| *way == (moved, shift)) {
                Some((_, days)) => days.push(day + moved),
                None => ways.push(((moved, shift), vec![day + moved])),
            }
        }
        if beyond {
            self.decisions.take(
                "trip-days-beyond-zone-rules",
                "days of a journey of an agency in another time zone than the feed's beyond the last the time zone rules reach (9999-12-30): left out of its trip",
                || record.id.to_owned(),
            );
        }
        if ways.iter().any(|&(way, _)| way != (0, 0)) {
            self.decisions.take(
                "trip-times-in-feed-zone",
                "a journey of an agency in another time zone than the feed's: its times written in the feed's — as much later or earlier as the zones' offsets differ on its days, on the day before where it would begin before its day —, the instants kept",
                || record.id.to_owned(),
            );
        }
        if ways.len() > 1 {
            self.decisions.take(
                "trip-split-by-offset",
                "a journey of an agency whose time zone's offset from the feed's changes over its days — the zones keeping summer time on other days —: a trip for each set of days of one difference, `<id>#<n>`, each with days of its own",
                || record.id.to_owned(),
            );
        }
        // One way, on the journey's days: its calendar's days.
        let whole = !beyond && ways.len() == 1 && ways[0].0 .0 == 0;
        for (part, ((_, shift), days)) in ways.into_iter().enumerate() {
            let service = if whole {
                self.calendars_used.push(calendar);
                Service::Calendar(calendar)
            } else {
                self.own_service(calendar, days)
            };
            self.trips.push(TripPlan {
                journey,
                id: String::new(),
                route,
                service,
                shift,
                part: trip_number(part),
            });
        }
    }

    /// The service of `days` of a trip of `calendar` running on days of its
    /// own, `c<calendar>-<n>`; one for all trips of those days.
    fn own_service(&mut self, calendar: CalendarId, days: Vec<i64>) -> Service {
        let key = (calendar, days);
        if let Some(&number) = self.own_service_of.get(&key) {
            return Service::Own(number);
        }
        let of_calendar = self
            .own_service_of
            .keys()
            .filter(|(of, _)| *of == calendar)
            .count();
        let number = u32::try_from(self.own_services.len()).expect("fewer than 2^32 services");
        self.own_services.push(OwnService {
            id: format!("{}-{}", service_id(calendar), of_calendar + 1),
            days: key.1.clone(),
        });
        self.own_service_of.insert(key, number);
        Service::Own(number)
    }

    /// The identifier of a service, as `trips.txt` and `calendar.txt` name it.
    fn service_name(&self, service: Service) -> String {
        match service {
            Service::Calendar(calendar) => service_id(calendar),
            Service::Own(number) => self.own_services[number as usize].id.clone(),
        }
    }

    /// The trips of a journey, one for each set of its days written alike.
    fn trips_of(&self, journey: JourneyId) -> &[TripPlan] {
        self.trips_of_journey
            .get(&journey)
            .map_or(&[], |&(first, count)| {
                &self.trips[first as usize..(first + count) as usize]
            })
    }

    /// The time zone of each stop the trips of an agency in another zone
    /// than the feed's call at — of its station, whose zone a stop under one
    /// takes (GTFS) —, where every trip calling there is of an agency in that
    /// zone; a stop called at by trips in different zones is in the feed's,
    /// the decision taken.
    fn plan_stop_zones(&mut self) {
        if self.agencies.iter().all(|agency| agency.zone.is_none()) {
            return;
        }
        // The zone of each stop row the trips call at, none for the feed's;
        // a row called at in different zones, `Err`.
        let mut zones: FxHashMap<u32, Result<Option<&str>, ()>> = FxHashMap::default();
        for trip in self.trips.iter().filter(|trip| trip.part == 0) {
            let zone = self.routes[trip.route as usize]
                .2
                .and_then(|agency| self.agencies[agency as usize].zone.as_ref())
                .map(|zone| zone.name.as_str());
            for (_, row) in self.calls(trip.journey) {
                let holder = self.stop_rows[row as usize].parent.unwrap_or(row);
                let known = zones.entry(holder).or_insert(Ok(zone));
                if *known != Ok(zone) {
                    *known = Err(());
                }
            }
        }
        let mut zones: Vec<(u32, Result<Option<String>, ()>)> = zones
            .into_iter()
            .map(|(row, zone)| (row, zone.map(|zone| zone.map(str::to_owned))))
            .collect();
        zones.sort_unstable_by_key(|&(row, _)| row);
        for (row, zone) in zones {
            match zone {
                Ok(Some(zone)) => {
                    self.stop_zones.insert(row, zone);
                }
                Ok(None) => {}
                Err(()) => self.decisions.take(
                    "stop-time-zone-ambiguous",
                    "a stop, or the station it stands under, called at by trips of agencies in different time zones: in the feed's",
                    || self.stop_rows[row as usize].id.clone(),
                ),
            }
        }
    }

    /// Where a call at a stop point takes place: at its stop, else — for a
    /// stop point placed at none — at its own position, if it has one.
    fn place_of(&self, point: StopPointId) -> Option<Place> {
        match self.index.stop_of_stop_point(point) {
            Some(stop) => Some(Place::Stop(stop)),
            None => {
                let record = self.index.stop_point(point);
                on_the_earth(record.latitude, record.longitude).map(|_| Place::Point(point))
            }
        }
    }

    /// The calls of a journey the feed writes: those at a stop of the feed,
    /// from the first with a time to the last ([`timed_ends`]).
    fn calls(&self, journey: JourneyId) -> Vec<(usize, u32)> {
        let times = self.index.journey(journey).passing_times;
        timed_ends(self.placed_calls(journey), times)
    }

    /// The calls of a journey at a stop of the feed: each by its position
    /// among the journey's calls, with the row of its stop.
    fn placed_calls(&self, journey: JourneyId) -> Vec<(usize, u32)> {
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
        for stop in self.stops_in_order() {
            let quay = index.stop(stop);
            if let (Some(parent), Some((latitude, longitude))) =
                (quay.parent, on_the_earth(quay.latitude, quay.longitude))
            {
                let sum = self.quay_positions.entry(parent).or_insert((0.0, 0.0, 0));
                *sum = (sum.0 + latitude, sum.1 + longitude, sum.2 + 1);
            }
        }
        self.plan_sites(used);
        let mut parents: Vec<StopId> = used
            .iter()
            .filter_map(|place| match place {
                Place::Stop(stop) => self.station_of(*stop),
                Place::Point(_) => None,
            })
            .collect();
        parents.sort_unstable_by_key(|stop| self.stop_rank[stop.0 as usize]);
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
        ordered.sort_unstable_by(|a, b| match (a, b) {
            (Place::Stop(a), Place::Stop(b)) => {
                self.stop_rank[a.0 as usize].cmp(&self.stop_rank[b.0 as usize])
            }
            (Place::Point(a), Place::Point(b)) => stop_point_order(index, *a, *b),
            (Place::Stop(_), Place::Point(_)) => Ordering::Less,
            (Place::Point(_), Place::Stop(_)) => Ordering::Greater,
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
                    let id = self.unique_stop_id(&mut ids, record.id);
                    let name = name_or_identifier(
                        &mut self.decisions,
                        record.name,
                        record.id,
                        STOP_NAMED_BY_ITS_IDENTIFIER,
                    );
                    Some(StopRow {
                        id,
                        name,
                        position: on_the_earth(record.latitude, record.longitude)
                            .expect("a stop point placed only with a position"),
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

    /// Which parent sites of the stops called at are no station, having no
    /// position of their own or of their quays: the stop places within one
    /// stand on their own, or within the site it is within.
    fn plan_sites(&mut self, used: &[Place]) {
        let index = self.index;
        let mut sites: Vec<StopId> = used
            .iter()
            .filter_map(|place| match *place {
                Place::Stop(stop) => {
                    let record = index.stop(stop);
                    match record.kind {
                        StopKind::Quay => record.parent,
                        StopKind::StopPlace => Some(stop),
                        // A kind of stop a later index tells apart stands under none.
                        _ => None,
                    }
                }
                Place::Point(_) => None,
            })
            .flat_map(|place| self.parent_sites(place))
            .collect();
        sites.sort_unstable_by_key(|site| self.stop_rank[site.0 as usize]);
        sites.dedup();
        for site in sites {
            let record = index.stop(site);
            let placed = on_the_earth(record.latitude, record.longitude).is_some()
                || self.quay_positions.contains_key(&site);
            if placed {
                continue;
            }
            self.unplaced_sites.insert(site);
            self.decisions.take(
                "parent-site-without-position",
                "a parent site without a position of its own or of its quays: no station, the stop places within it stand on their own",
                || record.id.to_owned(),
            );
        }
    }

    /// The stops of the index in the order of their identifiers.
    fn stops_in_order(&self) -> Vec<StopId> {
        let mut stops: Vec<StopId> = self.index.stops().collect();
        stops.sort_unstable_by_key(|stop| self.stop_rank[stop.0 as usize]);
        stops
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
            // A kind of stop a later index tells apart stands under none.
            _ => None,
        }
    }

    /// The outermost parent site of a stop place that is a station of the
    /// feed, if it is within one.
    fn parent_site_of(&self, place: StopId) -> Option<StopId> {
        self.parent_sites(place)
            .into_iter()
            .rev()
            .find(|site| !self.unplaced_sites.contains(site))
    }

    /// The parent sites the delivery defines that a stop place is within, a
    /// parent site within a parent site, from the innermost out; where the
    /// delivery states a cycle, they end where it closes.
    fn parent_sites(&self, place: StopId) -> Vec<StopId> {
        let mut sites = Vec::new();
        let mut current = place;
        while let Some(parent) = self
            .delivery
            .name(self.index.stop(current).id)
            .and_then(|id| self.delivery.gather.stops.get(&id))
            .and_then(|facts| facts.parent_site)
            .and_then(|parent| self.index.stop_by_id(self.delivery.text(parent)))
            .filter(|parent| *parent != place && !sites.contains(parent))
        {
            sites.push(parent);
            current = parent;
        }
        sites
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
        let own = on_the_earth(record.latitude, record.longitude);
        let position = own.or_else(|| match record.kind {
            StopKind::Quay => {
                let place = index.stop(record.parent?);
                let at = on_the_earth(place.latitude, place.longitude)?;
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
            // A kind of stop a later index tells apart takes no other's position.
            _ => None,
        })?;
        let station_name = record
            .parent
            .map(|p| index.stop(p).name)
            .filter(|name| !name.trim().is_empty());
        let name = match station_name {
            Some(station_name) if record.name.trim().is_empty() => {
                self.decisions.take(
                    "stop-named-by-its-station",
                    "a stop without a name: named as its stop place",
                    || record.id.to_owned(),
                );
                station_name.to_owned()
            }
            _ => name_or_identifier(
                &mut self.decisions,
                record.name,
                record.id,
                STOP_NAMED_BY_ITS_IDENTIFIER,
            ),
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
        let id = self.unique_stop_id(ids, &base);
        Some(StopRow {
            id,
            name,
            position,
            location_type,
            parent,
            wheelchair: facts.wheelchair,
            platform: facts.platform.map(|p| self.delivery.text(p).to_owned()),
            level: level.map(|l| self.delivery.text(l).to_owned()),
        })
    }

    /// A stop's identifier, made unique among the feed's stops.
    fn unique_stop_id(&mut self, ids: &mut UniqueIds, base: &str) -> String {
        let id = ids.unique(base);
        if id != base {
            self.decisions.take(
                "stop-made-unique",
                "an identifier several stops of the feed would carry — a global identifier, or one another stop's number or `#place` took —: the second and each after it numbered, `<id>#<n>`",
                || base.to_owned(),
            );
        }
        id
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
        let key = (
            record.id,
            agency,
            record.name,
            record.public_code,
            record.mode,
        );
        let route = match self.route_of_definition.get(&key) {
            Some(&route) => route,
            None => {
                let id = self.route_ids.unique(record.id);
                if id != record.id {
                    self.decisions.take(
                        "route-made-unique",
                        "a line defined again with another name, number, mode or agency, or one whose identifier another route took: a route of its own, `<id>#<n>`",
                        || record.id.to_owned(),
                    );
                }
                self.routes.push((line, id, agency, document));
                let route = u32::try_from(self.routes.len() - 1).expect("fewer than 2^32 routes");
                self.route_of_definition.insert(key, route);
                route
            }
        };
        self.route_of_line.insert(line, route);
        route
    }

    /// The agency row of an organisation; the publisher's for a line naming
    /// none.
    fn agency_row(&mut self, agency: Option<Agency<'d>>, line: &str) -> Option<u32> {
        let row = match agency {
            Some(agency) => {
                if let Some(&at) = self.agency_of_organisation.get(agency.id) {
                    return Some(at);
                }
                let url = web_address(&mut self.decisions, agency.url, agency.id);
                let url = match url {
                    Some(url) => url.to_owned(),
                    None => {
                        self.decisions.take(
                            "agency-without-url",
                            "an organisation stating no web address: the feed's publisher's is written — the publisher given, else the delivery's data source with one —, GTFS requires one; without a publisher, none",
                            || agency.id.to_owned(),
                        );
                        self.publisher
                            .as_ref()
                            .map_or_else(String::new, |publisher| publisher.url.clone())
                    }
                };
                let zone = self
                    .delivery
                    .document_facts(agency.document)
                    .time_zone
                    .and_then(|zone| time_zone_named(self.delivery.text(zone).trim()))
                    .filter(|zone| zone.name != self.feed_zone.name);
                if let Some(zone) = &zone {
                    self.decisions.take(
                        "agency-time-zone-unified",
                        "an organisation whose document names another time zone than the feed's: written in the feed's, as GTFS has one time zone for all agencies; its trips run at the instants their times state in its zone, written in the feed's, and its stops are in its zone (`stop_timezone`)",
                        || format!("{}: {}", agency.id, zone.name),
                    );
                }
                let name = name_or_identifier(
                    &mut self.decisions,
                    agency.name,
                    agency.id,
                    AGENCY_NAMED_BY_ITS_IDENTIFIER,
                );
                let at = u32::try_from(self.agencies.len()).expect("fewer than 2^32 agencies");
                self.agency_of_organisation.insert(agency.id, at);
                AgencyRow {
                    id: agency.id.to_owned(),
                    name,
                    url,
                    phone: agency.phone.map(str::to_owned),
                    zone,
                }
            }
            None => {
                self.decisions.take(
                    "route-without-agency",
                    "a line naming no operator or authority the delivery defines: the feed's publisher is its agency — the publisher given, else the delivery's data source with a web address —; without one, the route names none",
                    || line.to_owned(),
                );
                if self.publisher_agency.is_some() {
                    return self.publisher_agency;
                }
                let publisher = self.publisher.as_ref()?;
                let row = AgencyRow {
                    id: PUBLISHER_AGENCY.to_owned(),
                    name: publisher.name.clone(),
                    url: publisher.url.clone(),
                    phone: None,
                    zone: None,
                };
                self.publisher_agency =
                    Some(u32::try_from(self.agencies.len()).expect("fewer than 2^32 agencies"));
                row
            }
        };
        self.agencies.push(row);
        Some(u32::try_from(self.agencies.len() - 1).expect("fewer than 2^32 agencies"))
    }

    /// Each trip's identifier: its journey's, made unique where the index
    /// holds two journeys of one identifier — stated otherwise in two
    /// documents, or two runs of one template —, or a journey is several
    /// trips, split by its days.
    fn name_trips(&mut self) {
        let mut ids = UniqueIds::default();
        for trip in &mut self.trips {
            let id = self.index.journey(trip.journey).id;
            let unique = ids.unique(id);
            // The further trips of a journey split by its days are a decision
            // of their own ([`Conversion::plan_in_zone`]).
            if unique != id && trip.part == 0 {
                self.decisions.take(
                    "trip-made-unique",
                    "a journey identifier the index holds twice — stated otherwise in two documents —: the second trip `<id>#<n>`",
                    || id.to_owned(),
                );
            }
            trip.id = unique;
        }
    }

    /// The shapes: for each trip, the geometry of the link sequence
    /// projection its journey names, else of the service links it runs over
    /// ([`Conversion::links_of`]), one after the other, read in each one's
    /// reference system and axis order; trips over the same course share a
    /// shape. A trip with neither — no projection with a geometry, a pair of
    /// calls no link with a geometry joins — has none.
    fn plan_shapes(&mut self) {
        let gather = &self.delivery.gather;
        let mut without: Vec<(&str, &str)> = gather
            .links
            .iter()
            .filter(|link| !link.has_geometry())
            .map(|link| (gather.names.text(link.document), gather.names.text(link.id)))
            .collect();
        without.sort_unstable();
        for (_, link) in without {
            self.decisions.take(
                "link-without-geometry",
                "a service link stating no geometry, on itself or in a link sequence projection: no shape follows it",
                || link.to_owned(),
            );
        }
        let mut without: Vec<(&str, &str)> = gather
            .projections
            .iter()
            .filter(|projection| !projection.has_geometry())
            .map(|projection| {
                (
                    gather.names.text(projection.document),
                    gather.names.text(projection.id),
                )
            })
            .collect();
        without.sort_unstable();
        for (_, projection) in without {
            self.decisions.take(
                "projection-without-geometry",
                "a link sequence projection stating no geometry: a journey naming it takes the shape of its service links",
                || projection.to_owned(),
            );
        }
        // The shape of each course, none where it has fewer than two
        // positions: GTFS draws a shape between two at least.
        let mut shape_of_course: FxHashMap<Course, Option<u32>> = FxHashMap::default();
        for trip_number in 0..self.trips.len() {
            let journey = self.trips[trip_number].journey;
            let record = self.index.journey(journey);
            let Some(course) = self.course_of(journey) else {
                self.decisions.take(
                    "shape-incomplete",
                    "a trip with a pair of calls no service link with a geometry joins: no shape",
                    || record.id.to_owned(),
                );
                continue;
            };
            let shape = match shape_of_course.get(&course) {
                Some(&shape) => shape,
                None => {
                    let points = self.shape_points(journey, &course);
                    let shape = (points.len() >= 2).then(|| {
                        self.shapes.push(points);
                        u32::try_from(self.shapes.len() - 1).expect("fewer than 2^32 shapes")
                    });
                    shape_of_course.insert(course, shape);
                    shape
                }
            };
            let Some(shape) = shape else {
                self.decisions.take(
                    "shape-without-two-positions",
                    "a trip whose course gives fewer than two positions — its every stretch in a reference system the crate does not know, or one position —: no shape",
                    || record.id.to_owned(),
                );
                continue;
            };
            let trip_number = u32::try_from(trip_number).expect("fewer than 2^32 trips");
            self.shape_of_trip.insert(trip_number, shape);
        }
    }

    /// What a journey's shape is drawn from: the link sequence projection it
    /// names, where that states a geometry, else the service links it runs
    /// over.
    fn course_of(&self, journey: JourneyId) -> Option<Course> {
        let delivery = self.delivery;
        let record = self.index.journey(journey);
        let projection = delivery.name(record.document).and_then(|document| {
            let id = delivery.name(record.id)?;
            let named = delivery.journey_facts(document, id)?.projection?;
            delivery.projection(document, named)
        });
        match projection {
            Some(projection) if delivery.gather.projections[projection as usize].has_geometry() => {
                Some(Course::Projection(projection))
            }
            _ => self.links_of(journey).map(Course::Links),
        }
    }

    /// The service links a journey runs over, each with a geometry, as
    /// positions among the gather's links: from each call on, the links its
    /// pattern names in sequence from that call's stop point to a later
    /// one's — a link to the next call each, or one for the whole way, as
    /// Finland's deliveries name them —, else a link the delivery states to
    /// the next call's; each as the journey's document sees it, the
    /// codespace's shared data where the Nordic deliveries define them. None
    /// where a call has neither on to the next.
    fn links_of(&self, journey: JourneyId) -> Option<Vec<u32>> {
        let delivery = self.delivery;
        let record = self.index.journey(journey);
        let document = delivery.name(record.document)?;
        let named: Vec<Option<u32>> = record
            .pattern
            .and_then(|pattern| delivery.name(pattern))
            .map_or(&[][..], |pattern| delivery.pattern_links(document, pattern))
            .iter()
            .map(|&link| delivery.link(document, link))
            .collect();
        let stop_points: Vec<Name> = self
            .calls(journey)
            .iter()
            .map(|&(position, _)| {
                delivery.name(self.index.stop_point(record.stop_points[position]).id)
            })
            .collect::<Option<_>>()?;
        let links = &delivery.gather.links;
        let mut found = Vec::new();
        let (mut call, mut next) = (0, 0);
        while call + 1 < stop_points.len() {
            let (from, later) = (stop_points[call], &stop_points[call + 1..]);
            match named_run(&named, next, links, from, later) {
                Some(run) => {
                    found.extend(named[run.start..run.end].iter().flatten());
                    next = run.end;
                    call += 1 + run.calls_on;
                }
                None => {
                    found.push(delivery.link_between(document, from, later[0])?);
                    call += 1;
                }
            }
        }
        Some(found)
    }

    /// The positions of `course` as latitude and longitude: a projection's,
    /// read in the order that starts nearest the journey's first stop; or
    /// each link's one after the other, read in the order that starts
    /// nearest the stop point it leads from — each in its system, its own
    /// `srsName` else its document's default, a position repeated where one
    /// link ends and the next begins written once.
    fn shape_points(&mut self, journey: JourneyId, course: &Course) -> Vec<(f64, f64)> {
        let mut points: Vec<(f64, f64)> = Vec::new();
        match course {
            Course::Projection(projection) => {
                let projection = self.delivery.gather.projections[*projection as usize];
                let start = self
                    .index
                    .journey(journey)
                    .stop_points
                    .first()
                    .and_then(|&point| self.position_of_stop_point(point));
                let stretch = Stretch {
                    id: projection.id,
                    document: projection.document,
                    system: projection.system,
                    positions: projection.positions,
                };
                self.add_stretch(&mut points, stretch, start);
            }
            Course::Links(links) => {
                for &link in links {
                    let link = self.delivery.gather.links[link as usize];
                    let start = self
                        .index
                        .stop_point_by_id(self.delivery.gather.names.text(link.from))
                        .and_then(|point| self.position_of_stop_point(point));
                    let stretch = Stretch {
                        id: link.id,
                        document: link.document,
                        system: link.system,
                        positions: link.positions,
                    };
                    self.add_stretch(&mut points, stretch, start);
                }
            }
        }
        points
    }

    /// Appends the positions of `stretch` to `points` in WGS 84, read in the
    /// axis order that puts its first position near `start`, where that is
    /// known.
    fn add_stretch(
        &mut self,
        points: &mut Vec<(f64, f64)>,
        stretch: Stretch,
        start: Option<(f64, f64)>,
    ) {
        let gather = &self.delivery.gather;
        let named = stretch.system.map(|system| gather.names.text(system));
        let default_system = self
            .delivery
            .document_facts(stretch.document)
            .location_system
            .map(|system| gather.names.text(system));
        let Ok(system) = crs::system_of(named, default_system) else {
            self.decisions.take(
                "shape-in-unknown-system",
                "a geometry of a service link or link sequence projection in a reference system the crate does not know: its stretch of the shape is left out",
                || gather.names.text(stretch.id).to_owned(),
            );
            return;
        };
        let written = &gather.positions[stretch.positions.0 as usize..stretch.positions.1 as usize];
        let first = written[0];
        let readings = [
            system.to_wgs84(first.0, first.1),
            system.to_wgs84(first.1, first.0),
        ];
        let swapped =
            start.is_some_and(|start| reading_order(readings, start) == AxisOrder::LongitudeFirst);
        let read: Vec<(f64, f64)> = written
            .iter()
            .map(|&(a, b)| {
                if swapped {
                    system.to_wgs84(b, a)
                } else {
                    system.to_wgs84(a, b)
                }
            })
            .collect();
        if read
            .iter()
            .any(|&(latitude, longitude)| on_the_earth(Some(latitude), Some(longitude)).is_none())
        {
            self.decisions.take(
                "shape-off-the-earth",
                "a geometry of a service link or link sequence projection with a position off the earth or at 0, 0, as read in its reference system: its stretch of the shape is left out",
                || gather.names.text(stretch.id).to_owned(),
            );
            return;
        }
        for point in read {
            if points.last() != Some(&point) {
                points.push(point);
            }
        }
    }

    /// Where a stop point is: its own position, else its stop's, else that
    /// stop's stop place's.
    fn position_of_stop_point(&self, point: StopPointId) -> Option<(f64, f64)> {
        let record = self.index.stop_point(point);
        on_the_earth(record.latitude, record.longitude).or_else(|| {
            let stop = self.index.stop(record.stop?);
            on_the_earth(stop.latitude, stop.longitude).or_else(|| {
                let place = self.index.stop(stop.parent?);
                on_the_earth(place.latitude, place.longitude)
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
                self.feed_zone.name.as_str(),
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

    /// `stops.txt`; with `stop_timezone` where a stop is in another time
    /// zone than the feed's ([`Conversion::plan_stop_zones`]).
    fn stops(&mut self, output: &mut dyn Output) -> io::Result<u64> {
        let mut header = vec![
            "stop_id",
            "stop_name",
            "stop_lat",
            "stop_lon",
            "location_type",
            "parent_station",
            "wheelchair_boarding",
            "platform_code",
            "level_id",
        ];
        let zoned = !self.stop_zones.is_empty();
        if zoned {
            header.push("stop_timezone");
        }
        let mut table = Table::start(output, "stops.txt", &header)?;
        for (number, row) in self.stop_rows.iter().enumerate() {
            let (latitude, longitude) = (
                format!("{:.7}", row.position.0),
                format!("{:.7}", row.position.1),
            );
            let mut fields = vec![
                row.id.as_str(),
                row.name.as_str(),
                &latitude,
                &longitude,
                if row.location_type == 1 { "1" } else { "0" },
                row.parent
                    .map_or("", |parent| self.stop_rows[parent as usize].id.as_str()),
                wheelchair(row.wheelchair),
                row.platform.as_deref().unwrap_or(""),
                row.level.as_deref().unwrap_or(""),
            ];
            if zoned {
                let number = u32::try_from(number).expect("fewer than 2^32 stops");
                fields.push(self.stop_zones.get(&number).map_or("", String::as_str));
            }
            table.row(fields)?;
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
            // A long name repeating the short one says nothing more; GTFS
            // requires one of the two.
            let long = if short.is_empty() {
                name_or_identifier(
                    &mut self.decisions,
                    record.name,
                    record.id,
                    ROUTE_NAMED_BY_ITS_IDENTIFIER,
                )
            } else if record.name == short {
                String::new()
            } else {
                record.name.to_owned()
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
                long.as_str(),
                route_type.unwrap_or("3"),
                text(facts.and_then(|f| f.colour)),
                text(facts.and_then(|f| f.text_colour)),
            ])?;
        }
        Ok(table.rows())
    }

    /// `stop_times.txt`, `trips.txt` and `frequencies.txt`, trip by trip;
    /// the stop times, the large file, written as they are made, and first,
    /// so that each trip's calls and displays are found once.
    fn trips(&mut self, output: &mut dyn Output) -> io::Result<(u64, u64, u64)> {
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
        let mut starts = Vec::with_capacity(self.trips.len());
        for number in 0..self.trips.len() {
            starts.push(self.stop_times(number, &mut table)?);
        }
        let stop_times = table.rows();
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
        for (number, &start) in starts.iter().enumerate() {
            let row = self.trip_row(number, start, &mut frequencies);
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
        Ok((trips, stop_times, table.rows()))
    }

    /// The row of `trips.txt` of the trip numbered `number`, which starts as
    /// `start` says; its headway groups added to `frequencies` (trip, start,
    /// end, headway).
    fn trip_row(
        &self,
        number: usize,
        start: TripStart,
        frequencies: &mut Vec<[String; 4]>,
    ) -> [String; 8] {
        let trip = &self.trips[number];
        let record = self.index.journey(trip.journey);
        let document = self.delivery.name(record.document).unwrap_or(Name::MAX);
        let delivery = self.delivery;
        let facts = delivery
            .name(record.id)
            .and_then(|id| delivery.journey_facts(document, id));
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
            self.service_name(trip.service),
            trip.id.clone(),
            start
                .headsign
                .map_or_else(String::new, |h| delivery.text(h).to_owned()),
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
        // without exact times. GTFS runs a frequency from its start up to,
        // not including, its end, from the trip's first stop: from the
        // group's first departure to a second after its last, each as much
        // later as the trip's first call is after the journey's.
        for headway in record.headways {
            let first = headway.first_departure + start.delay;
            let last = headway.last_departure + start.delay + 1;
            frequencies.push([
                trip.id.clone(),
                time_text(shifted(Some(first), trip.shift)),
                time_text(shifted(Some(last), trip.shift)),
                headway.interval.to_string(),
            ]);
        }
        row
    }

    /// The rows of `stop_times.txt` of the trip numbered `number`; how it
    /// starts.
    fn stop_times(&self, number: usize, table: &mut Table<'_>) -> io::Result<TripStart> {
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
                &time_text(shifted(time.arrival.or(time.departure), trip.shift)),
                &time_text(shifted(time.departure.or(time.arrival), trip.shift)),
                self.stop_rows[stop_row as usize].id.as_str(),
                &sequence.to_string(),
                if call.boarding { "0" } else { "1" },
                if call.alighting { "0" } else { "1" },
                stop_headsign.map_or("", |shown| self.delivery.text(shown)),
            ])?;
        }
        Ok(TripStart {
            headsign,
            delay: delay(record.passing_times, &calls),
        })
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
            // A call the points do not name — a journey stating calls of its
            // own beside its pattern — shows what is shown before it.
            let found = stops[next..]
                .iter()
                .position(|point| Some(point.stop_point) == id);
            if let Some(found) = found {
                for point in &stops[next..=next + found] {
                    if let Some(display) = point
                        .display
                        .and_then(|display| delivery.display(document, display))
                    {
                        shown = Some(display.text);
                    }
                }
                next += found + 1;
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
            service_rows(
                service_id(calendar),
                &days,
                &mut calendar_rows,
                &mut date_rows,
            );
        }
        for service in &self.own_services {
            service_rows(
                service.id.clone(),
                &service.days,
                &mut calendar_rows,
                &mut date_rows,
            );
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
    /// staying seated (4), guaranteed (1), else recommended (0). Each in the
    /// order of the identifiers of the stops and stop points they start at.
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
        for stop in self.stops_in_order() {
            let mut from_stop: Vec<_> = self.index.transfers_from(stop).iter().collect();
            from_stop
                .sort_by_key(|transfer| (self.stop_rank[transfer.to.0 as usize], transfer.seconds));
            for transfer in from_stop {
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
        let mut repeated: Vec<String> = Vec::new();
        let mut points: Vec<StopPointId> = (0..self.index.stop_point_count() as u32)
            .map(StopPointId)
            .collect();
        points.sort_unstable_by(|&a, &b| stop_point_order(self.index, a, b));
        for point in points {
            for interchange in self.index.interchanges_at(point) {
                let from_trips = self.trips_of(interchange.from_journey);
                let to_trips = self.trips_of(interchange.to_journey);
                let ends = (
                    row_of_point(interchange.from_stop_point),
                    row_of_point(interchange.to_stop_point),
                );
                let (Some(from), Some(to)) = ends else {
                    left_out.push(self.index.journey(interchange.from_journey).id);
                    continue;
                };
                if from_trips.is_empty() || to_trips.is_empty() {
                    left_out.push(self.index.journey(interchange.from_journey).id);
                    continue;
                }
                let kind = if interchange.stay_seated {
                    "4"
                } else if interchange.guaranteed {
                    "1"
                } else {
                    "0"
                };
                // Between every trip of each journey: one split by its days
                // is each of its trips.
                let mut stated_again = false;
                for from_trip in from_trips {
                    for to_trip in to_trips {
                        let (from_trip, to_trip) = (from_trip.id.as_str(), to_trip.id.as_str());
                        if written.insert((from, to, from_trip, to_trip)) {
                            table.row([from, to, from_trip, to_trip, kind, ""])?;
                        } else {
                            stated_again = true;
                        }
                    }
                }
                if stated_again {
                    repeated.push(from_trips[0].id.clone());
                }
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
        let sources = &self.sources;
        let mut written = Vec::new();
        let mut documents: Vec<(&str, Name)> = delivery
            .gather
            .documents
            .keys()
            .map(|&d| (delivery.text(d), d))
            .collect();
        documents.sort_unstable();
        match &self.publisher {
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
                    .max_by(|&a, &b| publication_order(a, b))
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
            for &(name, url) in sources {
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
        for service in &self.own_services {
            if let (Some(&day), Some(&last_day)) = (service.days.first(), service.days.last()) {
                first = first.min(day);
                last = last.max(last_day);
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

/// Of the `calls` of a journey, by position among its calls, those from the
/// first with a time to the last: GTFS requires times at a trip's first and
/// last stop, and a call left out of a trip — at a stop the feed cannot
/// place — may leave one without at an end.
fn timed_ends(mut calls: Vec<(usize, u32)>, times: &[PassingTime]) -> Vec<(usize, u32)> {
    let timed = |&(position, _): &(usize, u32)| time_at(times[position]).is_some();
    let start = calls.iter().position(timed).unwrap_or(calls.len());
    let end = calls.iter().rposition(timed).map_or(start, |last| last + 1);
    calls.truncate(end);
    calls.drain(..start);
    calls
}

/// When a call takes place: its departure, else its arrival.
fn time_at(time: PassingTime) -> Option<u32> {
    time.departure.or(time.arrival)
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

/// The data sources a delivery names, each by its name and web address —
/// where it states one with its scheme ([`web_address`]) —, in their order,
/// once.
fn data_sources<'d>(
    delivery: &'d Delivery,
    decisions: &mut Decisions,
) -> Vec<(&'d str, Option<&'d str>)> {
    let mut sources: Vec<(&str, Option<&str>)> = delivery
        .gather
        .data_sources
        .iter()
        .map(|&(_, name, url)| (delivery.text(name), url.map(|u| delivery.text(u))))
        .collect();
    sources.sort_unstable();
    sources.dedup();
    sources
        .into_iter()
        .map(|(name, url)| (name, web_address(decisions, url, name)))
        .collect()
}

/// `url`, the web address of `holder`, where it is one GTFS takes: a fully
/// qualified one, with its scheme, `http://` or `https://`. The deliveries
/// write `www.…` and `//…` too, which a reader of the feed cannot follow:
/// taken as none, the decision taken.
fn web_address<'u>(
    decisions: &mut Decisions,
    url: Option<&'u str>,
    holder: &str,
) -> Option<&'u str> {
    let url = url?;
    let scheme = url.split_once("://").map(|(scheme, _)| scheme);
    let full = scheme.is_some_and(|scheme| {
        scheme.eq_ignore_ascii_case("http") || scheme.eq_ignore_ascii_case("https")
    }) && !url.ends_with("://");
    if !full {
        decisions.take(
            "url-without-scheme",
            "a web address without its scheme (`www.…`, `//…`): taken as none, as GTFS requires one with `http://` or `https://`",
            || format!("{holder}: {url}"),
        );
        return None;
    }
    Some(url)
}

/// A position as GTFS takes it, from a latitude and a longitude in degrees:
/// one on the earth, and other than 0, 0 — where a position left unset is
/// written, not a stop in the Gulf of Guinea.
fn on_the_earth(latitude: Option<f64>, longitude: Option<f64>) -> Option<(f64, f64)> {
    let (latitude, longitude) = latitude.zip(longitude)?;
    let earth = (-90.0..=90.0).contains(&latitude) && (-180.0..=180.0).contains(&longitude);
    (earth && (latitude, longitude) != (0.0, 0.0)).then_some((latitude, longitude))
}

/// Counts each stop place, quay and stop point of `index` whose position is
/// none GTFS takes ([`on_the_earth`]): every position of the feed is read
/// through it, as though the delivery stated none there.
fn note_positions_off_the_earth(index: &Index, decisions: &mut Decisions) {
    let mut note = |id: &str, latitude: Option<f64>, longitude: Option<f64>| {
        let Some((latitude, longitude)) = latitude.zip(longitude) else {
            return;
        };
        if on_the_earth(Some(latitude), Some(longitude)).is_some() {
            return;
        }
        if (latitude, longitude) == (0.0, 0.0) {
            decisions.take(
                "position-at-zero-zero",
                "a stop place, quay or stop point at latitude 0, longitude 0, where a position left unset is written: taken as without a position",
                || id.to_owned(),
            );
        } else {
            decisions.take(
                "position-off-the-earth",
                "a stop place, quay or stop point at a latitude beyond ±90° or a longitude beyond ±180°: taken as without a position",
                || id.to_owned(),
            );
        }
    };
    let mut stops: Vec<StopId> = index.stops().collect();
    stops.sort_unstable_by_key(|&stop| index.stop(stop).id);
    for stop in stops {
        let record = index.stop(stop);
        note(record.id, record.latitude, record.longitude);
    }
    let mut points: Vec<StopPointId> = (0..index.stop_point_count() as u32)
        .map(StopPointId)
        .collect();
    points.sort_unstable_by(|&a, &b| stop_point_order(index, a, b));
    for point in points {
        let record = index.stop_point(point);
        note(record.id, record.latitude, record.longitude);
    }
}

/// A journey left with fewer than two calls: before the stops are placed,
/// with calls at a place; after, with calls at a placed stop from the first
/// with a time to the last.
const TRIP_WITHOUT_TWO_CALLS: (&str, &str) = (
    "trip-without-two-calls",
    "a journey left with fewer than two calls at a place: left out",
);

/// A stop named by its identifier, as GTFS requires a name.
const STOP_NAMED_BY_ITS_IDENTIFIER: (&str, &str) = (
    "stop-named-by-its-identifier",
    "a stop without a name, nor a stop place with one: named by its identifier, as GTFS requires a name",
);

/// An agency named by its identifier, as GTFS requires a name.
const AGENCY_NAMED_BY_ITS_IDENTIFIER: (&str, &str) = (
    "agency-named-by-its-identifier",
    "an organisation without a name: named by its identifier, as GTFS requires a name",
);

/// A route named by its identifier, as GTFS requires a name.
const ROUTE_NAMED_BY_ITS_IDENTIFIER: (&str, &str) = (
    "route-named-by-its-identifier",
    "a line without a name or a number: its long name its identifier, as GTFS requires one of the two",
);

/// `name`, else — GTFS requiring one — the identifier `id` the delivery
/// states, the decision `code` taken.
fn name_or_identifier(
    decisions: &mut Decisions,
    name: &str,
    id: &str,
    (code, what): (&'static str, &'static str),
) -> String {
    if name.trim().is_empty() {
        decisions.take(code, what, || id.to_owned());
        id.to_owned()
    } else {
        name.to_owned()
    }
}

/// The time zone `zone` names, where it is written as a name of the time
/// zone database, as GTFS requires — an area and a location
/// (`Europe/Zurich`, `America/Argentina/Buenos_Aires`, `Etc/GMT+1`), or
/// `UTC` — and the database bundled with the crate holds it. NeTEx's
/// `TimeZone` is free text, and the examples of the standard write
/// abbreviations of an offset — `CET`, `CEST`, `BST` —, which name no rules
/// over the year (the database holds `CET` all the same, for old systems).
fn time_zone_named(zone: &str) -> Option<Zone> {
    const AREAS: [&str; 11] = [
        "Africa",
        "America",
        "Antarctica",
        "Arctic",
        "Asia",
        "Atlantic",
        "Australia",
        "Etc",
        "Europe",
        "Indian",
        "Pacific",
    ];
    let mut parts = zone.split('/');
    let area = parts.next().unwrap_or_default();
    let mut locations = parts.peekable();
    let written = zone == "UTC"
        || AREAS.contains(&area)
            && locations.peek().is_some()
            && locations.all(|location| {
                !location.is_empty()
                    && location.bytes().all(|byte| {
                        byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'+')
                    })
            });
    if !written {
        return None;
    }
    let rules = jiff::tz::db().get(zone).ok()?;
    let name = rules.iana_name().unwrap_or(zone).to_owned();
    Some(Zone { name, rules })
}

/// The order of two publication timestamps by the instant they name — an
/// offset stated taken away, one without as if in UTC —, then by their
/// text; a timestamp that is none sorts before every one that is.
fn publication_order(a: &str, b: &str) -> Ordering {
    (instant(a), a).cmp(&(instant(b), b))
}

/// The seconds since 1970-01-01 in UTC an `xsd:dateTime` names, its offset
/// (`Z`, `+02:00`) taken away; one without an offset as if in UTC.
fn instant(timestamp: &str) -> Option<i64> {
    let timestamp = timestamp.trim();
    let written = DateTime::from(timestamp);
    let day = written.date()?.days_since_epoch()?;
    let time = written.time()?.seconds_since_midnight()?;
    let (_, after_date) = timestamp.split_once('T')?;
    let offset = match after_date.find(['Z', '+', '-']) {
        None => 0,
        Some(at) => match &after_date[at..] {
            "Z" => 0,
            offset => {
                let (sign, rest) = offset.split_at(1);
                let (hours, minutes) = rest.split_once(':')?;
                let (hours, minutes): (i64, i64) = (hours.parse().ok()?, minutes.parse().ok()?);
                let seconds = hours * 3600 + minutes * 60;
                if sign == "-" {
                    -seconds
                } else {
                    seconds
                }
            }
        },
    };
    Some(day * 86_400 + i64::from(time) - offset)
}

/// What a trip's shape is drawn from, the key trips share a shape by.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum Course {
    /// The link sequence projection its journey names: its position among
    /// the gather's projections.
    Projection(u32),
    /// The service links it runs over: their positions among the gather's links.
    Links(Vec<u32>),
}

/// A geometry of a course: the link's or projection's identifier and
/// document, the reference system it names, and its positions, a range of
/// the gather's.
#[derive(Debug, Clone, Copy)]
struct Stretch {
    id: Name,
    document: Name,
    system: Option<Name>,
    positions: (u32, u32),
}

/// A run of the links a pattern names in sequence: where it starts, where it
/// ends — past its last —, and which of the calls after the one it leads
/// from it leads to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Run {
    start: usize,
    end: usize,
    calls_on: usize,
}

/// Of the links a pattern names in sequence — positions among `links`, none
/// for a link the delivery does not define —, from the one at `next` on, the
/// run leading from stop point `from` to the first of the `later` calls'
/// stop points one of its links leads to. None where there is none, or a
/// link of it states no geometry.
fn named_run(
    named: &[Option<u32>],
    next: usize,
    links: &[LinkGeometry],
    from: Name,
    later: &[Name],
) -> Option<Run> {
    let link = |at: usize| named[at].map(|link| &links[link as usize]);
    let start = (next..named.len()).find(|&at| link(at).is_some_and(|link| link.from == from))?;
    for at in start..named.len() {
        let link = link(at).filter(|link| link.has_geometry())?;
        if let Some(calls_on) = later.iter().position(|&stop_point| stop_point == link.to) {
            return Some(Run {
                start,
                end: at + 1,
                calls_on,
            });
        }
    }
    None
}

/// Each stop's place in the order of the stops' identifiers, versions and
/// names, by its number in the index — which follows the order the threads
/// reading the delivery took its documents in.
fn stop_ranks(index: &Index) -> Vec<u32> {
    let mut stops: Vec<StopId> = index.stops().collect();
    stops.sort_unstable_by_key(|&stop| {
        let record = index.stop(stop);
        (record.id, index.stop_version(stop), record.name)
    });
    let mut ranks = vec![0; stops.len()];
    for (rank, stop) in stops.into_iter().enumerate() {
        ranks[stop.0 as usize] = u32::try_from(rank).expect("fewer than 2^32 stops");
    }
    ranks
}

/// The order of two stop points by their identifiers, versions and names,
/// which tell every stop point apart.
fn stop_point_order(index: &Index, a: StopPointId, b: StopPointId) -> Ordering {
    let key = |point: StopPointId| {
        let record = index.stop_point(point);
        (record.id, index.stop_point_version(point), record.name)
    };
    key(a).cmp(&key(b))
}

/// The agency of lines naming none, from the publisher.
const PUBLISHER_AGENCY: &str = "publisher";

/// Identifiers made unique: the first of one text is the text, each after
/// it numbered `#2`, `#3`, … — the next number whose identifier none handed
/// out has taken, an identifier numbered in the delivery among them.
#[derive(Debug, Default)]
struct UniqueIds {
    taken: FxHashSet<String>,
    /// The number each text was handed out with last.
    numbered: FxHashMap<String, u32>,
}

impl UniqueIds {
    fn unique(&mut self, id: &str) -> String {
        if self.taken.insert(id.to_owned()) {
            return id.to_owned();
        }
        let number = self.numbered.entry(id.to_owned()).or_insert(1);
        loop {
            *number += 1;
            let unique = format!("{id}#{number}");
            if self.taken.insert(unique.clone()) {
                return unique;
            }
        }
    }
}

impl Profile {
    /// The prefix of the identifiers of a national profile.
    fn prefix(self) -> Option<&'static str> {
        match self {
            Profile::Delfi => Some("DE::"),
            Profile::Austria => Some("at:"),
            Profile::Switzerland => Some("ch:"),
            Profile::Epip | Profile::Other => None,
        }
    }

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

/// The profile a delivery's frames or identifiers show: the European one
/// where a frame is typed as its, else the national one most of its
/// journeys' identifiers show.
fn profile(index: &Index, epip: bool) -> Profile {
    if epip {
        return Profile::Epip;
    }
    identified(index).map_or(Profile::Other, |(profile, _, _)| profile)
}

/// The national profile most of a delivery's journeys' identifiers show by
/// their prefix — `DE::`, `at:`, `ch:` —, with how many show it of how many
/// there are; none where no prefix is on more than half.
fn identified(index: &Index) -> Option<(Profile, u32, u32)> {
    const PROFILES: [Profile; 3] = [Profile::Delfi, Profile::Austria, Profile::Switzerland];
    let mut counts = [0u32; 3];
    for journey in (0..index.journey_count() as u32).map(JourneyId) {
        let id = index.journey(journey).id;
        if let Some(slot) = PROFILES.iter().position(|profile| {
            profile
                .prefix()
                .is_some_and(|prefix| id.starts_with(prefix))
        }) {
            counts[slot] += 1;
        }
    }
    let total = u32::try_from(index.journey_count()).expect("fewer than 2^32 journeys");
    let (slot, &most) = counts
        .iter()
        .enumerate()
        .max_by_key(|&(slot, &count)| (count, std::cmp::Reverse(slot)))?;
    (most > 0 && most * 2 > total).then_some((PROFILES[slot], most, total))
}

/// The time zone of a delivery whose documents name none and that is given
/// none: of the country its journeys' identifiers show — the Swiss and
/// Austrian documents state offsets from UTC, or nothing —, else of the
/// country most of its stops with a position lie in; the decision taken.
fn time_zone_of_country(index: &Index, decisions: &mut Decisions) -> Result<&'static str, Error> {
    if let Some((profile, count, total)) = identified(index) {
        let zone = profile
            .time_zone()
            .expect("a national profile has a time zone");
        let prefix = profile.prefix().expect("a national profile has a prefix");
        decisions.take(
            "time-zone-of-identifiers",
            "no document names a time zone and none is given: the time zone of the country most of the journeys' identifiers show (`DE::`, `at:`, `ch:`)",
            || format!("{zone}: {count} of {total} journeys identified {prefix}"),
        );
        return Ok(zone);
    }
    let mut placed = 0u32;
    let mut by_country: BTreeMap<&'static str, (&'static str, u32)> = BTreeMap::new();
    for stop in index.stops() {
        let record = index.stop(stop);
        let Some(position) = on_the_earth(record.latitude, record.longitude) else {
            continue;
        };
        placed += 1;
        if let Some(country) = country::country_of(position) {
            by_country
                .entry(country.code)
                .or_insert((country.zone, 0))
                .1 += 1;
        }
    }
    let (code, (zone, count)) = by_country
        .into_iter()
        .max_by_key(|&(code, (_, count))| (count, std::cmp::Reverse(code)))
        .filter(|&(_, (_, count))| count * 2 > placed)
        .ok_or(Error::NoTimeZone)?;
    decisions.take(
        "time-zone-of-stops",
        "no document names a time zone, none is given, and the journeys' identifiers show no country: the time zone of the country most of the stops with a position lie in",
        || format!("{zone}: {count} of {placed} stops with a position in {code}"),
    );
    Ok(zone)
}

/// The GTFS route type of a NeTEx mode; none for a mode GTFS has none for.
fn route_type(mode: Option<&str>) -> Option<&'static str> {
    Some(match mode? {
        "tram" => "0",
        "metro" => "1",
        "rail" => "2",
        "bus" | "coach" => "3",
        "water" | "ferry" => "4",
        "cableway" | "telecabin" => "6",
        "funicular" => "7",
        "trolleyBus" => "11",
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

/// The rows of `calendar.txt` and `calendar_dates.txt` of the service
/// `service` running on `days`, in order: the weekdays it runs on on most of
/// their dates from its first day to its last, and each date it differs from
/// them on.
fn service_rows(
    service: String,
    days: &[i64],
    calendar_rows: &mut Vec<[String; 10]>,
    date_rows: &mut Vec<[String; 3]>,
) {
    let (first, last) = (days[0], days[days.len() - 1]);
    let mut active = [0u32; 7];
    let mut total = [0u32; 7];
    for day in first..=last {
        total[weekday(day)] += 1;
    }
    for &day in days {
        active[weekday(day)] += 1;
    }
    let weekdays: [bool; 7] = std::array::from_fn(|w| active[w] > 0 && active[w] * 2 >= total[w]);
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

/// A count of trips as a trip's number, or a part's.
fn trip_number(count: usize) -> u32 {
    u32::try_from(count).expect("fewer than 2^32 trips")
}

/// How long after the journey's first call the first of its `calls` written
/// is: a headway group's departures are those of the journey's first call.
/// The index starts a journey at a call with a time, and so does the feed a
/// trip ([`timed_ends`]).
fn delay(times: &[PassingTime], calls: &[(usize, u32)]) -> u32 {
    let time_of = |position: usize| time_at(times[position]);
    calls
        .first()
        .and_then(|&(position, _)| Some(time_of(position)?.saturating_sub(time_of(0)?)))
        .unwrap_or(0)
}

/// The earliest time a trip writes: of its `calls`, arriving or departing,
/// and of its headway groups' first departures.
fn earliest_time(times: &[PassingTime], headways: &[Headway], calls: &[(usize, u32)]) -> u32 {
    let delay = delay(times, calls);
    calls
        .iter()
        .flat_map(|&(position, _)| [times[position].arrival, times[position].departure])
        .flatten()
        .chain(
            headways
                .iter()
                .map(|headway| headway.first_departure + delay),
        )
        .min()
        .unwrap_or(0)
}

/// `time` written `shift` seconds later; a trip is shifted no earlier than
/// its earliest time allows ([`in_feed_zone`]).
fn shifted(time: Option<u32>, shift: i64) -> Option<u32> {
    time.map(|time| {
        u32::try_from(i64::from(time) + shift)
            .expect("a trip is shifted to begin on its day or later")
    })
}

/// How a day of a journey whose earliest time is `earliest` is written in
/// the time zone `feed` rather than in `zone`, the instants kept: how many
/// days earlier — none, or one where it would begin before its day — and
/// how many seconds later its times are. A GTFS time counts from twelve
/// hours before noon of its day, so the times of a day are as much later as
/// noon is later in `zone` than in `feed`. None where the rules of either
/// zone do not reach the day.
fn in_feed_zone(zone: &TimeZone, feed: &TimeZone, day: i64, earliest: u32) -> Option<(i64, i64)> {
    let noon_there = noon(zone, day)?;
    let mut moved = 0;
    loop {
        let shift = noon_there - noon(feed, day + moved)?;
        if i64::from(earliest) + shift >= 0 {
            return Some((moved, shift));
        }
        moved -= 1;
    }
}

/// The instant noon of `day` — days since 1970-01-01 — is in `zone`, in
/// seconds since 1970-01-01 in UTC; none beyond the instants the time zone
/// rules reach.
fn noon(zone: &TimeZone, day: i64) -> Option<i64> {
    let date = jiff::civil::date(1970, 1, 1)
        .checked_add(jiff::Span::new().try_days(day).ok()?)
        .ok()?;
    let noon = date.at(12, 0, 0, 0).to_zoned(zone.clone()).ok()?;
    Some(noon.timestamp().as_second())
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
    use crate::delivery::Reading;
    use crate::output;

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

    #[test]
    fn an_identifier_numbered_already_is_not_handed_out_twice() {
        let mut ids = UniqueIds::default();
        let handed_out: Vec<String> = ["a#2", "a", "a"].iter().map(|id| ids.unique(id)).collect();
        let distinct: std::collections::BTreeSet<&String> = handed_out.iter().collect();
        assert_eq!(distinct.len(), handed_out.len(), "{handed_out:?}");
    }

    #[test]
    fn a_trolleybus_is_a_trolleybus() {
        assert_eq!(route_type(Some("trolleyBus")), Some("11"));
        assert_eq!(route_type(Some("bus")), Some("3"));
        assert_eq!(route_type(Some("snowAndIce")), None);
    }

    #[test]
    fn only_a_name_of_the_time_zone_database_is_a_time_zone() {
        for zone in [
            "Europe/Zurich",
            "America/Argentina/Buenos_Aires",
            "Etc/GMT+1",
            "UTC",
        ] {
            assert!(time_zone_named(zone).is_some(), "{zone}");
        }
        // Written as a name, and its location's own: the database's.
        assert_eq!(
            time_zone_named("Europe/zurich").map(|zone| zone.name),
            Some("Europe/Zurich".to_owned())
        );
        // As the examples of the standard write them: abbreviations of an
        // offset, which name no zone's rules over the year; and names the
        // database does not hold.
        for zone in [
            "CET",
            "CEST",
            "BST",
            "GMT",
            "+01:00",
            "",
            "Europe/",
            "Mars/Olympus",
            "Europe/Nowhere",
        ] {
            assert!(time_zone_named(zone).is_none(), "{zone}");
        }
    }

    #[test]
    fn every_zone_a_country_or_a_profile_takes_is_one_of_the_database() {
        let zones = country::countries()
            .iter()
            .map(|country| country.zone)
            .chain(
                [Profile::Delfi, Profile::Austria, Profile::Switzerland].map(|profile| {
                    profile
                        .time_zone()
                        .expect("a national profile has a time zone")
                }),
            );
        for zone in zones {
            assert_eq!(
                time_zone_named(zone).map(|zone| zone.name).as_deref(),
                Some(zone)
            );
        }
    }

    #[test]
    fn publication_timestamps_are_ordered_by_the_instant_they_name() {
        let later = |a: &str, b: &str| {
            [a, b]
                .into_iter()
                .max_by(|&a, &b| publication_order(a, b))
                .expect("two")
                .to_owned()
        };
        // 06:00 at UTC+2 is 04:00 UTC, before 05:00 UTC.
        assert_eq!(
            later("2026-10-10T06:00:00+02:00", "2026-10-10T05:00:00Z"),
            "2026-10-10T05:00:00Z"
        );
        assert_eq!(
            later("2026-10-10T23:30:00-01:00", "2026-10-11T00:15:00Z"),
            "2026-10-10T23:30:00-01:00"
        );
        assert_eq!(
            later("2026-10-10T06:00:00.5", "2026-10-10T06:00:00"),
            "2026-10-10T06:00:00.5"
        );
        // A timestamp that is none sorts before every one that is.
        assert_eq!(
            later("yesterday", "2026-10-10T06:00:00"),
            "2026-10-10T06:00:00"
        );
    }

    /// Every file of the feed `delivery` converts to, by name, with the report.
    fn feed(delivery: &Delivery) -> (BTreeMap<String, Vec<u8>>, Report) {
        let directory = tempfile::tempdir().expect("a directory");
        let options = Options {
            shapes: true,
            ..Options::default()
        };
        let report = convert(
            delivery,
            &options,
            output::at(directory.path()).expect("an output"),
        )
        .expect("converts");
        let files = std::fs::read_dir(directory.path())
            .expect("the feed")
            .map(|entry| {
                let path = entry.expect("a file").path();
                let name = path
                    .file_name()
                    .expect("a name")
                    .to_string_lossy()
                    .into_owned();
                (name, std::fs::read(&path).expect("the file reads"))
            })
            .collect();
        (files, report)
    }

    #[test]
    fn the_feed_does_not_depend_on_the_order_the_threads_reading_it_are_merged_in() {
        let fixture = |name: &str| {
            let path = format!(
                "{}/tests/fixtures/uferweg/{name}",
                env!("CARGO_MANIFEST_DIR")
            );
            std::fs::read_to_string(path).expect("the fixture")
        };
        let (north, south) = (fixture("uferweg-nord.xml"), fixture("uferweg-sued.xml"));
        let north = [("uferweg-nord.xml", north.as_str())];
        let south = [("uferweg-sued.xml", south.as_str())];
        let reading = Reading {
            geometry: true,
            ..Reading::default()
        };
        let (files, report) = feed(&Delivery::read_by_workers(&[&north, &south], reading));
        let (other_files, other_report) =
            feed(&Delivery::read_by_workers(&[&south, &north], reading));
        assert_eq!(
            files.keys().collect::<Vec<_>>(),
            other_files.keys().collect::<Vec<_>>()
        );
        for (name, bytes) in &files {
            assert!(
                &other_files[name] == bytes,
                "{name} differs:\n{}\n---\n{}",
                String::from_utf8_lossy(bytes),
                String::from_utf8_lossy(&other_files[name])
            );
        }
        assert_eq!(other_report, report);
    }
}
