use super::netprobe;
use crate::app::AppState;
use base64::Engine;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::io::Read;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};
use tauri_plugin_updater::UpdaterExt;

const SPEED_PROBE_SECS: f64 = 5.0;
const MIN_SPEED_BYTES_PER_SEC: f64 = 64.0 * 1024.0;
const STALL_TIMEOUT: Duration = Duration::from_secs(20);

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdaterDownloadProgressEvent {
    pub event: String,
    pub content_length: Option<u64>,
    pub chunk_length: Option<u64>,
    pub downloaded: u64,
    pub total: Option<u64>,
    pub percentage: Option<u64>,
}

pub async fn download_and_install(app: AppHandle, state: Arc<AppState>) -> Result<(), String> {
    let endpoint_urls = netprobe::latest_json_candidates()
        .iter()
        .filter_map(|url| tauri::Url::parse(url).ok())
        .collect::<Vec<_>>();

    let mut builder = app.updater_builder();
    builder = builder
        .endpoints(endpoint_urls)
        .map_err(|e| format!("updater endpoints: {e}"))?;
    let updater = builder.build().map_err(|e| format!("updater build: {e}"))?;

    let update = updater
        .check()
        .await
        .map_err(|e| format!("update check: {e}"))?;
    let Some(update) = update else {
        return Ok(());
    };

    let target =
        tauri_plugin_updater::target().ok_or_else(|| "Unsupported update target".to_string())?;
    let download_url = update.download_url.to_string();
    if !netprobe::is_official_download_url(&download_url) {
        return Err("Refusing update URL outside the official GitHub release".to_string());
    }
    let expected_sha256 = update.raw_json["platforms"][&target]["sha256"]
        .as_str()
        .map(str::to_string)
        .ok_or_else(|| format!("sha256 is missing in latest.json for platform {target}"))?;

    state
        .logger
        .info(&format!("downloading app update {}", update.version));

    let downloaded = AtomicU64::new(0);
    let total = AtomicU64::new(0);
    let started = AtomicBool::new(false);
    let handle = app.clone();
    let finish_handle = handle.clone();

    let mut on_chunk = move |chunk_length: usize, content_length: Option<u64>| {
        if let Some(len) = content_length {
            total.store(len, Ordering::Relaxed);
        }
        let d = downloaded.fetch_add(chunk_length as u64, Ordering::Relaxed) + chunk_length as u64;
        let t = total.load(Ordering::Relaxed);
        let percentage = if t > 0 {
            ((d as f64 / t as f64) * 100.0).min(100.0).round() as u64
        } else {
            0
        };

        if !started.swap(true, Ordering::Relaxed) {
            let _ = handle.emit(
                crate::app::events::UPDATER_DOWNLOAD_EVENT,
                UpdaterDownloadProgressEvent {
                    event: "Started".into(),
                    content_length,
                    chunk_length: None,
                    downloaded: 0,
                    total: content_length,
                    percentage: Some(0),
                },
            );
        }

        let _ = handle.emit(
            crate::app::events::UPDATER_DOWNLOAD_EVENT,
            UpdaterDownloadProgressEvent {
                event: "Progress".into(),
                content_length: None,
                chunk_length: Some(chunk_length as u64),
                downloaded: d,
                total: if t > 0 { Some(t) } else { None },
                percentage: Some(percentage),
            },
        );
    };

    let bytes = download_with_mirrors(&download_url, &expected_sha256, &mut on_chunk)?;

    let _ = finish_handle.emit(
        crate::app::events::UPDATER_DOWNLOAD_EVENT,
        UpdaterDownloadProgressEvent {
            event: "Finished".into(),
            content_length: None,
            chunk_length: None,
            downloaded: 1,
            total: Some(1),
            percentage: Some(100),
        },
    );

    verify_minisign(&bytes, &update.signature, netprobe::UPDATE_PUBKEY)?;
    update
        .install(&bytes)
        .map_err(|e| format!("update install: {e}"))?;
    Ok(())
}

