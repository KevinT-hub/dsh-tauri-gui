//! Per-setup-flow session state: cancellation and progress. One session is
//! created per `begin_setup` / `run_detection` invocation; the frontend can
//! cancel a long-running install and re-run detection afterwards.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

#[derive(Default)]
pub struct SetupSession {
    cancelled: AtomicBool,
    active: AtomicBool,
}

impl SetupSession {
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    pub fn begin(self: &Arc<Self>) -> bool {
        !self.active.swap(true, Ordering::SeqCst)
    }

    pub fn finish(self: &Arc<Self>) {
        self.active.store(false, Ordering::SeqCst);
        self.cancelled.store(false, Ordering::SeqCst);
    }

    /// Reserved for the install flow: a cancelled session makes long-running
    /// installs abort at their next safe checkpoint. Kept as a stable
    /// extension boundary for the setup UI.
    #[allow(dead_code)]
    pub fn cancel(self: &Arc<Self>) {
        self.cancelled.store(true, Ordering::SeqCst);
    }

    #[allow(dead_code)]
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }
}
