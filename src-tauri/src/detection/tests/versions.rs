//! Version-requirement tests for the detection gate.

use crate::core::version;
use crate::detection::requirement;

#[test]
fn node_requirement_accepts_official_window() {
    assert!(version::node_supported("22.19.0"));
    assert!(version::node_supported("22.99.0"));
    assert!(version::node_supported("24.0.0"));
    assert!(version::node_supported("24.1.2"));
}

#[test]
fn node_requirement_rejects_outside_window() {
    assert!(!version::node_supported("22.18.9"));
    assert!(!version::node_supported("20.11.1"));
    assert!(!version::node_supported("23.0.0"));
    assert!(!version::node_supported("garbage"));
}

#[test]
fn requirement_text_mentions_the_engine_window() {
    assert!(requirement::NODE_REQUIREMENT.contains("22.19"));
}
