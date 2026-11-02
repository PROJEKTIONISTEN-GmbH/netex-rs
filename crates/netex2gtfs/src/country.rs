//! The countries a stop can lie in, each with its time zone: the member
//! states of the European Union and of EFTA, the United Kingdom and the small
//! states among them, their borders Natural Earth's (public domain), cut to
//! Europe and simplified to 0.005° — some 500 m — by `tools/countries.py`.

use std::sync::OnceLock;

/// The borders, as `tools/countries.py` writes them.
const BORDERS: &str = include_str!("../data/countries.txt");

/// The positions of a ring are written in units of this many degrees.
const QUANTUM: f64 = 1e-4;

/// A country of the table.
#[derive(Debug)]
pub(crate) struct Country {
    /// Its ISO 3166 code.
    pub(crate) code: &'static str,
    /// The time zone of its European part.
    pub(crate) zone: &'static str,
    rings: Vec<Ring>,
}

/// A ring of a country's border: an island, a mainland, or a hole in one.
#[derive(Debug)]
struct Ring {
    /// South, west, north and east, in degrees.
    bounds: [f64; 4],
    /// Latitude and longitude, in degrees.
    positions: Vec<(f64, f64)>,
}

impl Ring {
    /// Whether the ring holds a position: the number of its edges a ray from
    /// the position eastward crosses is odd.
    fn holds(&self, (latitude, longitude): (f64, f64)) -> bool {
        let [south, west, north, east] = self.bounds;
        if latitude < south || latitude > north || longitude < west || longitude > east {
            return false;
        }
        let mut inside = false;
        let mut previous = self.positions[self.positions.len() - 1];
        for &here in &self.positions {
            let ((lat_a, lon_a), (lat_b, lon_b)) = (previous, here);
            if (lat_a > latitude) != (lat_b > latitude) {
                let crossing = lon_a + (latitude - lat_a) / (lat_b - lat_a) * (lon_b - lon_a);
                if longitude < crossing {
                    inside = !inside;
                }
            }
            previous = here;
        }
        inside
    }
}

impl Country {
    /// Whether the country holds a position: an odd number of its rings
    /// hold it — a hole is a ring within a ring.
    fn holds(&self, position: (f64, f64)) -> bool {
        self.rings
            .iter()
            .filter(|ring| ring.holds(position))
            .count()
            % 2
            == 1
    }
}

/// Every country of the table, in the order of their codes.
fn countries() -> &'static [Country] {
    static COUNTRIES: OnceLock<Vec<Country>> = OnceLock::new();
    COUNTRIES.get_or_init(|| read(BORDERS))
}

/// The table as `tools/countries.py` writes it: a line naming a country and
/// its zone, then a line for each ring of its border.
fn read(text: &'static str) -> Vec<Country> {
    let mut countries: Vec<Country> = Vec::new();
    for line in text.lines().filter(|line| !line.starts_with('#')) {
        if line.starts_with(|c: char| c.is_ascii_uppercase()) {
            let (code, zone) = line.split_once(' ').expect("a country and its zone");
            countries.push(Country {
                code,
                zone,
                rings: Vec::new(),
            });
            continue;
        }
        let mut last = (0_i64, 0_i64);
        let positions: Vec<(f64, f64)> = line
            .split(' ')
            .map(|step| {
                let (latitude, longitude) = step.split_once(',').expect("a position");
                let parse = |n: &str| n.parse::<i64>().expect("a whole number");
                last = (last.0 + parse(latitude), last.1 + parse(longitude));
                (last.0 as f64 * QUANTUM, last.1 as f64 * QUANTUM)
            })
            .collect();
        let bounds = positions.iter().fold(
            [f64::MAX, f64::MAX, f64::MIN, f64::MIN],
            |[south, west, north, east], &(latitude, longitude)| {
                [
                    south.min(latitude),
                    west.min(longitude),
                    north.max(latitude),
                    east.max(longitude),
                ]
            },
        );
        countries
            .last_mut()
            .expect("a ring follows its country")
            .rings
            .push(Ring { bounds, positions });
    }
    countries
}

/// The country of the table a position lies in, latitude and longitude in
/// degrees; where simplified borders overlap, the one whose code sorts first.
pub(crate) fn country_of(position: (f64, f64)) -> Option<&'static Country> {
    countries().iter().find(|country| country.holds(position))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_place_lies_in_its_country() {
        for (place, position, code) in [
            ("Brussels", (50.8467, 4.3525), "BE"),
            ("Antwerp", (51.2194, 4.4025), "BE"),
            ("Lille", (50.6292, 3.0573), "FR"),
            ("Maastricht", (50.8514, 5.6910), "NL"),
            ("Aachen", (50.7753, 6.0839), "DE"),
            ("Luxembourg", (49.6116, 6.1319), "LU"),
            ("Vienna", (48.2082, 16.3738), "AT"),
            ("Zurich", (47.3769, 8.5417), "CH"),
            ("Vaduz", (47.1410, 9.5209), "LI"),
            ("Oslo", (59.9139, 10.7522), "NO"),
            ("Lillehammer", (61.1153, 10.4662), "NO"),
            ("Helsinki", (60.1699, 24.9384), "FI"),
            ("Copenhagen", (55.6761, 12.5683), "DK"),
            ("Prague", (50.0755, 14.4378), "CZ"),
            ("Bolzano", (46.4983, 11.3548), "IT"),
            ("Vatican City", (41.9029, 12.4534), "VA"),
            ("Corte", (42.3061, 9.1497), "FR"),
        ] {
            assert_eq!(
                country_of(position).map(|country| country.code),
                Some(code),
                "{place}"
            );
        }
    }

    #[test]
    fn the_sea_and_a_country_the_table_does_not_hold_lie_in_none() {
        assert!(country_of((54.5, 4.0)).is_none(), "the North Sea");
        // At 1:10m the coast runs a little off: Narvik's shore lies in its fjord.
        assert!(country_of((68.4385, 17.4272)).is_none(), "Narvik");
        assert!(country_of((44.7866, 20.4489)).is_none(), "Belgrade");
        assert!(country_of((40.7128, -74.0060)).is_none(), "New York");
    }

    #[test]
    fn the_table_holds_each_country_once_with_its_zone() {
        let countries = countries();
        assert_eq!(countries.len(), 34);
        assert!(countries.windows(2).all(|pair| pair[0].code < pair[1].code));
        for country in countries {
            assert!(
                country.zone.starts_with("Europe/"),
                "{}: {}",
                country.code,
                country.zone
            );
            assert!(!country.rings.is_empty(), "{}", country.code);
        }
        let belgium = countries.iter().find(|c| c.code == "BE").expect("Belgium");
        assert_eq!(belgium.zone, "Europe/Brussels");
    }
}
