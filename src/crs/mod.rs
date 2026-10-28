//! Positions in national reference systems, turned into WGS 84 — the `crs`
//! feature.
//!
//! A NeTEx location is latitude and longitude in WGS 84, or a GML position in
//! the reference system its `srsName` names, else the one its frame's
//! `DefaultLocationSystem` names. This module knows the systems the national
//! deliveries around the crate's own use, and turns a position written in one
//! of them into latitude and longitude, in pure Rust:
//!
//! | EPSG | System | Written | Projection | To WGS 84 |
//! |---|---|---|---|---|
//! | 4326, 4258 | WGS 84, ETRS89 | latitude, longitude | — | — |
//! | 2056 | CH1903+ / LV95 | easting, northing | Swiss oblique Mercator | translation |
//! | 21781 | CH1903 / LV03 | easting, northing | Swiss oblique Mercator | translation, then a correction |
//! | 31287 | MGI / Austria Lambert | northing, easting | Lambert conic conformal (2SP) | Helmert, then a correction |
//! | 31258 | MGI / Austria GK M31 | northing, easting | transverse Mercator | Helmert, then a correction |
//! | 28992 | Amersfoort / RD New | easting, northing | oblique stereographic | Helmert |
//! | 2154 | RGF93 / Lambert-93 | easting, northing | Lambert conic conformal (2SP) | — (ETRS89) |
//! | 25832, 25833 | ETRS89 / UTM 32N, 33N | easting, northing | transverse Mercator | — (ETRS89) |
//!
//! The axis order is the one EPSG states for the system, unless the name is
//! the legacy GML form `…/epsg.xml#4326` or `CRS84`, which put longitude
//! first. ETRS89 is taken for WGS 84, as the timetables of these countries do.
//! The Helmert parameters are those of EPSG (as PROJ applies them where it has
//! no grid); MGI and CH1903 are older networks, distorted against any Helmert
//! transformation by up to a metre and a half, so a polynomial of what is left
//! of the official grid transformation follows (`tools/crs_corrections.py`).
//! Against `tests/fixtures/crs/reference.tsv` — positions the Swiss and
//! Austrian registers state, and PROJ with the national grids — every system
//! lands within a metre (0.35 m at worst, MGI); at every stop of the two
//! registers held back from fitting the corrections, within 0.61 m (MGI) and
//! 0.74 m (LV03), where the Helmert transformations alone are 1.37 m and
//! 1.79 m off.

mod corrections;

/// The order a position's two numbers are written in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Axes {
    /// Latitude, then longitude.
    LatitudeLongitude,
    /// Longitude, then latitude.
    LongitudeLatitude,
    /// Easting, then northing.
    EastingNorthing,
    /// Northing, then easting.
    NorthingEasting,
}

/// A reference system the crate turns positions from, with the order its
/// positions are written in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ReferenceSystem {
    epsg: u32,
    axes: Axes,
}

impl ReferenceSystem {
    /// WGS 84 as EPSG states it: latitude, then longitude.
    pub const WGS84: ReferenceSystem = ReferenceSystem {
        epsg: 4326,
        axes: Axes::LatitudeLongitude,
    };

    /// The system an `srsName` names — `EPSG:2056`,
    /// `urn:ogc:def:crs:EPSG::2056`, `http://www.opengis.net/def/crs/EPSG/0/2056`
    /// and the like —, if it is one the crate knows.
    pub fn from_srs_name(name: &str) -> Option<ReferenceSystem> {
        let name = name.trim();
        let lower = name.to_ascii_lowercase();
        if lower == "wgs84" || lower.ends_with("crs84") {
            return Some(ReferenceSystem {
                epsg: 4326,
                axes: Axes::LongitudeLatitude,
            });
        }
        // The legacy GML name of a system writes x before y, whatever the system.
        let legacy = lower.contains("epsg.xml#");
        let code = lower
            .rsplit([':', '/', '#'])
            .next()
            .filter(|_| lower.contains("epsg"))?
            .parse::<u32>()
            .ok()?;
        let axes = match code {
            4326 | 4258 if legacy => Axes::LongitudeLatitude,
            4326 | 4258 => Axes::LatitudeLongitude,
            31287 | 31258 if !legacy => Axes::NorthingEasting,
            2056 | 21781 | 31287 | 31258 | 28992 | 2154 | 25832 | 25833 => Axes::EastingNorthing,
            _ => return None,
        };
        Some(ReferenceSystem { epsg: code, axes })
    }

