//! Mid-boss entity tracking -- fight windows, spawn/kill lifecycle, and rejuv attribution.

use std::collections::BTreeMap;

use haste::entities::Entity;
use haste::fxhash;
use tracing::{debug, info};

use crate::domain::mid_boss::{
    FightWindow, HealthSample, MidBossData, MidBossKillEvent, MidBossPostMatch, MidBossSpawnEvent,
    RejuvStatusEvent,
};
use crate::entities::constants::MAX_HEALTH_KEY;

/// Seconds of no damage after which the current fight window is considered closed.
const FIGHT_WINDOW_GAP_S: f32 = 5.0;

/// event_type value on CCitadelUserMsg_RejuvStatus that represents a rejuvenation grant
/// (i.e. a team successfully claiming the mid-boss kill benefit). Observed value from
/// replays -- teams claiming the kill receive 2+ events of this type; teams that only
/// deal damage receive other event types.
const REJUV_GRANT_EVENT_TYPE: i32 = 6;

/// Minimum rejuv grant events for a team to be credited with claiming the mid-boss kill.
/// With 3 grants per kill, strict majority (>= 2) always identifies a unique winner.
const REJUV_GRANT_THRESHOLD: u32 = 2;

/// Seconds after the fight window's last-damage anchor to accept rejuv events into
/// a kill's attribution window. See references.md "attribution window" in contracts.
const REJUV_ATTRIBUTION_WINDOW_S: f32 = 30.0;

/// `ECitadelLobbyTeam` values for the two playing sides. Used as keys in rejuvs_by_team
/// so both teams always appear even when one has zero grants.
const LOBBY_TEAM_AMBER: i32 = 2;
const LOBBY_TEAM_SAPPHIRE: i32 = 3;

/// Tracks mid-boss entities throughout the match.
///
/// Manages fight windows (continuous damage bursts separated by FIGHT_WINDOW_GAP_S gaps),
/// spawn/kill events sourced from CCitadelUserMsg_MidBossSpawned and CCitadelUserMsg_BossKilled,
/// and rejuvenation events from CCitadelUserMsg_RejuvStatus. The finalize() method derives
/// team_claimed for each kill by counting rejuv grant events grouped by user_team.
#[derive(Debug)]
pub struct MidBossTracker {
    /// FNV/FX hash of the mid-boss entity class name, stored as a string for serialization.
    boss_name_hash: String,

    /// Entity index of the currently active mid-boss entity. Updated on CREATE/UPDATE.
    mid_boss_entity_index: Option<i32>,

    /// Monotonically increasing spawn counter -- incremented on each MidBossSpawned event.
    current_spawn_cycle: u32,

    // --- Event collections ---
    spawn_events: Vec<MidBossSpawnEvent>,
    kill_events: Vec<MidBossKillEvent>,
    rejuv_events: Vec<RejuvStatusEvent>,
    fight_windows: Vec<FightWindow>,
    post_match: Vec<MidBossPostMatch>,

    // --- Fight window state ---
    /// Currently open fight window, if any.
    open_window: Option<OpenWindow>,
}

/// Internal state for a fight window that has been opened but not yet closed.
#[derive(Debug)]
struct OpenWindow {
    spawn_cycle: u32,
    window_start_s: f32,
    health_at_start: i32,
    last_health: i32,
    last_damage_time_s: f32,
    health_samples: Vec<HealthSample>,
}

impl MidBossTracker {
    pub fn new() -> Self {
        // Use fxhash on the entity class name so the output value is stable and
        // matches the hash used in should_track_position() / on_entity routing.
        let boss_name_hash = fxhash::hash_bytes(b"CNPC_MidBoss").to_string();
        Self {
            boss_name_hash,
            mid_boss_entity_index: None,
            current_spawn_cycle: 0,
            spawn_events: Vec::new(),
            kill_events: Vec::new(),
            rejuv_events: Vec::new(),
            fight_windows: Vec::new(),
            post_match: Vec::new(),
            open_window: None,
        }
    }

    /// Returns the entity index of the currently tracked mid-boss entity, if any.
    /// Used by replay_parser.rs to route damage events.
    pub fn mid_boss_entity_index(&self) -> Option<i32> {
        self.mid_boss_entity_index
    }

