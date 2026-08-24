//! Registry / mirror source policy, driven by the geo result.
//!
//! - Mainland China (`cn`): npm mirror + Node mirror (npmmirror).
//! - Everywhere else and geo-unknown: official sources.
//!
//! `geo.rs` never decides sources; it only returns a normalized `RegionCode`.
//! Source selection lives here so installation logic never touches geo logic.

use crate::geo::model::RegionCode;
use serde::Serialize;

/// npm registry for the current region.
pub const REGISTRY_CN: &str = "https://registry.npmmirror.com";
pub const REGISTRY_OFFICIAL: &str = "https://registry.npmjs.org";

/// Node.js dist mirror for the current region (install help / downloads).
pub const NODE_MIRROR_CN: &str = "https://npmmirror.com/mirrors/node";
pub const NODE_MIRROR_OFFICIAL: &str = "https://nodejs.org/dist";

/// Resolved source policy for the setup/install flows.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourcePolicy {
    pub region: RegionCode,
    pub npm_registry: &'static str,
    pub node_mirror: &'static str,
}

pub fn resolve_sources(region: RegionCode) -> SourcePolicy {
    match region {
        RegionCode::Cn => SourcePolicy {
            region,
            npm_registry: REGISTRY_CN,
            node_mirror: NODE_MIRROR_CN,
        },
        RegionCode::World | RegionCode::Unknown => SourcePolicy {
            region,
            npm_registry: REGISTRY_OFFICIAL,
            node_mirror: NODE_MIRROR_OFFICIAL,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cn_uses_mirror_registry() {
        let policy = resolve_sources(RegionCode::Cn);
        assert_eq!(policy.npm_registry, REGISTRY_CN);
        assert!(policy.region.is_cn());
    }

    #[test]
    fn world_uses_official_registry() {
        let policy = resolve_sources(RegionCode::World);
        assert_eq!(policy.npm_registry, REGISTRY_OFFICIAL);
    }

    #[test]
    fn unknown_uses_official_registry_and_never_blocks() {
        let policy = resolve_sources(RegionCode::Unknown);
        assert_eq!(policy.npm_registry, REGISTRY_OFFICIAL);
        assert!(!policy.region.is_cn());
    }
}
