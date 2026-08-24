//! Geo region service: parallel country-code lookup over fixed HTTPS
//! endpoints, majority consensus, in-process short-TTL cache.
//!
//! Guarantees:
//! - No tokens, cookies or user-identifiable payloads are ever sent.
//! - A failed endpoint only affects source selection — never engine startup.
//! - The result is cached in-process (short TTL); no IP or network detail is
//!   persisted to disk.

pub mod cache;
pub mod client;
pub mod consensus;
pub mod endpoints;
pub mod model;

use model::GeoResult;
use std::sync::Mutex;
use std::time::Duration;

/// In-process cache shared across all lookups in this process.
pub struct GeoCache {
    inner: Mutex<cache::GeoCacheInner>,
}

impl Default for GeoCache {
    fn default() -> Self {
        Self {
            inner: Mutex::new(cache::GeoCacheInner::default()),
        }
    }
}

/// Resolve the region for this machine. Uses the short-TTL cache first, then
/// runs the parallel endpoint consensus. Never fails: an unreachable geo
/// stack yields `RegionCode::Unknown`.
pub fn resolve(cache: &GeoCache) -> GeoResult {
    let now = std::time::Instant::now();
    if let Some(cached) = cache.inner.lock().unwrap().get(now) {
        return cached;
    }

    let samples = client::probe_endpoints(Duration::from_secs(5));
    let result = consensus::decide(&samples);
    cache.inner.lock().unwrap().store(result.clone(), now);
    result
}