    /// Called when CCitadelUserMsg_MidBossSpawned (ID 349) is received.
    /// Increments the spawn cycle and records a spawn event.
    pub fn handle_spawn(&mut self, match_time_s: f32) {
        self.current_spawn_cycle += 1;
        info!(
            "[mid_boss_tracker] Spawn event: spawn_cycle={} match_time_s={:.1}",
            self.current_spawn_cycle, match_time_s
        );
        // max_health is a placeholder; observe_entity fills it on the paired CREATE.
        self.spawn_events.push(MidBossSpawnEvent {
            spawn_cycle: self.current_spawn_cycle,
            spawn_time_s: match_time_s,
            max_health: 0,
        });
    }

    /// Called when CCitadelUserMsg_BossKilled (ID 347) is received and
    /// entity_killed_class == MID_BOSS_CLASS_ID.
    ///
    /// Closes any open fight window, anchoring window_end_s on last-damage time (not
    /// kill time -- BossKilled.gametime lags actual death by 7-18s on observed replays).
    /// Kill-event team fields are placeholders; finalize() populates them from rejuv grants.
    pub fn handle_kill(
        &mut self,
        matchtime_s: f32,
        x: f32,
        y: f32,
        z: f32,
        bosses_remaining: i32,
    ) {
        info!(
            "[mid_boss_tracker] Kill event: spawn_cycle={} matchtime_s={:.1} bosses_remaining={}",
            self.current_spawn_cycle, matchtime_s, bosses_remaining
        );
        // Close the open fight window with health_at_end=0, anchored on last-damage
        // time so the rejuv attribution window in finalize() sees grants that fire
        // several seconds before BossKilled's lagging gametime.
        if let Some(window) = self.open_window.take() {
            debug!(
                "[mid_boss_tracker] Closing fight window at kill: spawn_cycle={} window_start_s={:.1} last_damage_s={:.1} kill_s={:.1}",
                window.spawn_cycle, window.window_start_s, window.last_damage_time_s, matchtime_s
            );
            self.fight_windows.push(FightWindow {
                spawn_cycle: window.spawn_cycle,
                window_start_s: window.window_start_s,
                window_end_s: window.last_damage_time_s,
                health_at_start: window.health_at_start,
                health_at_end: 0,
                health_samples: window.health_samples,
            });
        }
        self.kill_events.push(MidBossKillEvent {
            spawn_cycle: self.current_spawn_cycle,
            team_killed: 0,
            team_claimed: 0,
            rejuvs_by_team: empty_rejuvs_by_team(),
            matchtime_s,
            x,
            y,
            z,
            bosses_remaining,
        });
    }

    /// Called when CCitadelUserMsg_RejuvStatus (ID 350) is received.
    pub fn handle_rejuv_status(
        &mut self,
        matchtime_s: f32,
        player_pawn: u32,
        user_team: i32,
        killing_team: i32,
        event_type: i32,
    ) {
        debug!(
            "[mid_boss_tracker] RejuvStatus: matchtime_s={:.1} user_team={} killing_team={} event_type={}",
            matchtime_s, user_team, killing_team, event_type
        );
        self.rejuv_events.push(RejuvStatusEvent {
            matchtime_s,
            player_pawn,
            user_team,
            killing_team,
            event_type,
        });
    }

    /// Called from on_entity when the mid-boss entity is CREATEd or UPDATEd.
    ///
    /// On CREATE: reads m_iMaxHealth and writes it to the current spawn event
    /// (matches per-cycle scaling; `m_iMaxHealth = 13000 + 195 * match_minutes`).
    /// On UPDATE: refreshes the entity index (keeps it current if entity is recycled).
    pub fn observe_entity(&mut self, entity_index: i32, entity: &Entity, is_create: bool) {
        self.mid_boss_entity_index = Some(entity_index);
        if is_create {
            if let Some(max_hp) = entity.get_value::<i32>(&MAX_HEALTH_KEY) {
                debug!(
                    "[mid_boss_tracker] Mid-boss CREATE: entity_index={} max_health={}",
                    entity_index, max_hp
                );
                // CREATE fires after the paired MidBossSpawned message, so the
                // latest spawn_event is the one this entity belongs to.
                if let Some(spawn) = self.spawn_events.last_mut() {
                    spawn.max_health = max_hp;
                }
            }
        }
    }

