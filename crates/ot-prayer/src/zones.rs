//! The JAKIM prayer-time zones, and which one a location is in
//! (`docs/11-prayer-and-azan.md` §3).
//!
//! Both tables are embedded from `data/`: the zones as JAKIM publishes them,
//! with English forms, and a point for each of their areas from GeoNames
//! (`data/README.md` has the sources). The nearest point gives the zone. That
//! can be wrong near a boundary, so the app always shows the detected zone for
//! confirmation and lets the user pick another.

use std::sync::LazyLock;

/// One JAKIM zone.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Zone {
    /// `SGR01`, `WLY01` …: the key JAKIM's times are fetched and stored by.
    pub code: String,
    /// The state, as JAKIM writes it and in English.
    pub state_ms: String,
    pub state_en: String,
    /// The areas, as JAKIM writes them and in English.
    pub areas_ms: String,
    pub areas_en: String,
}

impl Zone {
    /// The state in the given language.
    pub fn state(&self, english: bool) -> &str {
        if english {
            &self.state_en
        } else {
            &self.state_ms
        }
    }

    /// The areas in the given language.
    pub fn areas(&self, english: bool) -> &str {
        if english {
            &self.areas_en
        } else {
            &self.areas_ms
        }
    }
}

/// A place in a zone, for finding the nearest.
#[derive(Debug, Clone, PartialEq)]
pub struct Point {
    pub code: String,
    pub place: String,
    pub latitude: f64,
    pub longitude: f64,
}

/// How far the nearest point may be before a location in Malaysia is said to
/// have no zone — a ship at sea, a typo in the coordinates.
pub const MAX_DISTANCE_KM: f64 = 120.0;

/// Malaysia, Peninsular and East, as a box: a cheap first refusal before the
/// country code (`docs/11-prayer-and-azan.md` §3).
const LATITUDE: (f64, f64) = (0.8, 7.5);
const LONGITUDE: (f64, f64) = (99.5, 119.5);

static ZONES: LazyLock<Vec<Zone>> = LazyLock::new(|| {
    parse_zones(include_str!("../../../data/jakim_zones.csv")).unwrap_or_default()
});

static POINTS: LazyLock<Vec<Point>> = LazyLock::new(|| {
    parse_points(include_str!("../../../data/jakim_zone_points.csv")).unwrap_or_default()
});

/// Every zone, in JAKIM's order.
pub fn zones() -> &'static [Zone] {
    &ZONES
}

/// Every point.
pub fn points() -> &'static [Point] {
    &POINTS
}

/// A zone by its code, in any case — e-Solat accepts `sgr01` too.
pub fn zone(code: &str) -> Option<&'static Zone> {
    ZONES
        .iter()
        .find(|zone| zone.code.eq_ignore_ascii_case(code))
}

/// The zones matching what the user typed, in JAKIM's order: a case-insensitive
/// match on the code, the state or the areas, in either language, so "selangor",
/// "SGR", "ulu langat" and "hulu langat" all find their zone. An empty query
/// matches every zone.
pub fn search(query: &str) -> Vec<&'static Zone> {
    let needle = query.trim().to_lowercase();
    ZONES
        .iter()
        .filter(|zone| {
            needle.is_empty()
                || [
                    &zone.code,
                    &zone.state_ms,
                    &zone.state_en,
                    &zone.areas_ms,
                    &zone.areas_en,
                ]
                .iter()
                .any(|field| field.to_lowercase().contains(&needle))
        })
        .collect()
}

/// The zone of the point nearest a location, and how far that point is.
pub fn nearest(latitude: f64, longitude: f64) -> Option<(&'static Zone, f64)> {
    let (point, km) = POINTS
        .iter()
        .map(|point| {
            (
                point,
                distance_km(latitude, longitude, point.latitude, point.longitude),
            )
        })
        .min_by(|a, b| a.1.total_cmp(&b.1))?;
    Some((zone(&point.code)?, km))
}

/// Whether a location is in Malaysia: inside the box and, when the
/// geolocation lookup gave one, with the country code `MY`. Coordinates typed
/// by hand have no code, so the box and the distance to the nearest zone are
/// all there is; a point in Singapore or Brunei can pass that, and the
/// confirmation step is what catches it.
pub fn in_malaysia(latitude: f64, longitude: f64, country: Option<&str>) -> bool {
    let in_box = (LATITUDE.0..=LATITUDE.1).contains(&latitude)
        && (LONGITUDE.0..=LONGITUDE.1).contains(&longitude);
    in_box && country.is_none_or(|code| code.eq_ignore_ascii_case("MY"))
}

/// The zone to offer for a location, or `None` outside Malaysia or far from
/// every zone — JAKIM is not offered there.
pub fn detect(latitude: f64, longitude: f64, country: Option<&str>) -> Option<&'static Zone> {
    if !in_malaysia(latitude, longitude, country) {
        return None;
    }
    let (zone, km) = nearest(latitude, longitude)?;
    (km <= MAX_DISTANCE_KM).then_some(zone)
}

