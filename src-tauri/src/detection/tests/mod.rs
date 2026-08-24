//! Detection domain tests (compile-time gated). These test pure logic only:
//! version parsing, gate aggregation and source policy. No real user PATH,
//! no real dsh home, no network.

pub mod aggregation;
pub mod probes;
pub mod sources;
pub mod versions;