    /// Called when a damage event's victim is the currently tracked mid-boss entity.
    ///
    /// Manages fight window lifecycle:
    /// - First damage: opens a new window.
    /// - Subsequent damage within FIGHT_WINDOW_GAP_S: appends a sample.
    /// - Damage after gap > FIGHT_WINDOW_GAP_S: closes the old window, opens a new one.
    pub fn record_damage(&mut self, health: i32, match_time_s: f32) {
        match &mut self.open_window {
            None => {
                // Open a new fight window.
                debug!(
                    "[mid_boss_tracker] Opening fight window: spawn_cycle={} start_s={:.1} health_at_start={}",
                    self.current_spawn_cycle, match_time_s, health
                );
                self.open_window = Some(OpenWindow {
                    spawn_cycle: self.current_spawn_cycle,
                    window_start_s: match_time_s,
                    health_at_start: health,
                    last_health: health,
                    last_damage_time_s: match_time_s,
                    health_samples: vec![HealthSample {
                        time_s: match_time_s,
                        health,
                    }],
                });
            }
            Some(window) => {
                let gap = match_time_s - window.last_damage_time_s;
                if gap > FIGHT_WINDOW_GAP_S {
                    // Gap exceeded -- close old window and open a new one.
                    debug!(
                        "[mid_boss_tracker] Gap {:.1}s > {:.1}s -- closing window spawn_cycle={} end_s={:.1}",
                        gap, FIGHT_WINDOW_GAP_S, window.spawn_cycle, window.last_damage_time_s
                    );
                    // SAFETY: we are inside the Some(window) arm, so take() cannot return None.
                    let closed = self.open_window.take().expect("invariant: Some arm guarantees take returns Some");
                    let health_at_end = closed.last_health;
                    let window_end_s = closed.last_damage_time_s;
                    self.fight_windows.push(FightWindow {
                        spawn_cycle: closed.spawn_cycle,
                        window_start_s: closed.window_start_s,
                        window_end_s,
                        health_at_start: closed.health_at_start,
                        health_at_end,
                        health_samples: closed.health_samples,
                    });
                    // Open a new window starting now.
                    self.open_window = Some(OpenWindow {
                        spawn_cycle: self.current_spawn_cycle,
                        window_start_s: match_time_s,
                        health_at_start: health,
                        last_health: health,
                        last_damage_time_s: match_time_s,
                        health_samples: vec![HealthSample {
                            time_s: match_time_s,
                            health,
                        }],
                    });
                } else {
                    // Within the same window -- append a sample and update state.
                    window.health_samples.push(HealthSample {
                        time_s: match_time_s,
                        health,
                    });
                    window.last_health = health;
                    window.last_damage_time_s = match_time_s;
                }
            }
        }
    }

