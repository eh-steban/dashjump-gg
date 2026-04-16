use axum::{Json, extract::Path as AxumPath};
use serde::Serialize;
use std::path::Path;
use tracing::{info, warn};

use crate::config::Config;

#[derive(Serialize)]
pub struct CheckDemoResponse {
    pub available: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filename: Option<String>,
}

/// Check if a demo file exists locally for the given match ID.
///
/// Scans both the decompressed replays dir (`.dem`) and the compressed
/// replays dir (`.dem.bz2`). Matches filenames of the form `{match_id}.<ext>`
/// or `{match_id}_*.<ext>` -- the underscore/dot guard prevents match-id
/// prefix collisions (e.g. 7597223 vs 75972233).
///
/// Prefers decompressed when both exist so the parse path can skip the
/// bz2 decompression step. Returns `available: false` on read errors so the
/// backend can fall back to the Deadlock API.
pub async fn check_demo(AxumPath(match_id): AxumPath<u64>) -> Json<CheckDemoResponse> {
    info!("[check_demo] Checking for local demo file for match_id: {}", match_id);

    let config = Config::from_env();
    let match_id_str = match_id.to_string();

    if let Some(filename) = find_match_file(&config.replays_dir, &match_id_str, ".dem") {
        info!("[check_demo] Found decompressed demo: {}", filename);
        return Json(CheckDemoResponse { available: true, filename: Some(filename) });
    }

    if let Some(filename) = find_match_file(&config.compressed_replays_dir, &match_id_str, ".dem.bz2") {
        info!("[check_demo] Found compressed demo: {}", filename);
        return Json(CheckDemoResponse { available: true, filename: Some(filename) });
    }

    info!("[check_demo] Parser does not have local demo for match_id: {}", match_id);
    Json(CheckDemoResponse { available: false, filename: None })
}

/// Return the first filename in `dir` whose name is `{match_id}{suffix}` or
/// `{match_id}_*{suffix}`. The `_` / `.` boundary prevents prefix collisions.
fn find_match_file(dir: &Path, match_id: &str, suffix: &str) -> Option<String> {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(e) => {
            warn!("[check_demo] Cannot read {}: {}", dir.display(), e);
            return None;
        }
    };

    for entry in entries.flatten() {
        let file_name = entry.file_name();
        let name = file_name.to_string_lossy();

        if !name.ends_with(suffix) {
            continue;
        }

        let Some(rest) = name.strip_prefix(match_id) else {
            continue;
        };

        if rest == suffix || rest.starts_with('_') {
            return Some(name.into_owned());
        }
    }

    None
}
