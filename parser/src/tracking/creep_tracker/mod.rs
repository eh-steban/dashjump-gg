//! Per-creep lane tracking for lane pressure analysis.
//!
//! ## Design decisions
//!
//! ### Why spawn-time grouping instead of spatial clustering
//!
//! The previous implementation used spatial clustering (`cluster_creeps`) with a 1000-unit
//! distance threshold to group creeps into waves. This caused two bugs:
//!
//! 1. **False second wave during zipline descent.** When a wave of 4 creeps rides the zipline
//!    and touches down in lane, their individual trajectories briefly diverge before they
//!    converge into marching formation. During that 1-2 second window the spread can exceed
//!    the cluster threshold, causing the algorithm to generate two clusters ("wave 0" and
//!    "wave 1") for what is a single wave. Once the creeps converge the second cluster
//!    disappears, and when the real second wave arrives it gets index 0 -- producing unstable
//!    wave identities across time.
//!
//! 2. **Dead creeps not removed from wave identity.** The spatial approach re-clusters
//!    surviving creeps each second, so a cluster's key (`"lane_team_waveIdx"`) could shift
//!    after some creeps die, making it impossible to correlate a wave snapshot at second 50
//!    with one at second 60.
//!
//! Spawn-time grouping fixes both bugs: a wave_id is assigned at creep CREATE time and never
//! changes. Two creeps belong to the same wave if they spawned within 5 seconds of each other
//! in the same (lane, team). This is robust to zipline spread because wave identity is fixed
//! at spawn, not recomputed from position each tick.
//!
//! ### Entity lifecycle in Deadlock demos
//!
//! - CREATE fires once when the entity enters the interest management scope of the demo
//!   recorder. For lane creeps this happens close to actual spawn (within 1-2 ticks).
//! - DELETE fires when the entity is removed -- either on death or when it leaves scope.
//!   In practice, lane creeps DELETE on death because the lane is always in scope.
//! - The same entity_index is NOT reused within a match by Deadlock (confirmed by the
//!   absence of duplicate-index events in the existing codebase and haste documentation).
//! - Pre-match entities have lane == 0 (filtered in `replay_parser.rs::should_track_position`
//!   and also gated by `match_started` in `on_entity`).

use std::collections::HashMap;

use tracing::{debug, info};

use crate::domain::{CreepSnapshot, CreepTimeline, LaneCreepData, WaveMeta};

/// Maximum gap in seconds between consecutive creep spawns in the same (lane, team) for them
/// to be assigned to the same wave. Waves fire every ~30 seconds; 5 seconds gives a safe
/// window to group the 4 creeps in a single wave without spanning into the next wave.
const WAVE_GROUPING_WINDOW_S: u32 = 5;

/// Maximum world-unit distance between a player and a creep for the player to be considered
/// "nearby" -- i.e., contesting that creep's lane position.
const NEARBY_PLAYER_RADIUS: f32 = 1500.0;

/// Internal state for a creep that is currently alive.
#[derive(Debug, Clone)]
struct ActiveCreep {
    entity_index: i32,
    lane: i32,
    team: u32,
    x: f32,
    y: f32,
    wave_id: String,
}

/// Tracks individual lane creep (trooper) entities throughout the match.
///
/// Produces a `LaneCreepData` with per-creep match-relative timelines and per-wave metadata.
#[derive(Debug)]
pub struct CreepTracker {
    /// Pre-computed field key for `m_iLane` -- retained for constructor compatibility;
    /// lane is now passed directly to each method rather than read from the entity here.
    _lane_key: u64,

    /// Currently alive creeps by entity_index
    active_creeps: HashMap<i32, ActiveCreep>,

    /// Match-relative timeline per creep. Key: entity_index.
    /// Index i in the Vec corresponds to match second i; None = dead or not yet spawned.
    creep_timelines: HashMap<i32, CreepTimeline>,

    /// Wave-level metadata. Key: wave_id "lane_team_spawnsec".
    wave_meta: HashMap<String, WaveMeta>,

    /// Last spawn info per (lane, team): (spawn_sec, wave_id).
    /// Used to assign new creeps to an existing wave if they spawn within the grouping window.
    wave_last_spawn: HashMap<(i32, u32), (u32, String)>,

    /// Match-relative second of the last emitted snapshot. Used to pad timelines when
    /// multiple seconds elapse between snapshots (e.g., periods with no tick boundary).
    last_snapshot_sec: u32,
}

impl CreepTracker {
    pub fn new(lane_key: u64) -> Self {
        Self {
            _lane_key: lane_key,
            active_creeps: HashMap::new(),
            creep_timelines: HashMap::new(),
            wave_meta: HashMap::new(),
            wave_last_spawn: HashMap::new(),
            last_snapshot_sec: 0,
        }
    }

