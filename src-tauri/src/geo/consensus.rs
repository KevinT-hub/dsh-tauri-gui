//! Majority-consensus decision over per-endpoint samples.
//!
//! - A clear majority (strictly more than half of the *valid* answers, and
//!   at least one) wins.
//! - A tie between different countries, or zero valid answers, yields
//!   `RegionCode::Unknown` — geo must never be a startup blocker.

use super::model::{GeoResult, GeoSample, RegionCode};
use std::collections::HashMap;

pub fn decide(samples: &[GeoSample]) -> GeoResult {
    let mut counts: HashMap<&str, usize> = HashMap::new();
    let mut matched = 0usize;
    let mut sources = Vec::new();

    for sample in samples {
        if let Some(country) = sample.country.as_deref() {
            *counts.entry(country).or_insert(0) += 1;
            matched += 1;
            sources.push(sample.source);
        }
    }

    if matched == 0 {
        return GeoResult::unknown();
    }

    let mut ranked: Vec<(&str, usize)> = counts.into_iter().collect();
    ranked.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(b.0)));
    let (country, top) = ranked[0];
    let second = ranked.get(1).map(|(_, count)| *count).unwrap_or(0);

    // Strict majority over valid answers: `top > matched - top`.
    let region = if top > second && top * 2 > matched {
        RegionCode::from_iso(country)
    } else {
        RegionCode::Unknown
    };

    GeoResult {
        region,
        country: if region == RegionCode::Unknown {
            None
        } else {
            Some(country.to_string())
        },
        matched,
        total: samples.len(),
        sources,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(source: &'static str, country: Option<&str>) -> GeoSample {
        GeoSample {
            source,
            country: country.map(str::to_string),
        }
    }

    #[test]
    fn unanimous_result_wins() {
        let samples = vec![
            sample("a", Some("cn")),
            sample("b", Some("cn")),
            sample("c", Some("cn")),
        ];
        let result = decide(&samples);
        assert_eq!(result.region, RegionCode::Cn);
        assert_eq!(result.country.as_deref(), Some("cn"));
        assert_eq!(result.matched, 3);
    }

    #[test]
    fn clear_majority_wins() {
        let samples = vec![
            sample("a", Some("cn")),
            sample("b", Some("cn")),
            sample("c", Some("us")),
        ];
        let result = decide(&samples);
        assert_eq!(result.region, RegionCode::Cn);
    }

    #[test]
    fn tie_yields_unknown() {
        let samples = vec![sample("a", Some("cn")), sample("b", Some("us"))];
        let result = decide(&samples);
        assert_eq!(result.region, RegionCode::Unknown);
        assert_eq!(result.country, None);
    }

    #[test]
    fn all_failures_yield_unknown() {
        let samples = vec![sample("a", None), sample("b", None)];
        let result = decide(&samples);
        assert_eq!(result.region, RegionCode::Unknown);
        assert_eq!(result.matched, 0);
    }

    #[test]
    fn empty_input_yields_unknown() {
        let result = decide(&[]);
        assert_eq!(result.region, RegionCode::Unknown);
    }
}
