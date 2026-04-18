use axum::{Json, http::{StatusCode, header, HeaderValue}, response::Response};
use serde::Deserialize;
use tracing::{info, error};
use dashmap::DashMap;
use once_cell::sync::Lazy;
use tokio::sync::Mutex;
use std::sync::Arc;

use crate::demo::{decode_demo_url, setup_replay_path, download_if_needed, decompress_replay};
use crate::replay_parser;

static FILE_MUTEXES: Lazy<DashMap<String, Arc<Mutex<()>>>> = Lazy::new(DashMap::new);

#[derive(Deserialize)]
pub struct ParseRequest {
    pub demo_url: String,
}

pub async fn parse_demo(Json(payload): Json<ParseRequest>) -> Response {
    info!("[parse_demo] Received request to parse demo with URL: {}", payload.demo_url);

    let decoded_url = match decode_demo_url(&payload.demo_url) {
        Ok(url) => url,
        Err((status, Json(val))) => return build_response(status, &val),
    };

    let (filename, replay_path) = match setup_replay_path(&decoded_url) {
        Ok(v) => v,
        Err((status, Json(val))) => return build_response(status, &val),
    };

    // Acquire the mutex for this file
    let key = filename.to_string_lossy().to_string();
    let mutex = FILE_MUTEXES
        .entry(key.clone())
        .or_insert_with(|| Arc::new(Mutex::new(())))
        .clone();
    let _guard = mutex.lock().await;

    // Compressed inputs go through the download+decompress pipeline; decompressed
    // inputs (local .dem already in replays_dir) skip straight to parsing.
    let is_compressed = filename
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.eq_ignore_ascii_case("bz2"))
        .unwrap_or(false);

    let decompressed_path = if is_compressed {
        if let Err((status, Json(val))) = download_if_needed(&decoded_url, &replay_path).await {
            return build_response(status, &val);
        }
        match decompress_replay(&replay_path, &filename) {
            Ok(path) => path,
            Err((status, Json(val))) => return build_response(status, &val),
        }
    } else {
        if !replay_path.exists() {
            error!("[parse_demo] Decompressed demo not found at {}", replay_path.display());
            return build_response(
                StatusCode::NOT_FOUND,
                &serde_json::json!({
                    "error": format!("Decompressed demo not found: {}", replay_path.display())
                }),
            );
        }
        replay_path.clone()
    };

    info!("[parse_demo] Starting replay parsing for: {}", decompressed_path.display());

    // Use catch_unwind to handle panics from the haste library gracefully
    // This prevents crashes from replay format changes or library bugs
    let path_str = decompressed_path.to_str().unwrap().to_string();
    match replay_parser::parse_replay(&path_str).await {
        Ok(json) => {
            info!("[parse_demo] Replay parsed successfully.");
            build_response(StatusCode::OK, &json)
        },
        Err(e) => {
            error!("[parse_demo] replay_parser::parse_replay failed: {:?}", e);
            let val = serde_json::json!({
                "error": format!("Failed to parse replay: {}", e)
            });
            build_response(StatusCode::INTERNAL_SERVER_ERROR, &val)
        },
    }
}

pub fn build_response(status: StatusCode, value: &serde_json::Value) -> Response {
    // Pre-serialize to capture uncompressed size; CompressionLayer will handle gzip/deflate based on Accept-Encoding
    let body_str = serde_json::to_string(value).unwrap_or_else(|_| "{}".to_string());
    let uncompressed_len = body_str.len();
    let resp = Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, HeaderValue::from_static("application/json"))
        .header("X-Uncompressed-Size", HeaderValue::from_str(&uncompressed_len.to_string()).unwrap_or(HeaderValue::from_static("0")))
        .body(axum::body::Body::from(body_str))
        .unwrap();
    // Note: Content-Encoding will be added automatically by CompressionLayer when appropriate
    resp
}
