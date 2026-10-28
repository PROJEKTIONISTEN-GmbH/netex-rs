//! An area on the map: a box of longitudes and latitudes, or the polygons of
//! a GeoJSON geometry.

use serde_json::Value;

/// A ring of a polygon: its corners as longitude and latitude, WGS 84.
type Ring = Vec<(f64, f64)>;

/// An area, in WGS 84.
#[derive(Debug, Clone, PartialEq)]
pub enum Area {
    /// The box between two longitudes and two latitudes.
    Box {
        /// Its western edge.
        west: f64,
        /// Its southern edge.
        south: f64,
        /// Its eastern edge.
        east: f64,
        /// Its northern edge.
        north: f64,
    },
    /// Polygons, each its outer ring and its holes: a point is in a polygon
    /// inside an odd number of its rings.
    Polygons(Vec<Vec<Ring>>),
}

impl Area {
    /// The box `west,south,east,north`.
    pub fn from_box(text: &str) -> Result<Area, String> {
        let numbers: Vec<f64> = text
            .split(',')
            .map(|part| part.trim().parse::<f64>())
            .collect::<Result<_, _>>()
            .map_err(|_| format!("`{text}` is no box: west,south,east,north in degrees"))?;
        match numbers[..] {
            [west, south, east, north] if west < east && south < north => Ok(Area::Box {
                west,
                south,
                east,
                north,
            }),
            _ => Err(format!(
                "`{text}` is no box: west,south,east,north in degrees, west of east and south of north"
            )),
        }
    }

    /// The polygons of a GeoJSON text: a `Polygon` or `MultiPolygon`, alone,
    /// as a feature's geometry or as the geometries of a feature collection.
    pub fn from_geojson(text: &str) -> Result<Area, String> {
        let value: Value =
            serde_json::from_str(text).map_err(|error| format!("no GeoJSON: {error}"))?;
        let mut polygons = Vec::new();
        collect(&value, &mut polygons)?;
        if polygons.is_empty() {
            return Err("the GeoJSON holds no polygon".to_owned());
        }
        Ok(Area::Polygons(polygons))
    }

    /// Whether the point at `longitude`, `latitude` is in the area; on its
    /// edge counts as in a box.
    pub fn contains(&self, longitude: f64, latitude: f64) -> bool {
        match self {
            Area::Box {
                west,
                south,
                east,
                north,
            } => (*west..=*east).contains(&longitude) && (*south..=*north).contains(&latitude),
            Area::Polygons(polygons) => polygons.iter().any(|rings| {
                rings
                    .iter()
                    .filter(|ring| crosses(ring, longitude, latitude))
                    .count()
                    % 2
                    == 1
            }),
        }
    }
}

/// Whether a ray from the point to the east crosses `ring` an odd number of
/// times.
fn crosses(ring: &[(f64, f64)], x: f64, y: f64) -> bool {
    let mut inside = false;
    let mut previous = match ring.last() {
        Some(&corner) => corner,
        None => return false,
    };
    for &(cx, cy) in ring {
        let (px, py) = previous;
        if (cy > y) != (py > y) && x < (px - cx) * (y - cy) / (py - cy) + cx {
            inside = !inside;
        }
        previous = (cx, cy);
    }
    inside
}

fn collect(value: &Value, polygons: &mut Vec<Vec<Ring>>) -> Result<(), String> {
    match value["type"].as_str() {
        Some("FeatureCollection") => {
            for feature in value["features"].as_array().into_iter().flatten() {
                collect(feature, polygons)?;
            }
        }
        Some("Feature") => collect(&value["geometry"], polygons)?,
        Some("Polygon") => polygons.push(rings(&value["coordinates"])?),
        Some("MultiPolygon") => {
            for polygon in value["coordinates"].as_array().into_iter().flatten() {
                polygons.push(rings(polygon)?);
            }
        }
        Some(other) => return Err(format!("a GeoJSON {other} is no area")),
        None => return Err("a GeoJSON object without a type".to_owned()),
    }
    Ok(())
}

fn rings(value: &Value) -> Result<Vec<Ring>, String> {
    let bad = || "a polygon's coordinates are rings of [longitude, latitude]".to_owned();
    value
        .as_array()
        .ok_or_else(bad)?
        .iter()
        .map(|ring| {
            ring.as_array()
                .ok_or_else(bad)?
                .iter()
                .map(|corner| match corner.as_array().map(Vec::as_slice) {
                    Some([longitude, latitude, ..]) => Ok((
                        longitude.as_f64().ok_or_else(bad)?,
                        latitude.as_f64().ok_or_else(bad)?,
                    )),
                    _ => Err(bad()),
                })
                .collect()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_box_holds_what_lies_between_its_edges() {
        let area = Area::from_box("9.6, 52.3, 9.9, 52.5").expect("a box");
        assert!(area.contains(9.7320, 52.3760));
        assert!(area.contains(9.6, 52.3));
        assert!(!area.contains(9.5, 52.4));
        assert!(Area::from_box("9.9,52.3,9.6,52.5").is_err());
        assert!(Area::from_box("9.6,52.3,9.9").is_err());
    }

    #[test]
    fn a_polygon_holds_what_lies_inside_it_and_not_in_its_holes() {
        let geojson = r#"{"type": "Feature", "properties": {}, "geometry": {
            "type": "MultiPolygon", "coordinates": [
              [[[0, 0], [4, 0], [4, 4], [0, 4], [0, 0]], [[1, 1], [2, 1], [2, 2], [1, 2], [1, 1]]],
              [[[10, 10], [11, 10], [10, 11], [10, 10]]]
            ]}}"#;
        let area = Area::from_geojson(geojson).expect("an area");
        assert!(area.contains(3.0, 3.0));
        assert!(!area.contains(1.5, 1.5), "in a hole");
        assert!(area.contains(10.2, 10.2));
        assert!(!area.contains(10.9, 10.9));
        assert!(!area.contains(5.0, 5.0));
        assert!(Area::from_geojson(r#"{"type": "Point", "coordinates": [0, 0]}"#).is_err());
    }
}
