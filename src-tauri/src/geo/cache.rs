//! In-process short-TTL cache for geo results. No IP addresses or network
//! details are stored — only the resolved region/country, which is not
//! personally identifying.

use super::model::GeoResult;
use std::time::{Duration, Instant};

const TTL: Duration = Duration::from_secs(10 * 60);

#[derive(Default)]
pub struct GeoCacheInner {
    result: Option<GeoResult>,
    stored_at: Option<Instant>,
}

impl GeoCacheInner {
    pub fn get(&self, now: Instant) -> Option<GeoResult> {
        let stored_at = self.stored_at?;
        if now.duration_since(stored_at) < TTL {
            self.result.clone()
        } else {
            None
        }
    }

    pub fn store(&mut self, result: GeoResult, now: Instant) {
        self.result = Some(result);
        self.stored_at = Some(now);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cached_result_is_returned_within_ttl() {
        let mut inner = GeoCacheInner::default();
        let now = Instant::now();
        inner.store(GeoResult::unknown(), now);
        assert!(inner.get(now).is_some());
        assert!(inner.get(now + TTL - Duration::from_secs(1)).is_some());
        assert!(inner.get(now + TTL + Duration::from_secs(1)).is_none());
    }

    #[test]
    fn empty_cache_returns_none() {
        let inner = GeoCacheInner::default();
        assert!(inner.get(Instant::now()).is_none());
    }
}