    /// The system's EPSG code.
    pub fn epsg(self) -> u32 {
        self.epsg
    }

    /// The order its positions are written in.
    pub fn axes(self) -> Axes {
        self.axes
    }

    /// Whether its positions are latitude and longitude in degrees.
    pub fn is_geographic(self) -> bool {
        matches!(self.axes, Axes::LatitudeLongitude | Axes::LongitudeLatitude)
    }

    /// A position written as `first` and `second` in this system, as latitude
    /// and longitude in WGS 84.
    pub fn to_wgs84(self, first: f64, second: f64) -> (f64, f64) {
        let (easting, northing) = match self.axes {
            Axes::LatitudeLongitude => return (first, second),
            Axes::LongitudeLatitude => return (second, first),
            Axes::EastingNorthing => (first, second),
            Axes::NorthingEasting => (second, first),
        };
        let (latitude, longitude) = match self.epsg {
            2056 => SWISS_LV95.inverse(easting, northing),
            21781 => SWISS_LV03.inverse(easting, northing),
            31287 => AUSTRIA_LAMBERT.inverse(easting, northing),
            31258 => AUSTRIA_GK_M31.inverse(easting, northing),
            28992 => RD_NEW.inverse(easting, northing),
            2154 => LAMBERT_93.inverse(easting, northing),
            25832 => utm(32).inverse(easting, northing),
            25833 => utm(33).inverse(easting, northing),
            _ => unreachable!("a reference system is one the crate knows"),
        };
        let datum = match self.epsg {
            2056 => Datum::Ch1903Plus,
            21781 => Datum::Ch1903,
            31287 | 31258 => Datum::Mgi,
            28992 => Datum::Amersfoort,
            _ => Datum::Etrs89,
        };
        datum.to_wgs84(latitude, longitude)
    }
}

/// An ellipsoid: semi-major axis in metres, and the square of the first
/// eccentricity.
#[derive(Debug, Clone, Copy)]
struct Ellipsoid {
    a: f64,
    e2: f64,
}

impl Ellipsoid {
    const fn of(a: f64, inverse_flattening: f64) -> Ellipsoid {
        let f = 1.0 / inverse_flattening;
        Ellipsoid {
            a,
            e2: f * (2.0 - f),
        }
    }

    fn e(self) -> f64 {
        self.e2.sqrt()
    }

    /// Geodetic latitude and longitude in radians, on the surface, as
    /// earth-centred coordinates in metres.
    fn geocentric(self, latitude: f64, longitude: f64) -> [f64; 3] {
        let (sin, cos) = latitude.sin_cos();
        let nu = self.a / (1.0 - self.e2 * sin * sin).sqrt();
        [
            nu * cos * longitude.cos(),
            nu * cos * longitude.sin(),
            nu * (1.0 - self.e2) * sin,
        ]
    }

    /// Earth-centred coordinates as geodetic latitude and longitude in radians.
    fn geodetic(self, [x, y, z]: [f64; 3]) -> (f64, f64) {
        let p = x.hypot(y);
        let mut latitude = z.atan2(p * (1.0 - self.e2));
        for _ in 0..6 {
            let sin = latitude.sin();
            let nu = self.a / (1.0 - self.e2 * sin * sin).sqrt();
            latitude = (z + self.e2 * nu * sin).atan2(p);
        }
        (latitude, y.atan2(x))
    }
}

