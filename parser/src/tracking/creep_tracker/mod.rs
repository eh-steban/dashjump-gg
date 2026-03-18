use std::collections::HashMap;

use tracing::{debug, info};

use crate::domain::{CreepSnapshot, CreepTimeline, LaneCreepData, WaveMeta};
use crate::entities::constants::{
    CAGE_ENTITY_HEALTH, LIFE_ALIVE, LIFE_DEAD, NPC_STATE_ALERT, NPC_STATE_COMBAT,
    NPC_STATE_DEAD_CITADEL, NPC_STATE_DYING_CITADEL, NPC_STATE_IDLE, NPC_STATE_INERT,
    NPC_STATE_INIT, NPC_STATE_INVALID,
};

/// Maximum gap between consecutive cage-entity spawns for them to be assigned to the same wave.
/// Cage entities (health=1) launch in tight clusters of 4; a 5-second window safely groups them.
const CAGE_GROUPING_WINDOW_S: u32 = 5;

/// Maximum lookback for a real lane creep (health>1) to find and join a preceding cage wave.
/// Cage entities launch ~13-15 seconds before real creeps land. 18 seconds covers this gap
/// while staying safely below the ~30-second distance to the next cage wave launch.
const REAL_CREEP_GROUPING_WINDOW_S: u32 = 18;

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
    life_state: u8,
    npc_state: i32,
    is_cage: bool,
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
    /// `npc_state` is the current value of `m_NPCState`; if INERT, the creep is pre-spawn
    /// or recycling and registration is skipped.
    /// `life_state` is the current value of `m_lifeState` (0 = ALIVE, 2 = DEAD); stored on
    /// the active creep and used to detect entity reuse via DEAD->ALIVE transitions.
    /// `health` is the current value of `m_iHealth`; if equal to `CAGE_ENTITY_HEALTH` (1), the
    /// entity is a visual cage/zipline-carrier sprite. It is registered with `is_cage: true` so
    /// it appears on the minimap as a "wave inbound" indicator.
    pub fn handle_creep_create(
        &mut self,
        entity_index: i32,
        lane: i32,
        team: u32,
        x: f32,
        y: f32,
        match_sec: u32,
        npc_state: i32,
        life_state: u8,
        health: i32,
    ) {
        if matches!(npc_state, NPC_STATE_INERT | NPC_STATE_INIT | NPC_STATE_INVALID) {
            debug!(
                "[creep_tracker] CREATE entity={} -- npc_state={} (inactive), skipping registration",
                entity_index, npc_state
            );
            return;
        }

        let is_cage = health <= CAGE_ENTITY_HEALTH;
        if is_cage {
            debug!(
                "[creep_tracker] CREATE entity={} -- registering as cage entity (health=1)",
                entity_index
            );
        }

        let wave_id = self.assign_wave(lane, team, match_sec, is_cage);

        debug!(
            "[creep_tracker] CREATE entity={} lane={} team={} wave_id={} npc_state={} life_state={} health={} is_cage={} pos=({:.0},{:.0})",
            entity_index, lane, team, wave_id, npc_state, life_state, health, is_cage, x, y
        );

        let creep = ActiveCreep {
            entity_index,
            lane,
            team,
            x,
            y,
            wave_id,
            life_state,
            npc_state,
            is_cage,
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
    /// stream), it is registered with a fresh wave assignment provided it has an active state.
    ///
    /// For registered creeps:
    /// - A DEAD->ALIVE transition on `life_state` signals entity reuse for a new wave: the
    ///   old wave's death metadata is pinned (if this was the last creep in the wave) and the
    ///   entity receives a new `wave_id` via `assign_wave`.
    /// - All updates: position and life_state are updated in place (no eviction).
    ///   Snapshot suppression for DEAD creeps is handled in `build_creep_snapshot`.
    ///
    /// For unregistered creeps: only register when lane != 0 and npc_state is active.
    /// Cage entities (health <= CAGE_ENTITY_HEALTH) are registered with `is_cage: true`.
    pub fn handle_creep_update(
        &mut self,
        entity_index: i32,
        lane: i32,
        team: u32,
        x: f32,
        y: f32,
        match_sec: u32,
        npc_state: i32,
        life_state: u8,
        health: i32,
    ) {
        if self.active_creeps.contains_key(&entity_index) {
            // Read old life_state and wave_id while holding an immutable borrow.
            let (old_life_state, old_wave_id) = {
                let c = &self.active_creeps[&entity_index];
                (c.life_state, c.wave_id.clone())
            };

            if old_life_state == LIFE_DEAD && life_state == LIFE_ALIVE {
                // DEAD->ALIVE transition: the engine is reusing this entity slot for a new wave.
                // Pin death metadata on the old wave if this was the last registered creep in it.
                debug!(
                    "[creep_tracker] UPDATE entity={} DEAD->ALIVE -- pinning old wave {} and re-assigning",
                    entity_index, old_wave_id
                );

                let alive_in_wave = self
                    .active_creeps
                    .values()
                    .filter(|c| c.entity_index != entity_index && c.wave_id == old_wave_id)
                    .count();

                if alive_in_wave == 0 {
                    if let Some(meta) = self.wave_meta.get_mut(&old_wave_id) {
                        meta.last_death_sec = Some(match_sec);
                        meta.last_death_x = Some(x);
                        meta.last_death_y = Some(y);
                        debug!(
                            "[creep_tracker] Wave {} fully recycled (DEAD->ALIVE) at sec={} pos=({:.0},{:.0})",
                            old_wave_id, match_sec, x, y
                        );
                    }
                }

                // Assign a new wave for the recycled entity. `assign_wave` mutably borrows self,
                // so the immutable borrow on `old_wave_id` must already be dropped (it is, since
                // it was cloned above). Read is_cage from the stored active creep before the
                // mutable borrow.
                let recycled_is_cage = self.active_creeps[&entity_index].is_cage;
                let new_wave_id = self.assign_wave(lane, team, match_sec, recycled_is_cage);

                let creep = self.active_creeps.get_mut(&entity_index).unwrap();
                creep.x = x;
                creep.y = y;
                creep.wave_id = new_wave_id;
                creep.life_state = life_state;
                creep.npc_state = npc_state;
            } else {
                // Normal update: keep tracking, update position, life_state, and npc_state in
                // place. DEAD life_state or a death npc_state means snapshot emission is
                // suppressed in build_creep_snapshot rather than evicting the creep entirely.
                let creep = self.active_creeps.get_mut(&entity_index).unwrap();
                creep.x = x;
                creep.y = y;
                creep.life_state = life_state;
                creep.npc_state = npc_state;

                if life_state == LIFE_DEAD {
                    debug!(
                        "[creep_tracker] UPDATE entity={} life_state=DEAD -- suppressing snapshots but keeping in active_creeps",
                        entity_index
                    );
                } else if matches!(npc_state, NPC_STATE_DYING_CITADEL | NPC_STATE_DEAD_CITADEL) {
                    debug!(
                        "[creep_tracker] UPDATE entity={} npc_state={} (death state) while life_state=ALIVE -- will be suppressed at snapshot",
                        entity_index, npc_state
                    );
                }
            }
        } else if lane != 0 && !matches!(npc_state, NPC_STATE_INERT | NPC_STATE_INIT | NPC_STATE_INVALID) {
            // Creep entered interest scope without a preceding CREATE -- treat as late create.
            // Only register if lane is assigned and npc_state is active; lane=0 or INERT means
            // the entity is still pre-spawn or recycling and should not be tracked yet.
            debug!(
                "[creep_tracker] UPDATE entity={} not in active_creeps -- treating as late CREATE (lane={} npc_state={})",
                entity_index, lane, npc_state
            );
            self.handle_creep_create(entity_index, lane, team, x, y, match_sec, npc_state, life_state, health);
        } else {
            debug!(
                "[creep_tracker] UPDATE entity={} not in active_creeps -- skipping (lane={} npc_state={})",
                entity_index, lane, npc_state
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
            let (lane, team, x, y, wave_id, life_state, npc_state, is_cage) = {
                let c = &self.active_creeps[entity_index];
                (c.lane, c.team, c.x, c.y, c.wave_id.clone(), c.life_state, c.npc_state, c.is_cage)
            };

            // Emit a snapshot only when the creep is confirmed active in lane (whitelist).
            // Whitelisted npc_states:
            // - NPC_STATE_IDLE (1): active in lane, no threat
            // - NPC_STATE_ALERT (2): threat detected, not yet in combat
            // - NPC_STATE_COMBAT (3): fighting in lane
            // - NPC_STATE_INERT (6): stunned/frozen mid-lane, or cage entity on zipline
            // All other states (DYING_CITADEL, DEAD_CITADEL, DEAD, INIT, INVALID, etc.) are
            // suppressed. Unknown future states are also suppressed -- safe default.
            // life_state must be LIFE_ALIVE regardless of npc_state (guards INERT+DEAD/DYING).
            let is_active = life_state == LIFE_ALIVE
                && matches!(npc_state, NPC_STATE_IDLE | NPC_STATE_ALERT | NPC_STATE_COMBAT | NPC_STATE_INERT);
            let entry = if !is_active {
                None
            } else {
                let nearby_players = Self::compute_nearby_players(x, y, player_positions);
                Some(CreepSnapshot {
                    x,
                    y,
                    lane,
                    team,
                    wave_id,
                    nearby_players,
                    is_cage,
                })
            };

            let timeline = self.creep_timelines.entry(*entity_index).or_default();
            // Fill any gap up to match_sec (handles first-snapshot case)
            while timeline.len() < match_sec as usize {
                timeline.push(None);
            }
            if timeline.len() == match_sec as usize {
                timeline.push(entry);
            } else {
                // Already has an entry for this second (shouldn't happen, but overwrite to be safe)
                timeline[match_sec as usize] = entry;
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
    ///
    /// wave_meta is filtered to only include waves that have at least one live snapshot in a
    /// creep timeline. Orphan waves (registered but evicted before any snapshot was emitted --
    /// typically a CREATE followed immediately by an INERT transition in the same tick) are
    /// excluded to prevent downstream consumers from encountering wave IDs they cannot resolve.
    pub fn get_output(&self) -> LaneCreepData {
        // Collect all wave IDs referenced by at least one non-null snapshot.
        let mut live_wave_ids: std::collections::HashSet<&str> =
            std::collections::HashSet::new();
        for timeline in self.creep_timelines.values() {
            for snap in timeline.iter().flatten() {
                live_wave_ids.insert(snap.wave_id.as_str());
            }
        }

        let orphan_count = self
            .wave_meta
            .keys()
            .filter(|id| !live_wave_ids.contains(id.as_str()))
            .count();

        let total_creeps = self.creep_timelines.len();
        let total_waves = self.wave_meta.len();
        let non_null: usize = self
            .creep_timelines
            .values()
            .flat_map(|t| t.iter())
            .filter(|s| s.is_some())
            .count();

        if orphan_count > 0 {
            info!(
                "[creep_tracker] Dropping {} orphan wave(s) with no live snapshots (registered then immediately evicted)",
                orphan_count
            );
        }

        info!(
            "[creep_tracker] Output: {} creeps, {} waves ({} after orphan removal), {} non-null snapshots",
            total_creeps,
            total_waves,
            total_waves.saturating_sub(orphan_count),
            non_null
        );

        LaneCreepData {
            creeps: self
                .creep_timelines
                .iter()
                .map(|(k, v)| (k.to_string(), v.clone()))
                .collect(),
            wave_meta: self
                .wave_meta
                .iter()
                .filter(|(id, _)| live_wave_ids.contains(id.as_str()))
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
        }
    }

    // -------------------------------------------------------------------------
    // Private helpers
    // -------------------------------------------------------------------------

    /// Assign a creep to a wave based on spawn time.
    ///
    /// If a wave already exists for `(lane, team)` and the last spawn was within the
    /// applicable grouping window, the creep joins that wave. Otherwise a new wave is
    /// created with `wave_id = "lane_team_spawnsec"`.
    ///
    /// `is_cage` selects the window: cage entities use `CAGE_GROUPING_WINDOW_S` (5 s) to
    /// group their tight launch cluster; real creeps use `REAL_CREEP_GROUPING_WINDOW_S`
    /// (18 s) so they can look back and join the cage wave that preceded them by 13-15 s.
    fn assign_wave(&mut self, lane: i32, team: u32, current_sec: u32, is_cage: bool) -> String {
        let key = (lane, team);
        let window = if is_cage { CAGE_GROUPING_WINDOW_S } else { REAL_CREEP_GROUPING_WINDOW_S };

        if let Some((last_sec, wave_id)) = self.wave_last_spawn.get(&key) {
            if current_sec.saturating_sub(*last_sec) <= window {
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
