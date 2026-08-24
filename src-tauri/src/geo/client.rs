//! Parallel geo endpoint probing with bounded timeouts.
//!
//! Each endpoint runs on its own thread with a short connect/global timeout;
//! the whole probe set never blocks the caller longer than `per_endpoint` +
//! a small scheduling margin. Responses are parsed leniently — an endpoint
//! that returns garbage simply contributes no sample.

use super::endpoints;
use super::model::GeoSample;
use crate::core::http;
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Probe every endpoint concurrently and collect one sample per endpoint.
pub fn probe_endpoints(per_endpoint: Duration) -> Vec<GeoSample> {
    let samples = Arc::new(Mutex::new(Vec::with_capacity(endpoints::ENDPOINTS.len())));

    std::thread::scope(|scope| {
        for (name, url) in endpoints::ENDPOINTS {
            let samples = Arc::clone(&samples);
            scope.spawn(move || {
                let sample = probe_one(name, url, per_endpoint);
                samples.lock().unwrap().push(sample);
            });
        }
    });

    let mut all = Arc::try_unwrap(samples).unwrap().into_inner().unwrap();
    all.sort_by_key(|sample| sample.source);
    all
}

fn probe_one(name: &'static str, url: &str, timeout: Duration) -> GeoSample {
    let agent = http::http_agent(Some(timeout));
    let response = match agent
        .get(url)
        .header("User-Agent", http::USER_AGENT)
        .header("Accept", "application/json")
        .call()
    {
        Ok(response) if response.status().is_success() => response,
        Ok(_) | Err(_) => {
            return GeoSample {
                source: name,
                country: None,
            }
        }
    };

    let body: serde_json::Value = match response.into_body().read_json() {
        Ok(value) => value,
        Err(_) => {
            return GeoSample {
                source: name,
                country: None,
            }
        }
    };

    let country = extract_country(&body);
    GeoSample {
        source: name,
        country,
    }
}

/// Lenient extraction of an ISO country code from the common response
/// shapes: `{"country":"CN"}`, `{"country_code":"CN"}`,
/// `{"countryCode":"CN"}`, or a plain `"CN"` string.
fn extract_country(value: &serde_json::Value) -> Option<String> {
    let candidate = match value {
        serde_json::Value::String(text) => Some(text.clone()),
        serde_json::Value::Object(map) => {
            ["country", "country_code", "countryCode", "country_name"]
                .iter()
                .find_map(|key| map.get(*key).and_then(|v| v.as_str()))
                .map(str::to_string)
        }
        _ => None,
    }?;
    let normalized = candidate.trim().to_ascii_lowercase();
    let valid = normalized.len() == 2 && normalized.chars().all(|c| c.is_ascii_alphabetic());
    if valid {
        Some(normalized)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::extract_country;
    use serde_json::json;

    #[test]
    fn parses_common_response_shapes() {
        assert_eq!(
            extract_country(&json!({"country": "CN"})).as_deref(),
            Some("cn")
        );
        assert_eq!(
            extract_country(&json!({"country_code": "US"})).as_deref(),
            Some("us")
        );
        assert_eq!(
            extract_country(&json!({"countryCode": "jp"})).as_deref(),
            Some("jp")
        );
    }

    #[test]
    fn ignores_invalid_or_absent_codes() {
        assert_eq!(extract_country(&json!({"country": "CHI"})), None);
        assert_eq!(extract_country(&json!({"foo": "CN"})), None);
        assert_eq!(extract_country(&json!({"country": 42})), None);
        assert_eq!(extract_country(&json!(null)), None);
    }
}
