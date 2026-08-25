//! Startup orchestration for the desktop shell.
//!
//! This module owns the fast-path decision. `lib.rs` only wires Tauri events;
//! user-facing status labels stay in the frontend status mapper.

use crate::app::state::AppState;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use tauri::AppHandle;

pub fn start_cached_engine(app: &AppHandle, state: Arc<AppState>) {
    if crate::app::config::checklist_required_full(&state)
        || !state.config.lock().unwrap().auto_start_engine
    {
        return;
    }
    let Some(spec) = crate::app::toolchain_cache::load(&state.home) else {
        return;
    };
    *state.command_spec.lock().unwrap() = Some(spec);

    let app = app.clone();
    std::thread::spawn(move || {
        let version = state
            .command_spec
            .lock()
            .unwrap()
            .as_ref()
            .map(|spec| spec.dsh_version.clone());
        crate::app::set_status(
            &state,
            Some(&app),
            "engine-starting",
            "engineStarting",
            None,
            None,
            None,
            version,
        );
        state.stopping.store(false, Ordering::SeqCst);
        if let Err(err) = crate::engine::connect_existing_or_spawn(&app, &state) {
            crate::app::set_status(
                &state,
                Some(&app),
                "error",
                "engineStartFailed",
                Some(err),
                None,
                None,
                None,
            );
        }
    });
}
