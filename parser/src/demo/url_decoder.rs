use axum::{http::StatusCode, Json};
use base64::engine::general_purpose::URL_SAFE;
use base64::Engine;
use regex::Regex;
use std::path::PathBuf;
use tracing::error;

use crate::config::Config;

/// Decode a base64-encoded demo URL
pub fn decode_demo_url(demo_url: &str) -> Result<String, (StatusCode, Json<serde_json::Value>)> {
    let decoded_url_bytes = match URL_SAFE.decode(demo_url) {
        Ok(bytes) => bytes,
        Err(e) => {
            error!("[parse_demo] Failed to decode base64 demo_url: {}", e);
            return Err((
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": format!("Invalid base64 in demo_url: {}", e)})),
            ));
        }
    };
    Ok(String::from_utf8_lossy(&decoded_url_bytes).into_owned())
}

/// Extract filename from URL/path and resolve it to the correct replays dir.
///
/// Accepts three shapes:
///   - `.../{id}.dem.bz2` or `.../{id}_{pid}.dem.bz2` -- compressed, goes to compressed_replays_dir
///   - `.../{id}.dem`     or `.../{id}_{pid}.dem`     -- decompressed, goes to replays_dir
///
/// Returns `(filename, resolved_path)`. The parse_demo handler uses the `.bz2`
/// suffix on `filename` to decide whether to run the download+decompress
/// pipeline or skip straight to parsing.
pub fn setup_replay_path(
    decoded_url: &str,
) -> Result<(PathBuf, PathBuf), (StatusCode, Json<serde_json::Value>)> {
    // Try compressed first since it's the CDN URL shape we download from.
    let re_compressed = Regex::new(r"(\d+(?:_\d+)?\.dem\.bz2)").expect("static regex");
    let re_decompressed = Regex::new(r"(\d+(?:_\d+)?\.dem)(?:$|[^.])").expect("static regex");

    let config = Config::from_env();

    if let Some(cap) = re_compressed.captures(decoded_url).and_then(|c| c.get(1)) {
        let filename = PathBuf::from(cap.as_str());
        let path = config.compressed_replays_dir.join(&filename);
        return Ok((filename, path));
    }

    if let Some(cap) = re_decompressed.captures(decoded_url).and_then(|c| c.get(1)) {
        let filename = PathBuf::from(cap.as_str());
        let path = config.replays_dir.join(&filename);
        return Ok((filename, path));
    }

    error!("[parse_demo] Could not extract filename from URL: {}", decoded_url);
    Err((
        StatusCode::BAD_REQUEST,
        Json(serde_json::json!({
            "error": format!("Could not extract filename from URL: {}", decoded_url)
        })),
    ))
}
