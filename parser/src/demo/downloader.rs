use axum::{http::StatusCode, Json};
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use tracing::{info, error};

/// Download a demo file from URL if it doesn't already exist locally
///
/// # Arguments
/// * `decoded_url` - The URL to download from
/// * `replay_path` - The local path where the file should be saved
///
/// # Returns
/// * `Ok(())` if file already exists or download succeeds
/// * `Err((StatusCode, Json))` if download or write fails
pub async fn download_if_needed(
    decoded_url: &str,
    replay_path: &PathBuf,
) -> Result<(), (StatusCode, Json<serde_json::Value>)> {
    if Path::new(replay_path).exists() {
        info!(
            "[parse_demo] Replay file already exists at {}. Skipping download.",
            replay_path.display()
        );
        return Ok(());
    }

    info!(
        "[parse_demo] Downloading replay from URL: {} to the replay path: {}",
        decoded_url,
        replay_path.display()
    );

    let bytes = match reqwest::get(decoded_url).await {
        Ok(resp) => match resp.bytes().await {
            Ok(b) => b,
            Err(e) => {
                error!("[parse_demo] Failed to read bytes from response: {}", e);
                return Err((
                    StatusCode::BAD_GATEWAY,
                    Json(serde_json::json!({"error": format!("Failed to read bytes from response: {}", e)})),
                ));
            }
        },
        Err(e) => {
            error!("[parse_demo] Failed to download demo from URL: {}", e);
            return Err((
                StatusCode::BAD_GATEWAY,
                Json(serde_json::json!({"error": format!("Failed to download demo: {}", e)})),
            ));
        }
    };

    let bytes_len = bytes.len();
    if let Err(e) = File::create(replay_path).and_then(|mut file| file.write_all(&bytes)) {
        error!("[parse_demo] Failed to write replay to file: {}", e);
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": format!("Failed to write file: {}", e)})),
        ));
    }

    info!(
        "[parse_demo] Download complete: {} ({} bytes)",
        replay_path.display(),
        bytes_len
    );

    Ok(())
}
