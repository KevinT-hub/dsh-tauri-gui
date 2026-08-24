//! Geo domain types: normalized region code, per-endpoint samples and the
//! final result. Pure data — no network, no disk.

use serde::{Deserialize, Serialize};

/// Normalized region derived from the majority country code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RegionCode {
    /// Mainland China.
    Cn,
    /// Any other region.
    World,
    /// Geo lookup failed or results conflicted; safe default.
    Unknown,
}

impl RegionCode {
    /// Map an ISO 3166-1 alpha-2 country code to a region. Anything that is
    /// not a valid two-letter code, or `CN`, maps to `World`; empty input
    /// maps to `Unknown`.
    pub fn from_iso(code: &str) -> Self {
        let code = code.trim().to_ascii_lowercase();
        if code.is_empty() {
            return RegionCode::Unknown;
        }
        let valid = code.len() == 2 && code.chars().all(|c| c.is_ascii_alphabetic());
        if !valid {
            return RegionCode::Unknown;
        }
        if code == "cn" {
            RegionCode::Cn
        } else {
            RegionCode::World
        }
    }

    #[allow(dead_code)]
    pub fn is_cn(self) -> bool {
        matches!(self, RegionCode::Cn)
    }
}

/// One endpoint's raw answer before consensus.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GeoSample {
    /// Endpoint name, for diagnostics and the result summary.
    pub source: &'static str,
    /// Normalized country code (`cn`, `us`, ...) or `None` on failure.
    pub country: Option<String>,
}

/// Final geo result handed to the UI and to `detection::sources`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GeoResult {
    pub region: RegionCode,
    /// The consensus country code, when available.
    pub country: Option<String>,
    /// Number of endpoints that returned a valid code.
    pub matched: usize,
    /// Total number of endpoints probed.
    pub total: usize,
    /// Endpoint names that answered (diagnostics only).
    pub sources: Vec<&'static str>,
}

impl GeoResult {
    pub fn unknown() -> Self {
        Self {
            region: RegionCode::Unknown,
            country: None,
            matched: 0,
            total: 0,
            sources: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iso_normalization() {
        assert_eq!(RegionCode::from_iso("cn"), RegionCode::Cn);
        assert_eq!(RegionCode::from_iso("CN"), RegionCode::Cn);
        assert_eq!(RegionCode::from_iso(" cN "), RegionCode::Cn);
        assert_eq!(RegionCode::from_iso("us"), RegionCode::World);
        assert_eq!(RegionCode::from_iso("hk"), RegionCode::World);
        assert_eq!(RegionCode::from_iso(""), RegionCode::Unknown);
        assert_eq!(RegionCode::from_iso("chi"), RegionCode::Unknown);
        assert_eq!(RegionCode::from_iso("12"), RegionCode::Unknown);
    }
}