const BESSEL: Ellipsoid = Ellipsoid::of(6_377_397.155, 299.152_812_8);
const GRS80: Ellipsoid = Ellipsoid::of(6_378_137.0, 298.257_222_101);
const WGS84_ELLIPSOID: Ellipsoid = Ellipsoid::of(6_378_137.0, 298.257_223_563);

/// The latitude whose isometric latitude on `ellipsoid` is `psi`.
fn latitude_of_isometric(ellipsoid: Ellipsoid, psi: f64) -> f64 {
    let e = ellipsoid.e();
    let mut latitude = 2.0 * psi.exp().atan() - std::f64::consts::FRAC_PI_2;
    for _ in 0..12 {
        let es = e * latitude.sin();
        latitude = 2.0 * (psi.exp() * ((1.0 + es) / (1.0 - es)).powf(e / 2.0)).atan()
            - std::f64::consts::FRAC_PI_2;
    }
    latitude
}

/// A map projection's inverse: easting and northing in metres to latitude and
/// longitude in degrees on the projection's ellipsoid.
trait Inverse {
    fn inverse(&self, easting: f64, northing: f64) -> (f64, f64);
}

/// The Swiss oblique Mercator (EPSG's Hotine oblique Mercator, variant B, as
/// swisstopo states it): a conformal sphere, then a cylinder through Bern.
#[derive(Debug, Clone, Copy)]
struct SwissObliqueMercator {
    false_easting: f64,
    false_northing: f64,
}

const SWISS_LV95: SwissObliqueMercator = SwissObliqueMercator {
    false_easting: 2_600_000.0,
    false_northing: 1_200_000.0,
};
const SWISS_LV03: SwissObliqueMercator = SwissObliqueMercator {
    false_easting: 600_000.0,
    false_northing: 200_000.0,
};

impl Inverse for SwissObliqueMercator {
    fn inverse(&self, easting: f64, northing: f64) -> (f64, f64) {
        use std::f64::consts::FRAC_PI_4;
        let ellipsoid = BESSEL;
        let e = ellipsoid.e();
        // The origin, the old observatory of Bern: 46°57'08.66" N, 7°26'22.50" E.
        let latitude0 = (46.0 + 57.0 / 60.0 + 8.66 / 3600.0_f64).to_radians();
        let longitude0 = (7.0 + 26.0 / 60.0 + 22.50 / 3600.0_f64).to_radians();
        let sin0 = latitude0.sin();
        let radius = ellipsoid.a * (1.0 - ellipsoid.e2).sqrt() / (1.0 - ellipsoid.e2 * sin0 * sin0);
        let alpha = (1.0 + ellipsoid.e2 / (1.0 - ellipsoid.e2) * latitude0.cos().powi(4)).sqrt();
        let b0 = (sin0 / alpha).asin();
        let k = (FRAC_PI_4 + b0 / 2.0).tan().ln()
            - alpha * (FRAC_PI_4 + latitude0 / 2.0).tan().ln()
            + alpha * e / 2.0 * ((1.0 + e * sin0) / (1.0 - e * sin0)).ln();
        let y = easting - self.false_easting;
        let x = northing - self.false_northing;
        let l_bar = y / radius;
        let b_bar = 2.0 * ((x / radius).exp().atan() - FRAC_PI_4);
        let b = (b0.cos() * b_bar.sin() + b0.sin() * b_bar.cos() * l_bar.cos()).asin();
        let l = l_bar
            .sin()
            .atan2(b0.cos() * l_bar.cos() - b0.sin() * b_bar.tan());
        let longitude = longitude0 + l / alpha;
        // The isometric latitude on the ellipsoid the sphere's latitude stands for.
        let psi = ((FRAC_PI_4 + b / 2.0).tan().ln() - k) / alpha;
        let latitude = latitude_of_isometric(ellipsoid, psi);
        (latitude.to_degrees(), longitude.to_degrees())
    }
}