    /// Called after parsing is complete. Closes any open fight window, then sources
    /// team_killed, team_claimed, and rejuvs_by_team for each kill from RejuvStatus
    /// grants observed within the kill's attribution window. Populates post_match.
    ///
    /// Attribution window is `[anchor, anchor + REJUV_ATTRIBUTION_WINDOW_S]`, where
    /// anchor is the last-damage time (window_end_s) of the kill's matching fight
    /// window. See references.md in the contracts spec for why we don't anchor on
    /// BossKilled.gametime.
    pub fn finalize(&mut self) {
        // Close any still-open fight window (match ends without a kill event).
        if let Some(window) = self.open_window.take() {
            debug!(
                "[mid_boss_tracker] finalize: closing open fight window spawn_cycle={} end_s={:.1}",
                window.spawn_cycle, window.last_damage_time_s
            );
            self.fight_windows.push(FightWindow {
                spawn_cycle: window.spawn_cycle,
                window_start_s: window.window_start_s,
                window_end_s: window.last_damage_time_s,
                health_at_start: window.health_at_start,
                health_at_end: window.last_health,
                health_samples: window.health_samples,
            });
        }

        // For each kill, anchor attribution on the last fight window matching its
        // spawn_cycle (window_end_s = last damage time). Fall back to the kill's own
        // matchtime_s if the cycle has no fight window -- defensive for synthetic
        // tests that call handle_kill without prior record_damage.
        let anchors: Vec<f32> = self
            .kill_events
            .iter()
            .map(|kill| self.attribution_anchor_for(kill.spawn_cycle, kill.matchtime_s))
            .collect();

        for (kill, anchor) in self.kill_events.iter_mut().zip(anchors.iter()) {
            let window_end = anchor + REJUV_ATTRIBUTION_WINDOW_S;

            let mut rejuvs_by_team = empty_rejuvs_by_team();
            let mut killing_team_seen: Option<i32> = None;
            for rejuv in &self.rejuv_events {
                if rejuv.matchtime_s < *anchor || rejuv.matchtime_s > window_end {
                    continue;
                }
                if killing_team_seen.is_none() {
                    killing_team_seen = Some(rejuv.killing_team);
                }
                if rejuv.event_type == REJUV_GRANT_EVENT_TYPE {
                    let key = rejuv.user_team.to_string();
                    *rejuvs_by_team.entry(key).or_insert(0) += 1;
                }
            }

            kill.team_killed = killing_team_seen.unwrap_or(0);
            kill.team_claimed = derive_team_claimed(&rejuvs_by_team, kill.team_killed);
            kill.rejuvs_by_team = rejuvs_by_team;

            if kill.team_killed != 0 && kill.team_claimed != kill.team_killed {
                info!(
                    "[mid_boss_tracker] Kill steal: spawn_cycle={} team_killed={} team_claimed={} rejuvs_by_team={:?}",
                    kill.spawn_cycle, kill.team_killed, kill.team_claimed, kill.rejuvs_by_team
                );
            }
        }

        // Populate post_match summaries from kill events.
        for kill in &self.kill_events {
            self.post_match.push(MidBossPostMatch {
                team_killed: kill.team_killed,
                rejuvs_by_team: kill.rejuvs_by_team.clone(),
                destroyed_time_s: kill.matchtime_s as u32,
            });
        }

        info!(
            "[mid_boss_tracker] finalize complete: spawns={} kills={} fight_windows={} rejuv_events={}",
            self.spawn_events.len(),
            self.kill_events.len(),
            self.fight_windows.len(),
            self.rejuv_events.len(),
        );
    }

    /// Returns the attribution anchor time for a kill in the given spawn cycle.
    /// Uses the last fight window matching the cycle (its `window_end_s` is last-
    /// damage time). Falls back to `kill_matchtime_s` when no fight window is
    /// recorded for the cycle -- this path is exercised only by unit tests that
    /// skip the damage pipeline.
    fn attribution_anchor_for(&self, spawn_cycle: u32, kill_matchtime_s: f32) -> f32 {
        self.fight_windows
            .iter()
            .rev()
            .find(|w| w.spawn_cycle == spawn_cycle)
            .map(|w| w.window_end_s)
            .unwrap_or(kill_matchtime_s)
    }

    /// Returns all collected mid-boss data. Must be called after finalize().
    pub fn get_output(&self) -> MidBossData {
        MidBossData {
            boss_name_hash: self.boss_name_hash.clone(),
            spawn_events: self.spawn_events.clone(),
            kill_events: self.kill_events.clone(),
            rejuv_events: self.rejuv_events.clone(),
            fight_windows: self.fight_windows.clone(),
            post_match: self.post_match.clone(),
        }
    }
}

/// Returns a fresh `rejuvs_by_team` map with both team keys present and zeroed.
/// Downstream consumers can then unconditionally read `["2"]` / `["3"]` without
/// a missing-key branch.
fn empty_rejuvs_by_team() -> BTreeMap<String, u32> {
    let mut m = BTreeMap::new();
    m.insert(LOBBY_TEAM_AMBER.to_string(), 0);
    m.insert(LOBBY_TEAM_SAPPHIRE.to_string(), 0);
    m
}

/// Picks the team with a strict majority (`>= REJUV_GRANT_THRESHOLD`) of rejuv
/// grants. With 3 grants per kill, exactly one team always reaches the threshold,
/// so this returns the unique winner for real matches. Falls back to
/// `fallback_killing_team` for synthetic inputs that lack enough grants (unit
/// tests, empty attribution windows).
fn derive_team_claimed(rejuvs_by_team: &BTreeMap<String, u32>, fallback_killing_team: i32) -> i32 {
    rejuvs_by_team
        .iter()
        .filter(|(_, count)| **count >= REJUV_GRANT_THRESHOLD)
        .max_by_key(|(_, count)| **count)
        .and_then(|(team, _)| team.parse::<i32>().ok())
        .unwrap_or(fallback_killing_team)
}

#[cfg(test)]
mod tests;
