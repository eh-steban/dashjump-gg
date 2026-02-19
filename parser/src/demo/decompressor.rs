use axum::{http::StatusCode, Json};
use bzip2::read::BzDecoder;
use std::fs::File;
use std::path::{Path, PathBuf};
use tracing::{info, error};

use crate::config::Config;

/// Decompress a bz2-compressed replay file
///
/// # Arguments
/// * `replay_path` - Path to the compressed .dem.bz2 file
/// * `filename` - Original filename (used to derive decompressed name)
///
/// # Returns
/// * `Ok(PathBuf)` - Path to the decompressed .dem file
/// * `Err((StatusCode, Json))` - Error details if decompression fails
pub fn decompress_replay(
    replay_path: &PathBuf,
    filename: &PathBuf,
) -> Result<PathBuf, (StatusCode, Json<serde_json::Value>)> {
    let decompressed_filename = filename.file_stem().unwrap_or_else(|| filename.as_os_str());
    let config = Config::from_env();
    let decompressed_path = config.replays_dir.join(decompressed_filename);

    if Path::new(&decompressed_path).exists() {
        info!(
            "[parse_demo] Decompressed replay file already exists at {}. Skipping decompression.",
            decompressed_path.display()
        );
        return Ok(decompressed_path);
    }

    info!(
        "[parse_demo] Decompressing replay to {}",
        decompressed_path.display()
    );

    let compressed_file = match File::open(replay_path) {
        Ok(f) => f,
        Err(e) => {
            error!("[parse_demo] Failed to open compressed file: {}", e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": format!("Failed to open compressed file: {}", e)})),
            ));
        }
    };

    let mut decoder = BzDecoder::new(compressed_file);
    let mut decompressed_file = match File::create(decompressed_path.clone()) {
        Ok(f) => f,
        Err(e) => {
            error!("[parse_demo] Failed to create decompressed file: {}", e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": format!("Failed to create decompressed file: {}", e)})),
            ));
        }
    };

    let bytes_written = match std::io::copy(&mut decoder, &mut decompressed_file) {
        Ok(bytes) => bytes,
        Err(e) => {
            error!("[parse_demo] Failed to decompress file: {}", e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": format!("Failed to decompress file: {}", e)})),
            ));
        }
    };

    info!(
        "[parse_demo] Decompression complete: {} ({} bytes)",
        decompressed_path.display(),
        bytes_written
    );

    Ok(decompressed_path)
}