/// The Lambert conic conformal projection with two standard parallels
/// (EPSG method 9802).
#[derive(Debug, Clone, Copy)]
struct LambertConic {
    ellipsoid: Ellipsoid,
    /// Latitude and longitude of the false origin, in degrees.
    origin: (f64, f64),
    /// The standard parallels, in degrees.
    parallels: (f64, f64),
    false_easting: f64,
    false_northing: f64,
}

const AUSTRIA_LAMBERT: LambertConic = LambertConic {
    ellipsoid: BESSEL,
    origin: (47.5, 13.0 + 20.0 / 60.0),
    parallels: (49.0, 46.0),
    false_easting: 400_000.0,
    false_northing: 400_000.0,
};
const LAMBERT_93: LambertConic = LambertConic {
    ellipsoid: GRS80,
    origin: (46.5, 3.0),
    parallels: (49.0, 44.0),
    false_easting: 700_000.0,
    false_northing: 6_600_000.0,
};

impl Inverse for LambertConic {
    fn inverse(&self, easting: f64, northing: f64) -> (f64, f64) {
        let ellipsoid = self.ellipsoid;
        let e = ellipsoid.e();
        let m =
            |latitude: f64| latitude.cos() / (1.0 - ellipsoid.e2 * latitude.sin().powi(2)).sqrt();
        let t = |latitude: f64| {
            let es = e * latitude.sin();
            (std::f64::consts::FRAC_PI_4 - latitude / 2.0).tan()
                / ((1.0 - es) / (1.0 + es)).powf(e / 2.0)
        };
        let (latitude1, latitude2) = (self.parallels.0.to_radians(), self.parallels.1.to_radians());
        let n = (m(latitude1).ln() - m(latitude2).ln()) / (t(latitude1).ln() - t(latitude2).ln());
        let f = m(latitude1) / (n * t(latitude1).powf(n));
        let rho_origin = ellipsoid.a * f * t(self.origin.0.to_radians()).powf(n);
        let dx = easting - self.false_easting;
        let dy = rho_origin - (northing - self.false_northing);
        let rho = n.signum() * dx.hypot(dy);
        let t_prime = (rho / (ellipsoid.a * f)).powf(1.0 / n);
        let theta = (n.signum() * dx).atan2(n.signum() * dy);
        let longitude = theta / n + self.origin.1.to_radians();
        let latitude = latitude_of_isometric(ellipsoid, -t_prime.ln());
        (latitude.to_degrees(), longitude.to_degrees())
    }
}

/// The transverse Mercator projection, by Krüger's series to the sixth
/// power of the third flattening (Karney 2011), exact to well under a
/// millimetre within the width of a UTM zone or an Austrian meridian strip.
#[derive(Debug, Clone, Copy)]
struct TransverseMercator {
    ellipsoid: Ellipsoid,
    /// The central meridian, in degrees.
    meridian: f64,
    scale: f64,
    false_easting: f64,
    false_northing: f64,
}

const AUSTRIA_GK_M31: TransverseMercator = TransverseMercator {
    ellipsoid: BESSEL,
    meridian: 13.0 + 20.0 / 60.0,
    scale: 1.0,
    false_easting: 450_000.0,
    false_northing: -5_000_000.0,
};

const fn utm(zone: u32) -> TransverseMercator {
    TransverseMercator {
        ellipsoid: GRS80,
        meridian: zone as f64 * 6.0 - 183.0,
        scale: 0.9996,
        false_easting: 500_000.0,
        false_northing: 0.0,
    }
}