/// Great-circle distance.
fn distance_km(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    let (p1, p2) = (lat1.to_radians(), lat2.to_radians());
    let dp = p2 - p1;
    let dl = (lon2 - lon1).to_radians();
    let h = (dp / 2.0).sin().powi(2) + p1.cos() * p2.cos() * (dl / 2.0).sin().powi(2);
    6371.0 * 2.0 * h.sqrt().asin()
}

fn parse_zones(text: &str) -> Result<Vec<Zone>, String> {
    rows(
        text,
        &["code", "state_ms", "state_en", "areas_ms", "areas_en"],
    )?
    .into_iter()
    .map(|mut f| {
        Ok(Zone {
            areas_en: f.pop().unwrap_or_default(),
            areas_ms: f.pop().unwrap_or_default(),
            state_en: f.pop().unwrap_or_default(),
            state_ms: f.pop().unwrap_or_default(),
            code: f.pop().unwrap_or_default(),
        })
    })
    .collect()
}

fn parse_points(text: &str) -> Result<Vec<Point>, String> {
    rows(
        text,
        &["code", "place", "latitude", "longitude", "geonameid"],
    )?
    .into_iter()
    .map(|f| {
        let number = |i: usize| {
            f[i].parse::<f64>()
                .map_err(|e| format!("{} {}: {e}", f[0], f[i]))
        };
        Ok(Point {
            code: f[0].clone(),
            place: f[1].clone(),
            latitude: number(2)?,
            longitude: number(3)?,
        })
    })
    .collect()
}

/// The rows of a CSV with the given header, each with that many fields.
fn rows(text: &str, header: &[&str]) -> Result<Vec<Vec<String>>, String> {
    let mut lines = text.lines().filter(|line| !line.trim().is_empty());
    let first = lines.next().ok_or("empty")?;
    if split(first)? != header {
        return Err(format!("unexpected header {first:?}"));
    }
    lines
        .map(|line| {
            let fields = split(line)?;
            if fields.len() == header.len() {
                Ok(fields)
            } else {
                Err(format!("{} fields in {line:?}", fields.len()))
            }
        })
        .collect()
}

