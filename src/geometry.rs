//! Coordinates as a delivery writes them in a link's geometry.
//!
//! A GML position list in WGS 84 (EPSG:4326) is latitude first; the Austrian
//! deliveries write their service links' geometries longitude first. Which
//! order a link's geometry is written in is told by its first position against
//! where the link starts: the reading of the position that lies near the
//! link's start stop point is the one meant. Each link is judged on its own,
//! so a delivery that writes some links one way and some the other is read
//! right link by link.

use quick_xml::events::Event;
use quick_xml::{Reader, XmlVersion};

/// How near the link's start its first position must lie, read one way, for
/// that reading to be the one meant, in metres.
pub const NEAR_THE_START_METRES: f64 = 150.0;

/// The order a geometry's positions are written in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AxisOrder {
    /// Latitude first, as WGS 84 has it.
    LatitudeFirst,
    /// Longitude first.
    LongitudeFirst,
    /// Neither reading of the first position is near the start, or both are.
    Undecided,
}

/// The order the first position of a link's geometry is written in, `first`
/// as written, judged against where the link starts, `start` as latitude and
/// longitude: the reading within [`NEAR_THE_START_METRES`] of the start.
pub fn axis_order(first: (f64, f64), start: (f64, f64)) -> AxisOrder {
    reading_order([first, (first.1, first.0)], start)
}

/// Which of two readings of a link geometry's first position, each as
/// latitude and longitude, lies within [`NEAR_THE_START_METRES`] of the
/// link's start: the reading its reference system states
/// ([`AxisOrder::LatitudeFirst`] — as WGS 84 states it), the other one
/// ([`AxisOrder::LongitudeFirst`]), or neither or both.
pub fn reading_order(readings: [(f64, f64); 2], start: (f64, f64)) -> AxisOrder {
    let near = |reading: (f64, f64)| metres_between(reading, start) <= NEAR_THE_START_METRES;
    match (near(readings[0]), near(readings[1])) {
        (true, false) => AxisOrder::LatitudeFirst,
        (false, true) => AxisOrder::LongitudeFirst,
        _ => AxisOrder::Undecided,
    }
}

/// The distance between two positions on the earth, latitude and longitude
/// each, in metres (the haversine on a sphere of the earth's mean radius).
pub fn metres_between(a: (f64, f64), b: (f64, f64)) -> f64 {
    const EARTH_RADIUS_METRES: f64 = 6_371_008.8;
    let (a_latitude, b_latitude) = (a.0.to_radians(), b.0.to_radians());
    let latitude = (b.0 - a.0).to_radians();
    let longitude = (b.1 - a.1).to_radians();
    let h = (latitude / 2.0).sin().powi(2)
        + a_latitude.cos() * b_latitude.cos() * (longitude / 2.0).sin().powi(2);
    2.0 * EARTH_RADIUS_METRES * h.sqrt().min(1.0).asin()
}

/// The first position of a GML geometry as written — the first two numbers
/// of its first `posList` or `pos` —, if it states one.
pub fn first_position(geometry: &str) -> Option<(f64, f64)> {
    let mut reader = Reader::from_str(geometry);
    let mut in_positions = false;
    loop {
        match reader.read_event().ok()? {
            Event::Start(element) => {
                in_positions = matches!(element.local_name().as_ref(), "posList" | "pos");
            }
            Event::Text(text) if in_positions => {
                let text = text.xml10_content();
                let mut numbers = text.split_ascii_whitespace().map(str::parse::<f64>);
                return match (numbers.next(), numbers.next()) {
                    (Some(Ok(a)), Some(Ok(b))) => Some((a, b)),
                    _ => None,
                };
            }
            Event::End(_) => in_positions = false,
            Event::Eof => return None,
            _ => {}
        }
    }
}

/// A GML geometry as written: the reference system it names, and its
/// positions, each of `dimension` numbers.
#[derive(Debug, Clone, PartialEq)]
pub struct Geometry {
    /// The `srsName` of the geometry, or of its first position list or
    /// position, if one names a system.
    pub srs_name: Option<String>,
    /// How many numbers make a position (`srsDimension`, two where none is stated).
    pub dimension: usize,
    /// The numbers of every position, in order.
    pub numbers: Vec<f64>,
}

impl Geometry {
    /// The first two numbers of each position, as written.
    pub fn positions(&self) -> impl Iterator<Item = (f64, f64)> + '_ {
        self.numbers
            .chunks_exact(self.dimension)
            .map(|position| (position[0], position[1]))
    }
}