    /// Returns true if the entity type hash is a lane creep.
    pub fn is_creep_entity(hash: u64) -> bool {
        use crate::entities::constants::CNPC_TROOPER_ENTITY;
        hash == CNPC_TROOPER_ENTITY
    }

    /// Handle creep entity creation.
    ///
    /// Assigns the creep to an existing wave or creates a new one based on spawn time.
    /// `match_sec` is the current match-relative second (0 = game start).
    pub fn handle_creep_create(
        &mut self,
        entity_index: i32,
        lane: i32,
        team: u32,
        x: f32,
        y: f32,
        match_sec: u32,
    ) {
        let wave_id = self.assign_wave(lane, team, match_sec);

        debug!(
            "[creep_tracker] CREATE entity={} lane={} team={} wave_id={} pos=({:.0},{:.0})",
            entity_index, lane, team, wave_id, x, y
        );

        let creep = ActiveCreep {
            entity_index,
            lane,
            team,
            x,
            y,
            wave_id,
        };

        self.active_creeps.insert(entity_index, creep);
        // Pre-create the timeline entry so it exists even if the creep never gets a snapshot
        self.creep_timelines.entry(entity_index).or_default();
    }

    /// Handle creep entity deletion (death or scope exit).
    ///
    /// Removes the creep from active tracking. If this was the last alive creep in its wave,
    /// pins `last_death_sec`/`last_death_x`/`last_death_y` on the wave metadata.
    pub fn handle_creep_delete(&mut self, entity_index: i32, match_sec: u32) {
        let Some(dying) = self.active_creeps.remove(&entity_index) else {
            debug!(
                "[creep_tracker] DELETE entity={} -- not in active_creeps (already removed or pre-match)",
                entity_index
            );
            return;
        };

        let dying_wave_id = dying.wave_id.clone();
        let dying_x = dying.x;
        let dying_y = dying.y;

        debug!(
            "[creep_tracker] DELETE entity={} wave_id={} match_sec={} pos=({:.0},{:.0})",
            entity_index, dying_wave_id, match_sec, dying_x, dying_y
        );

        // Check if this was the last alive creep in the wave
        let alive_in_wave = self
            .active_creeps
            .values()
            .filter(|c| c.wave_id == dying_wave_id)
            .count();

        if alive_in_wave == 0 {
            if let Some(meta) = self.wave_meta.get_mut(&dying_wave_id) {
                meta.last_death_sec = Some(match_sec);
                meta.last_death_x = Some(dying_x);
                meta.last_death_y = Some(dying_y);

                debug!(
                    "[creep_tracker] Wave {} fully dead at sec={} pos=({:.0},{:.0})",
                    dying_wave_id, match_sec, dying_x, dying_y
                );
            }
        }
    }

    /// Handle position update for a creep that is already active.
    ///
    /// If the creep is not yet in `active_creeps` (e.g., pre-existing entity picked up mid
    /// stream), it is registered with a fresh wave assignment.
    pub fn handle_creep_update(
        &mut self,
        entity_index: i32,
        lane: i32,
        team: u32,
        x: f32,
        y: f32,
        match_sec: u32,
    ) {
        if let Some(creep) = self.active_creeps.get_mut(&entity_index) {
            // Always update position for registered creeps. Lane may transiently read 0
            // during the zipline-to-walking transition; blocking on lane=0 here would freeze
            // the creep at its landing position.
            creep.x = x;
            creep.y = y;
        } else if lane != 0 {
            // Creep entered interest scope without a preceding CREATE -- treat as late create.
            // Only register if lane is assigned; lane=0 means the entity is still pre-spawn
            // or in-transit and should not be tracked yet.
            debug!(
                "[creep_tracker] UPDATE entity={} not in active_creeps -- treating as late CREATE (lane={})",
                entity_index, lane
            );
            self.handle_creep_create(entity_index, lane, team, x, y, match_sec);
        } else {
            debug!(
                "[creep_tracker] UPDATE entity={} not in active_creeps with lane=0 -- skipping",
                entity_index
            );
        }
    }

