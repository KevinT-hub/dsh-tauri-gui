//! Geo commands: country/region state and the resolved source policy for
//! the setup screen. `geo.rs` itself never installs, never writes user
//! config and never exposes the user's IP — only the normalized region.

use crate::app::state::AppState;
use crate::detection;
use serde::Serialize;
use std::sync::Arc;
use tauri::{AppHandle, Manager};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GeoState {
    pub geo: crate::geo::model::GeoResult,
    pub source_policy: detection::SourcePolicy,
}

/// Return the cached/current region and the matching source policy. Uses the
/// in-process short-TTL cache; a geo failure yields `unknown` and the
/// official-source policy (never blocks the UI).
#[tauri::command]
pub fn get_geo_state(app: AppHandle) -> GeoState {
    let state: Arc<AppState> = app.state::<Arc<AppState>>().inner().clone();
    let geo = crate::geo::resolve(&state.geo_cache);
    let source_policy = detection::resolve_sources(geo.region);
    GeoState { geo, source_policy }
}