fn download_with_mirrors<F>(
    primary_url: &str,
    expected_sha256: &str,
    on_chunk: &mut F,
) -> Result<Vec<u8>, String>
where
    F: FnMut(usize, Option<u64>),
{
    let candidates = netprobe::installer_candidates(primary_url);
    let mut last_error = None;

    for url in candidates {
        match download_one(&url, expected_sha256, on_chunk) {
            Ok(bytes) => return Ok(bytes),
            Err(error) => {
                eprintln!("[updater] download source failed: {url}: {error}");
                last_error = Some(error);
            }
        }
    }

    Err(last_error.unwrap_or_else(|| "All download sources failed".to_string()))
}

fn download_one<F>(url: &str, expected_sha256: &str, on_chunk: &mut F) -> Result<Vec<u8>, String>
where
    F: FnMut(usize, Option<u64>),
{
    // Mirrors are untrusted: use a clean client without tokens, cookies or
    // forwarded authorization headers.
    let agent = crate::core::http::http_agent(None);
    let response = agent
        .get(url)
        .header("Accept", "application/octet-stream")
        .call()
        .map_err(|e| format!("{url}: {e}"))?;
    if !response.status().is_success() {
        return Err(format!("{url}: HTTP {}", response.status()));
    }

    let content_length = response.body().content_length();
    let start = Instant::now();
    let mut last_chunk = start;
    let mut buffer = Vec::new();
    let mut body = response.into_body();
    let mut reader = body.as_reader();
    let mut chunk = [0u8; 64 * 1024];

    loop {
        let read = reader.read(&mut chunk).map_err(|e| format!("{url}: {e}"))?;
        if read == 0 {
            break;
        }
        let now = Instant::now();

        let elapsed = now.duration_since(start).as_secs_f64();
        if elapsed >= SPEED_PROBE_SECS && (buffer.len() as f64 / elapsed) < MIN_SPEED_BYTES_PER_SEC
        {
            return Err(format!(
                "{url}: download too slow ({:.1} KiB/s)",
                buffer.len() as f64 / elapsed / 1024.0
            ));
        }
        if now.duration_since(last_chunk) > STALL_TIMEOUT {
            return Err(format!(
                "{url}: download stalled for {}s",
                STALL_TIMEOUT.as_secs()
            ));
        }

        last_chunk = now;
        on_chunk(read, content_length);
        buffer.extend_from_slice(&chunk[..read]);
    }

    let digest = hex::encode(Sha256::digest(&buffer));
    if !digest.eq_ignore_ascii_case(expected_sha256) {
        return Err(format!(
            "{url}: SHA-256 mismatch (expected {expected_sha256}, got {digest})"
        ));
    }

    Ok(buffer)
}

fn verify_minisign(bytes: &[u8], signature_b64: &str, pubkey_b64: &str) -> Result<(), String> {
    let pubkey_bytes = base64::engine::general_purpose::STANDARD
        .decode(pubkey_b64)
        .map_err(|e| format!("invalid updater pubkey: {e}"))?;
    let pubkey_str =
        String::from_utf8(pubkey_bytes).map_err(|e| format!("invalid updater pubkey: {e}"))?;
    let public_key = minisign_verify::PublicKey::decode(&pubkey_str)
        .map_err(|e| format!("invalid updater pubkey: {e}"))?;

    let signature_bytes = base64::engine::general_purpose::STANDARD
        .decode(signature_b64)
        .map_err(|e| format!("invalid update signature: {e}"))?;
    let signature_str =
        String::from_utf8(signature_bytes).map_err(|e| format!("invalid update signature: {e}"))?;
    let signature = minisign_verify::Signature::decode(&signature_str)
        .map_err(|e| format!("invalid update signature: {e}"))?;

    public_key
        .verify(bytes, &signature, true)
        .map_err(|e| format!("minisign verification failed: {e}"))
}
