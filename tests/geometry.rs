//! A geometry as a delivery writes it: numbers that are no coordinates are
//! no position, and a geometry built by hand with too few numbers to a
//! position gives none rather than a panic.

use netex_rs::geometry::{first_position, geometry, Geometry};

#[test]
fn a_number_that_is_no_coordinate_makes_no_position() {
    for numbers in ["NaN 48.1", "inf 48.1", "48.1 -inf", "1e999 48.1"] {
        let list = format!("<LineString><posList>{numbers} 48.2 9.3</posList></LineString>");
        assert_eq!(geometry(&list), None, "{numbers}");
        assert_eq!(first_position(&list), None, "{numbers}");
    }
    let list = "<LineString><posList>48.1 9.2 48.2 9.3</posList></LineString>";
    assert_eq!(first_position(list), Some((48.1, 9.2)));
}

#[test]
fn a_geometry_of_fewer_than_two_numbers_to_a_position_has_no_positions() {
    for dimension in [0, 1] {
        let built = Geometry {
            srs_name: None,
            dimension,
            numbers: vec![48.1, 9.2, 48.2, 9.3],
        };
        assert_eq!(built.positions().count(), 0, "dimension {dimension}");
    }
}
