//! The link sequence projection a journey names: its course drawn on a map
//! (`LinkSequenceProjectionRef`), as Bordeaux's deliveries name one for each
//! journey.

use netex_rs::model::ServiceJourney;
use netex_rs::Limits;

#[test]
fn the_projection_a_journey_names_is_typed() {
    let xml = concat!(
        r#"<ServiceJourney xmlns="http://www.netex.org.uk/netex" id="J" version="any">"#,
        r#"<LinkSequenceProjectionRef ref="P" version="any"/>"#,
        r#"<JourneyPatternRef ref="JP" version="any"/>"#,
        r#"</ServiceJourney>"#
    );
    let journey: ServiceJourney = netex_rs::read_element(xml, "ServiceJourney", &Limits::default())
        .expect("the journey reads");
    let projection = journey
        .link_sequence_projection_ref
        .as_ref()
        .and_then(|r| r.r#ref.as_ref())
        .map(|id| id.as_str());
    assert_eq!(projection, Some("P"));
    assert!(journey.unprocessed.is_empty(), "nothing carried untyped");
    let written = netex_rs::write_element(&journey, "ServiceJourney");
    let again: ServiceJourney =
        netex_rs::read_element(&written, "ServiceJourney", &Limits::default())
            .expect("the written journey reads");
    assert_eq!(again, journey, "written back as it was read");
}