/// One CSV line: commas separate fields, and a field in double quotes may hold
/// commas and `""` for a quote.
fn split(line: &str) -> Result<Vec<String>, String> {
    let mut fields = Vec::new();
    let mut field = String::new();
    let mut chars = line.chars().peekable();
    let mut quoted = false;
    while let Some(c) = chars.next() {
        match (c, quoted) {
            ('"', false) if field.is_empty() => quoted = true,
            ('"', true) if chars.peek() == Some(&'"') => {
                field.push('"');
                chars.next();
            }
            ('"', true) => quoted = false,
            (',', false) => fields.push(std::mem::take(&mut field)),
            (c, _) => field.push(c),
        }
    }
    if quoted {
        return Err(format!("unclosed quote in {line:?}"));
    }
    fields.push(field);
    Ok(fields)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_tables_parse_whole() {
        // Parsing directly, so a broken file fails here rather than turning
        // into an empty table at run time.
        let zones = parse_zones(include_str!("../../../data/jakim_zones.csv")).expect("zones");
        let points =
            parse_points(include_str!("../../../data/jakim_zone_points.csv")).expect("points");
        assert_eq!(zones.len(), 60, "JAKIM's 60 zones");
        assert_eq!(zones, super::zones());
        assert_eq!(points.len(), super::points().len());
        assert!(points.len() > 200);
    }

    #[test]
    fn every_code_is_unique_well_formed_and_named_in_both_languages() {
        let mut codes: Vec<&str> = zones().iter().map(|z| z.code.as_str()).collect();
        for zone in zones() {
            let (letters, digits) = zone.code.split_at(3);
            assert!(
                letters.chars().all(|c| c.is_ascii_uppercase()),
                "{}",
                zone.code
            );
            assert!(
                digits.len() == 2 && digits.chars().all(|c| c.is_ascii_digit()),
                "{}",
                zone.code
            );
            for text in [
                &zone.state_ms,
                &zone.state_en,
                &zone.areas_ms,
                &zone.areas_en,
            ] {
                assert!(!text.trim().is_empty(), "{} has an empty name", zone.code);
            }
        }
        codes.sort_unstable();
        codes.dedup();
        assert_eq!(codes.len(), 60);
    }

    #[test]
    fn every_point_is_in_a_known_zone_inside_malaysia_and_only_swk09_has_none() {
        for point in points() {
            assert!(zone(&point.code).is_some(), "{} is not a zone", point.code);
            assert!(
                in_malaysia(point.latitude, point.longitude, None),
                "{} {} is outside the box",
                point.code,
                point.place
            );
        }
        let without: Vec<&str> = zones()
            .iter()
            .filter(|z| !points().iter().any(|p| p.code == z.code))
            .map(|z| z.code.as_str())
            .collect();
        assert_eq!(
            without,
            vec!["SWK09"],
            "the special zone is chosen, not detected"
        );
    }

    #[test]
    fn the_spikes_zones_are_present_and_found_in_any_case() {
        for code in ["SGR01", "WLY01", "JHR01", "sgr01"] {
            assert!(zone(code).is_some(), "{code}");
        }
        assert_eq!(zone("XXX99"), None);
    }

    #[test]
    fn each_state_and_territory_capital_lands_in_its_zone() {
        let capitals = [
            ("Kuala Lumpur", 3.1390, 101.6869, "WLY01"),
            ("Putrajaya", 2.9264, 101.6964, "WLY01"),
            ("Labuan", 5.2831, 115.2308, "WLY02"),
            ("Shah Alam", 3.0733, 101.5185, "SGR01"),
            ("Johor Bahru", 1.4927, 103.7414, "JHR02"),
            ("Alor Setar", 6.1248, 100.3678, "KDH01"),
            ("Kota Bharu", 6.1254, 102.2381, "KTN01"),
            ("Melaka", 2.1896, 102.2501, "MLK01"),
            ("Seremban", 2.7297, 101.9381, "NGS03"),
            ("Kuantan", 3.8077, 103.3260, "PHG02"),
            ("Kangar", 6.4414, 100.1986, "PLS01"),
            ("George Town", 5.4141, 100.3288, "PNG01"),
            ("Ipoh", 4.5975, 101.0901, "PRK02"),
            ("Kota Kinabalu", 5.9804, 116.0735, "SBH07"),
            ("Kuching", 1.5535, 110.3593, "SWK08"),
            ("Kuala Terengganu", 5.3302, 103.1408, "TRG01"),
        ];
        for (city, lat, lon, expected) in capitals {
            let found = detect(lat, lon, Some("MY")).map(|z| z.code.as_str());
            assert_eq!(found, Some(expected), "{city}");
        }
    }

    #[test]
    fn neighbours_in_the_klang_valley_are_told_apart_by_their_towns() {
        let at = |lat, lon| detect(lat, lon, Some("MY")).map(|z| z.code.as_str());
        assert_eq!(at(3.1073, 101.6067), Some("SGR01"), "Petaling Jaya");
        assert_eq!(at(3.0449, 101.4456), Some("SGR03"), "Klang");
        assert_eq!(at(3.3375, 101.2504), Some("SGR02"), "Kuala Selangor");
    }

    #[test]
    fn outside_malaysia_jakim_is_not_offered() {
        // Bangkok and Jakarta are outside the box; Singapore is inside it, so
        // the lookup's country code is what refuses it.
        assert_eq!(detect(13.7563, 100.5018, None), None, "Bangkok");
        assert_eq!(detect(-6.2088, 106.8456, None), None, "Jakarta");
        assert_eq!(detect(1.3521, 103.8198, Some("SG")), None, "Singapore");
        assert!(!in_malaysia(1.3521, 103.8198, Some("SG")));
        assert!(in_malaysia(3.1390, 101.6869, Some("my")), "any case");
        // In the box but far from every zone: the South China Sea.
        assert_eq!(detect(4.5, 109.0, None), None);
    }

    #[test]
    fn a_quoted_field_keeps_its_commas_and_quotes() {
        assert_eq!(
            split(r#"A,"b, c","say ""hi""",d"#).expect("splits"),
            vec!["A", "b, c", r#"say "hi""#, "d"]
        );
        assert!(split(r#"A,"open"#).is_err());
        assert!(rows("x,y\n1\n", &["x", "y"]).is_err(), "a short row");
        assert!(rows("a,b\n", &["x", "y"]).is_err(), "a wrong header");
    }

    #[test]
    fn zone_search_finds_by_code_state_and_area_in_either_language() {
        let codes = |query: &str| -> Vec<String> {
            search(query).iter().map(|zone| zone.code.clone()).collect()
        };
        assert_eq!(search("").len(), zones().len(), "empty matches every zone");
        assert_eq!(search("   ").len(), zones().len());
        assert_eq!(codes("sgr01"), ["SGR01"], "a code, in any case");
        assert!(codes("SGR").starts_with(&["SGR01".to_string()]));
        // A state, by JAKIM's name and by the English one.
        assert!(codes("selangor").contains(&"SGR01".to_string()));
        assert!(codes("Selangor").iter().all(|code| code.starts_with("SGR")));
        // An area, which is how most people look for a zone.
        assert!(codes("hulu langat").contains(&"SGR01".to_string()));
        assert!(codes("petaling").contains(&"SGR01".to_string()));
        assert_eq!(codes("no such place anywhere"), Vec::<String>::new());
        assert!(
            codes("  kuala lumpur ").contains(&"WLY01".to_string()),
            "trimmed"
        );
    }

    #[test]
    fn the_longest_zone_text_is_known_so_a_layout_can_be_tested_with_it() {
        let longest = zones()
            .iter()
            .flat_map(|zone| [&zone.areas_ms, &zone.areas_en])
            .map(|text| text.chars().count())
            .max()
            .expect("zones");
        // Recorded so a layout test uses the real worst case, and a data change
        // that makes it longer is noticed here and not in a clipped label.
        assert!(
            (100..=400).contains(&longest),
            "the longest areas text is {longest} characters"
        );
    }
}
