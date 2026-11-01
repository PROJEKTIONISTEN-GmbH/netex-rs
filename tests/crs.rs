//! Positions in national reference systems turned into WGS 84 (the `crs`
//! feature), against positions the Swiss and Austrian registers state and
//! PROJ with the national grids (`tests/fixtures/crs/reference.tsv`); and
//! the index reading them — without the feature, as it always has.

mod support;

#[cfg(feature = "crs")]
use std::collections::BTreeMap;

#[cfg(feature = "crs")]
use netex_rs::crs::{Axes, ReferenceSystem};
#[cfg(feature = "crs")]
use netex_rs::geometry::metres_between;
use netex_rs::index::{Concern, Index, IndexBuilder};
use netex_rs::stream::{Kind, ReadOptions, Selection};

#[cfg(feature = "crs")]
/// The worst distance from the reference, in metres, of each system's points.
fn worst_by_system() -> BTreeMap<u32, (f64, usize)> {
    let path = support::fixtures_dir().join("crs").join("reference.tsv");
    let text = std::fs::read_to_string(&path).expect("the reference points");
    let mut worst: BTreeMap<u32, (f64, usize)> = BTreeMap::new();
    for line in text.lines().filter(|line| !line.starts_with('#')) {
        let fields: Vec<&str> = line.split('\t').collect();
        let number = |i: usize| fields[i].parse::<f64>().expect("a number");
        let epsg: u32 = fields[0].parse().expect("an EPSG code");
        let system = ReferenceSystem::from_srs_name(&format!("urn:ogc:def:crs:EPSG::{epsg}"))
            .expect("a system the crate knows");
        let got = system.to_wgs84(number(1), number(2));
        let distance = metres_between(got, (number(3), number(4)));
        let entry = worst.entry(epsg).or_insert((0.0, 0));
        entry.0 = entry.0.max(distance);
        entry.1 += 1;
    }
    worst
}

#[cfg(feature = "crs")]
#[test]
fn every_system_lands_within_a_metre_of_the_reference() {
    let worst = worst_by_system();
    for (epsg, (metres, points)) in &worst {
        println!("EPSG:{epsg}: {points} points, worst {metres:.3} m");
        assert!(*metres < 1.0, "EPSG:{epsg} is {metres:.3} m off at worst");
    }
    assert_eq!(
        worst.keys().copied().collect::<Vec<_>>(),
        [2056, 2154, 21781, 25832, 25833, 28992, 31258, 31287]
    );
    // A projection of ETRS89 alone is exact to the precision the reference is
    // written in — millimetres, eight decimals of a degree —, and so is the
    // Swiss register's own pair of positions: a datum correction is the only
    // error left.
    for epsg in [2056, 2154, 25832, 25833] {
        assert!(
            worst[&epsg].0 < 0.005,
            "EPSG:{epsg}: {:.4} m",
            worst[&epsg].0
        );
    }
}

/// Quays of the Dutch stop register (Centraal Haltebestand, CC0, as the
/// national access point published it on 31 October 2026): each code, its
/// position as the register writes it in RD New — whole metres —, and where
/// RDNAPTRANS2018, the official transformation through its correction grid,
/// puts it in ETRS89 (PROJ 9.5.1 with `nl_nsgi_rdtrans2018.tif`). The centre
/// of the system at Amersfoort, the far north, south, west and east of the
/// register — a stop in Belgium and one in Germany among them — and the quay
/// the crate places farthest from the official position of all 74 736 stops
/// of the register (Ihrhove, in Germany).
#[cfg(feature = "crs")]
const DUTCH_REGISTER: [(&str, f64, f64, f64, f64); 6] = [
    ("NL:Q:50420143", 154776.0, 462915.0, 52.15440887, 5.38393048),
    ("NL:Q:29300201", 208085.0, 612579.0, 53.49670915, 6.18708768),
    ("NL:Q:66451040", 198351.0, 307335.0, 50.75433277, 6.00148306),
    ("NL:Q:79840130", 13995.0, 371351.0, 51.31382964, 3.36459557),
    ("NL:Q:13150004", 293760.0, 584371.0, 53.22792497, 7.46518613),
    ("NL:Q:13150003", 292554.0, 576754.0, 53.15982900, 7.44388202),
];