impl Inverse for TransverseMercator {
    fn inverse(&self, easting: f64, northing: f64) -> (f64, f64) {
        let ellipsoid = self.ellipsoid;
        let f = 1.0 - (1.0 - ellipsoid.e2).sqrt();
        let n = f / (2.0 - f);
        let (n2, n3) = (n * n, n * n * n);
        let (n4, n5, n6) = (n3 * n, n3 * n2, n3 * n3);
        let rectifying = ellipsoid.a / (1.0 + n) * (1.0 + n2 / 4.0 + n4 / 64.0 + n6 / 256.0);
        let beta = [
            n / 2.0 - 2.0 / 3.0 * n2 + 37.0 / 96.0 * n3 - n4 / 360.0 - 81.0 / 512.0 * n5
                + 96_199.0 / 604_800.0 * n6,
            n2 / 48.0 + n3 / 15.0 - 437.0 / 1440.0 * n4 + 46.0 / 105.0 * n5
                - 1_118_711.0 / 3_870_720.0 * n6,
            17.0 / 480.0 * n3 - 37.0 / 840.0 * n4 - 209.0 / 4480.0 * n5 + 5569.0 / 90_720.0 * n6,
            4397.0 / 161_280.0 * n4 - 11.0 / 504.0 * n5 - 830_251.0 / 7_257_600.0 * n6,
            4583.0 / 161_280.0 * n5 - 108_847.0 / 3_991_680.0 * n6,
            20_648_693.0 / 638_668_800.0 * n6,
        ];
        let xi = (northing - self.false_northing) / (self.scale * rectifying);
        let eta = (easting - self.false_easting) / (self.scale * rectifying);
        let (mut xi_prime, mut eta_prime) = (xi, eta);
        for (j, b) in beta.iter().enumerate() {
            let k = 2.0 * (j + 1) as f64;
            xi_prime -= b * (k * xi).sin() * (k * eta).cosh();
            eta_prime -= b * (k * xi).cos() * (k * eta).sinh();
        }
        let chi = (xi_prime.sin() / eta_prime.cosh()).asin();
        let longitude = self.meridian.to_radians() + eta_prime.sinh().atan2(xi_prime.cos());
        let latitude = latitude_of_isometric(ellipsoid, chi.tan().asinh());
        (latitude.to_degrees(), longitude.to_degrees())
    }
}

/// The oblique stereographic projection (EPSG method 9809): a conformal
/// sphere, then a stereographic projection from it.
#[derive(Debug, Clone, Copy)]
struct ObliqueStereographic {
    ellipsoid: Ellipsoid,
    /// Latitude and longitude of the origin, in degrees.
    origin: (f64, f64),
    scale: f64,
    false_easting: f64,
    false_northing: f64,
}

const RD_NEW: ObliqueStereographic = ObliqueStereographic {
    ellipsoid: BESSEL,
    // Amersfoort: 52°09'22.178" N, 5°23'15.500" E.
    origin: (
        52.0 + 9.0 / 60.0 + 22.178 / 3600.0,
        5.0 + 23.0 / 60.0 + 15.5 / 3600.0,
    ),
    scale: 0.999_907_9,
    false_easting: 155_000.0,
    false_northing: 463_000.0,
};

impl Inverse for ObliqueStereographic {
    fn inverse(&self, easting: f64, northing: f64) -> (f64, f64) {
        use std::f64::consts::FRAC_PI_4;
        let ellipsoid = self.ellipsoid;
        let (e2, e) = (ellipsoid.e2, ellipsoid.e());
        let latitude0 = self.origin.0.to_radians();
        let longitude0 = self.origin.1.to_radians();
        let sin0 = latitude0.sin();
        let rho0 = ellipsoid.a * (1.0 - e2) / (1.0 - e2 * sin0 * sin0).powf(1.5);
        let nu0 = ellipsoid.a / (1.0 - e2 * sin0 * sin0).sqrt();
        let radius = (rho0 * nu0).sqrt();
        let n = (1.0 + e2 * latitude0.cos().powi(4) / (1.0 - e2)).sqrt();
        let s1 = (1.0 + sin0) / (1.0 - sin0);
        let s2 = (1.0 - e * sin0) / (1.0 + e * sin0);
        let w1 = (s1 * s2.powf(e)).powf(n);
        let sin_chi0 = (w1 - 1.0) / (w1 + 1.0);
        let c = (n + sin0) * (1.0 - sin_chi0) / ((n - sin0) * (1.0 + sin_chi0));
        let w2 = c * w1;
        let chi0 = ((w2 - 1.0) / (w2 + 1.0)).asin();
        let k = radius * self.scale;
        let (de, dn) = (easting - self.false_easting, northing - self.false_northing);
        let g = 2.0 * k * (FRAC_PI_4 - chi0 / 2.0).tan();
        let h = 4.0 * k * chi0.tan() + g;
        let i = (de / (h + dn)).atan();
        let j = (de / (g - dn)).atan() - i;
        let chi = chi0 + 2.0 * ((dn - de * (j / 2.0).tan()) / (2.0 * k)).atan();
        let big_lambda = j + 2.0 * i + longitude0;
        let longitude = (big_lambda - longitude0) / n + longitude0;
        let psi = 0.5 * ((1.0 + chi.sin()) / (c * (1.0 - chi.sin()))).ln() / n;
        let latitude = latitude_of_isometric(ellipsoid, psi);
        (latitude.to_degrees(), longitude.to_degrees())
    }
}