    /// Build per-second snapshots for all active and dead creeps at `match_sec`.
    ///
    /// - Active creeps receive a `Some(CreepSnapshot)` with current position and nearby players.
    /// - Creeps in `creep_timelines` that are NOT active receive `None` (they are dead).
    /// - Any gap between `last_snapshot_sec` and `match_sec` is padded with `None` entries.
    ///
    /// `player_positions`: slice of `(custom_id, x, y)` for all currently visible players.
    /// `match_sec`: match-relative second (0 = game start).
    pub fn build_creep_snapshot(&mut self, match_sec: u32, player_positions: &[(i32, f32, f32)]) {
        // Pad all timelines for any seconds we skipped (gap > 1)
        // last_snapshot_sec tracks the last second that was fully emitted.
        // If match_sec == last_snapshot_sec + 1 we have no gap to fill.
        // Initialisation: last_snapshot_sec starts at 0, and the first real call is typically 0.
        // We use a sentinel: the very first call sets last_snapshot_sec to match_sec.

        let gap_start = if self.last_snapshot_sec == 0 && self.creep_timelines.is_empty() {
            // First ever call -- no padding needed; just emit match_sec
            match_sec
        } else {
            self.last_snapshot_sec + 1
        };

        // Pad seconds [gap_start .. match_sec) with None for all known creeps
        for pad_sec in gap_start..match_sec {
            debug!(
                "[creep_tracker] Padding second {} with None for {} known creeps",
                pad_sec,
                self.creep_timelines.len()
            );
            for timeline in self.creep_timelines.values_mut() {
                // Extend only if timeline is shorter than the padded index
                while timeline.len() <= pad_sec as usize {
                    timeline.push(None);
                }
            }
        }

        // Now emit snapshots for match_sec
        let active_indices: Vec<i32> = self.active_creeps.keys().copied().collect();

        for entity_index in &active_indices {
            let (lane, team, x, y, wave_id) = {
                let c = &self.active_creeps[entity_index];
                (c.lane, c.team, c.x, c.y, c.wave_id.clone())
            };

            let nearby_players = Self::compute_nearby_players(x, y, player_positions);

            let snapshot = CreepSnapshot {
                x,
                y,
                lane,
                team,
                wave_id,
                nearby_players,
            };

            let timeline = self.creep_timelines.entry(*entity_index).or_default();
            // Fill any gap up to match_sec (handles first-snapshot case)
            while timeline.len() < match_sec as usize {
                timeline.push(None);
            }
            if timeline.len() == match_sec as usize {
                timeline.push(Some(snapshot));
            } else {
                // Already has an entry for this second (shouldn't happen, but overwrite to be safe)
                timeline[match_sec as usize] = Some(snapshot);
            }
        }

        // All dead creeps (in creep_timelines but not active) get None at match_sec
        let all_known: Vec<i32> = self.creep_timelines.keys().copied().collect();
        for entity_index in all_known {
            if self.active_creeps.contains_key(&entity_index) {
                continue; // already handled above
            }
            let timeline = self.creep_timelines.get_mut(&entity_index).unwrap();
            while timeline.len() < match_sec as usize {
                timeline.push(None);
            }
            if timeline.len() == match_sec as usize {
                timeline.push(None);
            }
        }

        self.last_snapshot_sec = match_sec;
    }

    /// Get the final output for JSON serialization.
    pub fn get_output(&self) -> LaneCreepData {
        let total_creeps = self.creep_timelines.len();
        let total_waves = self.wave_meta.len();
        let non_null: usize = self
            .creep_timelines
            .values()
            .flat_map(|t| t.iter())
            .filter(|s| s.is_some())
            .count();

        info!(
            "[creep_tracker] Output: {} creeps, {} waves, {} non-null snapshots",
            total_creeps, total_waves, non_null
        );

        LaneCreepData {
            creeps: self
                .creep_timelines
                .iter()
                .map(|(k, v)| (k.to_string(), v.clone()))
                .collect(),
            wave_meta: self.wave_meta.clone(),
        }
    }

    // -------------------------------------------------------------------------
    // Private helpers
    // -------------------------------------------------------------------------

    /// Assign a creep to a wave based on spawn time.
    ///
    /// If a wave already exists for `(lane, team)` and the last spawn was within
    /// `WAVE_GROUPING_WINDOW_S` seconds, the creep joins that wave.
    /// Otherwise a new wave is created with `wave_id = "lane_team_spawnsec"`.
    fn assign_wave(&mut self, lane: i32, team: u32, current_sec: u32) -> String {
        let key = (lane, team);

        if let Some((last_sec, wave_id)) = self.wave_last_spawn.get(&key) {
            if current_sec.saturating_sub(*last_sec) <= WAVE_GROUPING_WINDOW_S {
                return wave_id.clone();
            }
        }

        // New wave
        let wave_id = format!("{}_{}_{}", lane, team, current_sec);
        self.wave_last_spawn
            .insert(key, (current_sec, wave_id.clone()));
        self.wave_meta.insert(
            wave_id.clone(),
            WaveMeta {
                lane,
                team,
                spawn_sec: current_sec,
                last_death_sec: None,
                last_death_x: None,
                last_death_y: None,
            },
        );
        wave_id
    }

    /// Compute which player custom_ids are within `NEARBY_PLAYER_RADIUS` of `(cx, cy)`.
    fn compute_nearby_players(cx: f32, cy: f32, player_positions: &[(i32, f32, f32)]) -> Vec<i32> {
        let radius_sq = NEARBY_PLAYER_RADIUS * NEARBY_PLAYER_RADIUS;
        player_positions
            .iter()
            .filter_map(|&(id, px, py)| {
                let dx = cx - px;
                let dy = cy - py;
                if dx * dx + dy * dy <= radius_sq {
                    Some(id)
                } else {
                    None
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests;
