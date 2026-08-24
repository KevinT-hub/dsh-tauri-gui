//! Source-policy tests: cn/world/unknown map to the right registries.

use crate::detection::sources::{self, REGISTRY_CN, REGISTRY_OFFICIAL};
use crate::geo::model::RegionCode;

#[test]
fn cn_resolves_to_mirror() {
    let policy = sources::resolve_sources(RegionCode::Cn);
    assert_eq!(policy.npm_registry, REGISTRY_CN);
    assert!(policy.region.is_cn());
}

#[test]
fn world_resolves_to_official() {
    let policy = sources::resolve_sources(RegionCode::World);
    assert_eq!(policy.npm_registry, REGISTRY_OFFICIAL);
}

#[test]
fn unknown_resolves_to_official_without_blocking() {
    let policy = sources::resolve_sources(RegionCode::Unknown);
    assert_eq!(policy.npm_registry, REGISTRY_OFFICIAL);
}