/// A geodetic datum, and how it turns into WGS 84.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Datum {
    /// ETRS89, taken for WGS 84.
    Etrs89,
    /// CH1903+ (LV95): the translation `CH1903+ to WGS 84 (1)`.
    Ch1903Plus,
    /// CH1903 (LV03): the translation `CH1903 to WGS 84 (2)`, then the correction.
    Ch1903,
    /// MGI: the Helmert transformation `MGI to WGS 84 (3)`, then the correction.
    Mgi,
    /// Amersfoort: the Helmert transformation `Amersfoort to WGS 84 (4)`.
    Amersfoort,
}

/// A Helmert transformation between earth-centred coordinates: translations
/// in metres, rotations in arc seconds and a scale in parts per million, in
/// the position vector convention (the coordinate frame convention turns the
/// rotations the other way).
#[derive(Debug, Clone, Copy)]
struct Helmert {
    translation: [f64; 3],
    rotation: [f64; 3],
    scale_ppm: f64,
}

impl Helmert {
    const fn translation(x: f64, y: f64, z: f64) -> Helmert {
        Helmert {
            translation: [x, y, z],
            rotation: [0.0; 3],
            scale_ppm: 0.0,
        }
    }

    fn apply(self, [x, y, z]: [f64; 3]) -> [f64; 3] {
        let [rx, ry, rz] = self.rotation.map(|seconds| (seconds / 3600.0).to_radians());
        let s = 1.0 + self.scale_ppm * 1e-6;
        let [tx, ty, tz] = self.translation;
        [
            tx + s * (x - rz * y + ry * z),
            ty + s * (rz * x + y - rx * z),
            tz + s * (-ry * x + rx * y + z),
        ]
    }
}

/// The correction after a Helmert transformation: a polynomial in longitude
/// and latitude, in degrees from `centre`, of the longitude and latitude to
/// add, in degrees; its terms `u^i v^j` for `i` from 0 to the degree and `j`
/// from 0 to the degree less `i`.
#[derive(Debug)]
struct Correction {
    centre: (f64, f64),
    degree: usize,
    longitude: &'static [f64],
    latitude: &'static [f64],
}

impl Correction {
    /// Latitude and longitude in degrees, corrected.
    fn apply(&self, latitude: f64, longitude: f64) -> (f64, f64) {
        let (u, v) = (longitude - self.centre.0, latitude - self.centre.1);
        let (mut dlon, mut dlat) = (0.0, 0.0);
        let mut term = 0;
        let mut ui = 1.0;
        for i in 0..=self.degree {
            let mut vj = 1.0;
            for _ in 0..=self.degree - i {
                dlon += self.longitude[term] * ui * vj;
                dlat += self.latitude[term] * ui * vj;
                term += 1;
                vj *= v;
            }
            ui *= u;
        }
        (latitude + dlat, longitude + dlon)
    }
}