/// RD New is read by the EPSG Helmert transformation alone, without the
/// correction grid: within a quarter of a metre of RDNAPTRANS2018 at every
/// quay of the register — the register states its positions to the whole
/// metre, so half a metre is lost to its rounding before any transformation.
#[cfg(feature = "crs")]
#[test]
fn a_quay_of_the_dutch_register_lands_where_rdnaptrans2018_puts_it() {
    let rd = ReferenceSystem::from_srs_name("urn:ogc:def:crs:EPSG::28992").expect("RD New");
    assert_eq!(rd.axes(), Axes::EastingNorthing);
    for (code, x, y, latitude, longitude) in DUTCH_REGISTER {
        let metres = metres_between(rd.to_wgs84(x, y), (latitude, longitude));
        assert!(metres < 0.25, "{code} is {metres:.3} m off");
    }
}

#[cfg(feature = "crs")]
#[test]
fn an_srs_name_names_its_system_in_any_of_the_usual_forms() {
    for name in [
        "EPSG:2056",
        "epsg:2056",
        "urn:ogc:def:crs:EPSG::2056",
        "urn:ogc:def:crs:EPSG:6.6:2056",
        "http://www.opengis.net/def/crs/EPSG/0/2056",
    ] {
        let system = ReferenceSystem::from_srs_name(name).expect(name);
        assert_eq!(
            (system.epsg(), system.axes()),
            (2056, Axes::EastingNorthing)
        );
    }
    let austria = ReferenceSystem::from_srs_name("EPSG:31287").expect("MGI Lambert");
    assert_eq!(austria.axes(), Axes::NorthingEasting);
    let wgs84 = ReferenceSystem::from_srs_name("urn:ogc:def:crs:EPSG::4326").expect("WGS 84");
    assert_eq!(wgs84, ReferenceSystem::WGS84);
    assert!(wgs84.is_geographic());
    assert_eq!(wgs84.to_wgs84(47.1, 8.2), (47.1, 8.2));
    // The legacy GML form and CRS84 write longitude first.
    for name in [
        "http://www.opengis.net/gml/srs/epsg.xml#4326",
        "urn:ogc:def:crs:OGC:1.3:CRS84",
    ] {
        let system = ReferenceSystem::from_srs_name(name).expect(name);
        assert_eq!(system.axes(), Axes::LongitudeLatitude);
        assert_eq!(system.to_wgs84(8.2, 47.1), (47.1, 8.2));
    }
    for unknown in [
        "EPSG:3857",
        "EPSG:27700",
        "urn:ogc:def:crs:EPSG::",
        "local",
        "",
    ] {
        assert_eq!(ReferenceSystem::from_srs_name(unknown), None, "{unknown}");
    }
}

#[cfg(feature = "crs")]
fn location(xml: &str) -> netex_rs::model::Location {
    netex_rs::read_element(xml, "Location", &netex_rs::Limits::default()).expect("a location")
}

