pub mod checker;
pub mod downloader;
pub mod netprobe;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppUpdateInfo {
    pub available: bool,
    pub version: String,
    pub notes: String,
    pub date: String,
    pub download_url: String,
    pub sha256: String,
    pub source: String,
}