impl Datum {
    /// Latitude and longitude in degrees on this datum, in WGS 84.
    fn to_wgs84(self, latitude: f64, longitude: f64) -> (f64, f64) {
        let (helmert, correction) = match self {
            Datum::Etrs89 => return (latitude, longitude),
            Datum::Ch1903Plus => (Helmert::translation(674.374, 15.056, 405.346), None),
            Datum::Ch1903 => (
                Helmert::translation(674.374, 15.056, 405.346),
                Some(&corrections::CH1903),
            ),
            Datum::Mgi => (
                Helmert {
                    translation: [577.326, 90.129, 463.919],
                    rotation: [5.137, 1.474, 5.297],
                    scale_ppm: 2.4232,
                },
                Some(&corrections::MGI),
            ),
            // Coordinate frame convention: the rotations turned to position vector.
            Datum::Amersfoort => (
                Helmert {
                    translation: [565.4171, 50.3319, 465.5524],
                    rotation: [
                        -0.398_957_388_243_134,
                        0.343_987_817_378_283,
                        -1.877_401_639_980_45,
                    ],
                    scale_ppm: 4.0725,
                },
                None,
            ),
        };
        let geocentric = BESSEL.geocentric(latitude.to_radians(), longitude.to_radians());
        let (latitude, longitude) = WGS84_ELLIPSOID.geodetic(helmert.apply(geocentric));
        let (latitude, longitude) = (latitude.to_degrees(), longitude.to_degrees());
        match correction {
            Some(correction) => correction.apply(latitude, longitude),
            None => (latitude, longitude),
        }
    }
}

/// A reference system the crate does not know, as its `srsName` names it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownSystem(pub String);

/// The system `srs_name` names, else `default_system` — a frame's
/// `DefaultLocationSystem` —, else WGS 84; an error where the one named is
/// not one the crate knows.
pub fn system_of(
    srs_name: Option<&str>,
    default_system: Option<&str>,
) -> Result<ReferenceSystem, UnknownSystem> {
    match srs_name.or(default_system) {
        Some(name) => {
            ReferenceSystem::from_srs_name(name).ok_or_else(|| UnknownSystem(name.to_owned()))
        }
        None => Ok(ReferenceSystem::WGS84),
    }
}

/// Where a location is, as latitude and longitude in WGS 84: its `Latitude`
/// and `Longitude`, which are degrees; else its GML position or its
/// `Coordinates`, in the system named by the position's `srsName`, else by
/// the location's, else by `default_system` — the frame's
/// `DefaultLocationSystem` —, else in WGS 84. None where it states no
/// position; an error naming the system where the crate does not know it.
pub fn location_in_wgs84(
    location: &crate::model::Location,
    default_system: Option<&str>,
) -> Result<Option<(f64, f64)>, UnknownSystem> {
    if let (Some(latitude), Some(longitude)) = (location.latitude, location.longitude) {
        return Ok(Some((latitude, longitude)));
    }
    let location_system = location.srs_name.as_deref();
    let written = location
        .unprocessed
        .iter()
        .filter(|fragment| &*fragment.name == "pos")
        .find_map(|fragment| crate::geometry::geometry(&fragment.xml))
        .and_then(|geometry| {
            let first = geometry.positions().next()?;
            Some((first, geometry.srs_name))
        })
        .or_else(|| {
            let text = location.coordinates.as_deref()?;
            let mut numbers = text
                .split(|c: char| c.is_ascii_whitespace() || c == ',')
                .filter(|part| !part.is_empty())
                .map(str::parse::<f64>);
            match (numbers.next(), numbers.next()) {
                (Some(Ok(a)), Some(Ok(b))) => Some(((a, b), None)),
                _ => None,
            }
        });
    let Some(((first, second), position_system)) = written else {
        return Ok(None);
    };
    let system = system_of(
        position_system.as_deref().or(location_system),
        default_system,
    )?;
    Ok(Some(system.to_wgs84(first, second)))
}