/// A GML geometry — a `LineString` with a `posList` or `pos` elements, or a
/// `pos` alone — as written, if it states a position and every number of it
/// is one.
pub fn geometry(xml: &str) -> Option<Geometry> {
    let mut reader = Reader::from_str(xml);
    let mut geometry = Geometry {
        srs_name: None,
        dimension: 2,
        numbers: Vec::new(),
    };
    let mut in_positions = false;
    loop {
        match reader.read_event().ok()? {
            Event::Start(element) | Event::Empty(element) => {
                in_positions = matches!(element.local_name().as_ref(), "posList" | "pos");
                for attribute in element.attributes().flatten() {
                    let value = attribute.normalized_value(XmlVersion::Explicit1_0).ok()?;
                    match attribute.key.local_name().as_ref() {
                        "srsName" if geometry.srs_name.is_none() => {
                            geometry.srs_name = Some(value.into_owned());
                        }
                        "srsDimension" => geometry.dimension = value.trim().parse().ok()?,
                        _ => {}
                    }
                }
            }
            Event::Text(text) if in_positions => {
                for number in text.xml10_content().split_ascii_whitespace() {
                    geometry.numbers.push(number.parse().ok()?);
                }
            }
            Event::End(_) => in_positions = false,
            Event::Eof => break,
            _ => {}
        }
    }
    let whole = geometry.dimension >= 2 && geometry.numbers.len() % geometry.dimension == 0;
    (whole && !geometry.numbers.is_empty()).then_some(geometry)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_degree_of_latitude_is_about_111_kilometres() {
        let metres = metres_between((46.0, 14.0), (47.0, 14.0));
        assert!((111_000.0..111_400.0).contains(&metres), "{metres}");
        assert_eq!(metres_between((46.0, 14.0), (46.0, 14.0)), 0.0);
    }

    #[test]
    fn the_first_position_is_read_from_a_position_list_or_a_position() {
        let list = r#"<gml:LineString xmlns:gml="http://www.opengis.net/gml/3.2" gml:id="l"><gml:posList srsDimension="2">14.296652 48.334134 14.3 48.34</gml:posList></gml:LineString>"#;
        assert_eq!(first_position(list), Some((14.296652, 48.334134)));
        let points = r#"<LineString xmlns="http://www.opengis.net/gml/3.2"><pos>48.1 14.2</pos><pos>48.2 14.3</pos></LineString>"#;
        assert_eq!(first_position(points), Some((48.1, 14.2)));
        assert_eq!(first_position("<LineString/>"), None);
        assert_eq!(
            first_position("<LineString><posList>1</posList></LineString>"),
            None
        );
    }

    #[test]
    fn a_geometry_is_read_with_its_system_and_every_position() {
        let list = r#"<gml:LineString xmlns:gml="http://www.opengis.net/gml/3.2" gml:id="l" srsName="EPSG:2056"><gml:posList srsDimension="3">2600000 1200000 540 2600100 1200050 541</gml:posList></gml:LineString>"#;
        let read = geometry(list).expect("a geometry");
        assert_eq!(read.srs_name.as_deref(), Some("EPSG:2056"));
        assert_eq!(
            read.positions().collect::<Vec<_>>(),
            [(2_600_000.0, 1_200_000.0), (2_600_100.0, 1_200_050.0)]
        );
        let pos = r#"<gml:pos xmlns:gml="http://www.opengis.net/gml/3.2" srsName="urn:ogc:def:crs:EPSG::31287">350000.5 600000.25</gml:pos>"#;
        let read = geometry(pos).expect("a position");
        assert_eq!(
            read.srs_name.as_deref(),
            Some("urn:ogc:def:crs:EPSG::31287")
        );
        assert_eq!(
            read.positions().collect::<Vec<_>>(),
            [(350_000.5, 600_000.25)]
        );
        assert_eq!(
            geometry("<LineString><posList>1 2 3</posList></LineString>"),
            None
        );
        assert_eq!(
            geometry("<LineString><posList>1 x</posList></LineString>"),
            None
        );
        assert_eq!(geometry("<LineString/>"), None);
    }

    #[test]
    fn a_position_near_the_start_both_ways_is_undecided() {
        // On the diagonal latitude and longitude read alike.
        assert_eq!(axis_order((45.0, 45.0), (45.0, 45.0)), AxisOrder::Undecided);
    }
}