#[cfg(feature = "crs")]
#[test]
fn a_location_is_where_its_degrees_or_its_position_in_its_system_say() {
    use netex_rs::crs::{location_in_wgs84, UnknownSystem};
    let near = |got: Option<(f64, f64)>, expected: (f64, f64)| {
        let got = got.expect("a position");
        assert!(
            metres_between(got, expected) < 1.0,
            "{got:?} is not {expected:?}"
        );
    };
    // Degrees are degrees, whatever system the frame names.
    let degrees = location(
        r#"<Location xmlns="http://www.netex.org.uk/netex"><Longitude>8.31</Longitude><Latitude>47.04</Latitude></Location>"#,
    );
    assert_eq!(
        location_in_wgs84(&degrees, Some("EPSG:2056")),
        Ok(Some((47.04, 8.31)))
    );
    // A GML position in the system it names; else the location's; else the frame's.
    let lucerne = (47.036_910_73, 8.312_871_83);
    let own = location(
        r#"<Location xmlns="http://www.netex.org.uk/netex" xmlns:gml="http://www.opengis.net/gml/3.2"><gml:pos srsName="EPSG:2056">2666444.676 1209911.277</gml:pos></Location>"#,
    );
    near(location_in_wgs84(&own, Some("EPSG:4326")).unwrap(), lucerne);
    let of_location = location(
        r#"<Location xmlns="http://www.netex.org.uk/netex" xmlns:gml="http://www.opengis.net/gml/3.2" srsName="EPSG:21781"><gml:pos>666443.893 209911.387</gml:pos></Location>"#,
    );
    near(location_in_wgs84(&of_location, None).unwrap(), lucerne);
    let of_frame = location(
        r#"<Location xmlns="http://www.netex.org.uk/netex"><Coordinates>2666444.676 1209911.277</Coordinates></Location>"#,
    );
    near(
        location_in_wgs84(&of_frame, Some("urn:ogc:def:crs:EPSG::2056")).unwrap(),
        lucerne,
    );
    // A system the crate does not know is said, not guessed.
    assert_eq!(
        location_in_wgs84(&of_frame, Some("EPSG:3857")),
        Err(UnknownSystem("EPSG:3857".to_owned()))
    );
    let nothing = location(r#"<Location xmlns="http://www.netex.org.uk/netex"/>"#);
    assert_eq!(location_in_wgs84(&nothing, Some("EPSG:2056")), Ok(None));
}

/// A Swiss-style document whose positions are in national systems: a stop
/// place in LV95 by its own `srsName`, a quay in MGI Lambert by its frame's
/// default, a stop point in a system no table holds, and a link in LV95 —
/// one written as LV95 states it, one northing first.
const NATIONAL: &str = r#"<PublicationDelivery xmlns="http://www.netex.org.uk/netex" xmlns:gml="http://www.opengis.net/gml/3.2" version="1.10">
  <PublicationTimestamp>2026-10-14T00:00:00</PublicationTimestamp>
  <ParticipantRef>test</ParticipantRef>
  <dataObjects><CompositeFrame id="c" version="1">
    <FrameDefaults><DefaultLocationSystem>EPSG:31287</DefaultLocationSystem></FrameDefaults>
    <frames>
      <SiteFrame id="g" version="1"><stopPlaces>
        <StopPlace id="lucerne" version="1"><Name>Luzern</Name>
          <Centroid><Location><gml:pos srsName="EPSG:2056">2666444.676 1209911.277</gml:pos></Location></Centroid>
          <quays><Quay id="lucerne:1" version="1"><Name>Luzern 1</Name>
            <Centroid><Location><gml:pos>350000.0 400000.0</gml:pos></Location></Centroid>
          </Quay></quays>
        </StopPlace>
      </stopPlaces></SiteFrame>
      <ServiceFrame id="s" version="1">
        <FrameDefaults><DefaultLocationSystem>EPSG:2056</DefaultLocationSystem></FrameDefaults>
        <scheduledStopPoints>
          <ScheduledStopPoint id="A" version="1"><Name>A</Name><Location><gml:pos>2666444.676 1209911.277</gml:pos></Location></ScheduledStopPoint>
          <ScheduledStopPoint id="Z" version="1"><Name>Z</Name><Location srsName="EPSG:3857"><gml:pos>925000 5945000</gml:pos></Location></ScheduledStopPoint>
        </scheduledStopPoints>
        <serviceLinks>
          <ServiceLink id="as-stated" version="1"><FromPointRef ref="A" version="1"/><ToPointRef ref="Z" version="1"/>
            <gml:LineString gml:id="l1"><gml:posList>2666444.676 1209911.277 2666500 1209950</gml:posList></gml:LineString></ServiceLink>
          <ServiceLink id="northing-first" version="1"><FromPointRef ref="A" version="1"/><ToPointRef ref="Z" version="1"/>
            <gml:LineString gml:id="l2"><gml:posList>1209911.277 2666444.676 1209950 2666500</gml:posList></gml:LineString></ServiceLink>
        </serviceLinks>
      </ServiceFrame>
    </frames>
  </CompositeFrame></dataObjects>
</PublicationDelivery>"#;

/// The index of [`NATIONAL`] built every way: the full model, the lean
/// stream, and the lean stream handing on to a sink; all three the same.
fn national() -> Index {
    let options = ReadOptions::new(Selection::routing().with(Kind::ServiceLink));
    let mut full = IndexBuilder::new();
    full.add(
        &netex_rs::read(NATIONAL).expect("the document reads"),
        "national.xml",
    );
    let mut lean = IndexBuilder::new();
    lean.add_streamed(NATIONAL, &options, "national.xml")
        .expect("the document streams");
    /// Reads the frame defaults it is handed itself, as a caller might.
    struct Defaults(usize);
    impl<'i> netex_rs::stream::Sink<'i> for Defaults {
        fn entity(
            &mut self,
            _: netex_rs::stream::Entity,
            _: &netex_rs::stream::Scope,
        ) -> netex_rs::Result<()> {
            Ok(())
        }
        fn skipped(
            &mut self,
            cursor: &mut netex_rs::xml::Cursor<'i>,
            element: &netex_rs::xml::Child<'i>,
            _: netex_rs::stream::Skipped<'i>,
            _: &netex_rs::stream::Scope,
        ) -> netex_rs::Result<()> {
            if element.local == "FrameDefaults" {
                use netex_rs::model::Element;
                netex_rs::model::FrameDefaults::read(cursor, element)?;
                self.0 += 1;
                return Ok(());
            }
            cursor.skip_over(element)
        }
    }
    let mut handing_on = IndexBuilder::new();
    let mut defaults = Defaults(0);
    handing_on
        .add_streamed_with(NATIONAL, &options, "national.xml", &mut defaults)
        .expect("the document streams");
    assert_eq!(
        defaults.0, 2,
        "the sink reads the frame defaults the index read too"
    );
    let (full, lean, handing_on) = (full.finish(), lean.finish(), handing_on.finish());
    assert!(lean == full, "the lean index differs from the full one");
    assert!(
        handing_on == full,
        "the index handing on differs from the full one"
    );
    full
}

fn position(index: &Index, stop: &str) -> Option<(f64, f64)> {
    let stop = index.stop(index.stop_by_id(stop).expect("the stop"));
    stop.latitude.zip(stop.longitude)
}

#[cfg(feature = "crs")]
fn findings(index: &Index, concern: Concern) -> Vec<String> {
    index
        .findings()
        .filter(|finding| finding.concern == concern)
        .map(|finding| finding.to_string())
        .collect()
}

#[cfg(feature = "crs")]
#[test]
fn the_index_reads_positions_in_the_system_they_or_their_frame_name() {
    let index = national();
    let lucerne = (47.036_910_73, 8.312_871_83);
    let near = |got: Option<(f64, f64)>, expected: (f64, f64)| {
        let got = got.expect("a position");
        assert!(
            metres_between(got, expected) < 1.0,
            "{got:?} is not {expected:?}"
        );
    };
    near(position(&index, "lucerne"), lucerne);
    // MGI Lambert 350 000 / 400 000, written northing first by the frame's default:
    // easting 400 000 is the false easting, the meridian 13°20' E.
    let quay = position(&index, "lucerne:1").expect("the quay's position");
    assert!((quay.1 - (13.0 + 20.0 / 60.0)).abs() < 1e-3, "{quay:?}");
    let point = index.stop_point(index.stop_point_by_id("A").expect("A"));
    near(point.latitude.zip(point.longitude), lucerne);
    let z = index.stop_point(index.stop_point_by_id("Z").expect("Z"));
    assert_eq!((z.latitude, z.longitude), (None, None));
    assert_eq!(
        findings(&index, Concern::UnknownReferenceSystem),
        ["position in a reference system the crate does not know, left without a position: ScheduledStopPoint Z in national.xml (EPSG:3857)"]
    );
    // The link written as LV95 states it is no finding; the other one is.
    assert_eq!(
        findings(&index, Concern::AxisOrderSwapped),
        ["link geometry written longitude first: ServiceLink northing-first in national.xml"]
    );
    assert_eq!(index.finding_count(Concern::AxisOrderUndecided), 0);
}

#[cfg(not(feature = "crs"))]
#[test]
fn without_the_feature_a_position_is_its_latitude_and_longitude_alone() {
    let index = national();
    assert_eq!(position(&index, "lucerne"), None);
    assert_eq!(position(&index, "lucerne:1"), None);
    assert_eq!(index.finding_count(Concern::UnknownReferenceSystem), 0);
    // Read as latitude and longitude, neither reading of a link lies near its
    // start, which has no position either.
    assert_eq!(index.finding_count(Concern::AxisOrderSwapped), 0);
    assert_eq!(index.finding_count(Concern::AxisOrderUndecided), 2);
}
