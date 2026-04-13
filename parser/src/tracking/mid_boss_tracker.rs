//! Mid-boss entity tracking -- fight windows, spawn/kill lifecycle, and team_claimed derivation.

use std::collections::HashMap;

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
const REJUV_GRANT_THRESHOLD: u32 = 2;

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
    /// Max health read from the entity on CREATE (m_iMaxHealth).
    max_health: Option<i32>,

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
            max_health: None,
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
        self.spawn_events.push(MidBossSpawnEvent {
            spawn_cycle: self.current_spawn_cycle,
            spawn_time_s: match_time_s,
        });
    }

    /// Called when CCitadelUserMsg_BossKilled (ID 347) is received and
    /// entity_killed_class == MID_BOSS_CLASS_ID.
    ///
    /// Closes any open fight window with health_at_end=0, then records the kill event.
    /// team_claimed defaults to `team` here; finalize() corrects it from rejuv grants.
    pub fn handle_kill(
        &mut self,
        team: i32,
        matchtime_s: f32,
        x: f32,
        y: f32,
        z: f32,
        bosses_remaining: i32,
    ) {
        info!(
            "[mid_boss_tracker] Kill event: spawn_cycle={} team={} matchtime_s={:.1} bosses_remaining={}",
            self.current_spawn_cycle, team, matchtime_s, bosses_remaining
        );
        // Close the open fight window at kill time with health_at_end=0.
        if let Some(window) = self.open_window.take() {
            debug!(
                "[mid_boss_tracker] Closing fight window at kill: spawn_cycle={} window_start_s={:.1} window_end_s={:.1}",
                window.spawn_cycle, window.window_start_s, matchtime_s
            );
            self.fight_windows.push(FightWindow {
                spawn_cycle: window.spawn_cycle,
                window_start_s: window.window_start_s,
                window_end_s: matchtime_s,
                health_at_start: window.health_at_start,
                health_at_end: 0,
                health_samples: window.health_samples,
            });
        }
        self.kill_events.push(MidBossKillEvent {
            spawn_cycle: self.current_spawn_cycle,
            team,
            team_claimed: team, // corrected by finalize()
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
    /// On CREATE: reads m_iMaxHealth and stores the entity index.
    /// On UPDATE: stores the entity index (keeps it current if entity is recycled).
    pub fn observe_entity(&mut self, entity_index: i32, entity: &Entity, is_create: bool) {
        self.mid_boss_entity_index = Some(entity_index);
        if is_create {
            if let Some(max_hp) = entity.get_value::<i32>(&MAX_HEALTH_KEY) {
                debug!(
                    "[mid_boss_tracker] Mid-boss CREATE: entity_index={} max_health={}",
                    entity_index, max_hp
                );
                self.max_health = Some(max_hp);
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

    /// Called after parsing is complete. Closes any open fight window, then derives
    /// team_claimed for each kill event by counting rejuv grant events (event_type == 6)
    /// grouped by user_team within the kill's spawn cycle. The team with >= 2 grants wins.
    /// Fallback to the killing team if no team meets the threshold.
    ///
    /// Also populates post_match summaries.
    pub fn finalize(&mut self) {
        // Close any open fight window.
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

        // Build per-kill attribution windows.
        // Rejuv grant events fire AFTER the kill (boss dies, then grants flow to players).
        // Window: [kill_time_s, next_kill_time_s) -- or unbounded for the last kill.
        // A 30-second lookahead cap prevents attributing distant events to the wrong kill.
        const REJUV_ATTRIBUTION_WINDOW_S: f32 = 30.0;

        let kill_times: Vec<(u32, f32)> = self
            .kill_events
            .iter()
            .map(|k| (k.spawn_cycle, k.matchtime_s))
            .collect();

        // Derive team_claimed for each kill.
        for kill in &mut self.kill_events {
            let kill_time = kill.matchtime_s;

            // Upper bound: the earlier of (kill_time + cap) or the next kill's time.
            let next_kill_time = kill_times
                .iter()
                .filter(|(_, t)| *t > kill_time)
                .map(|(_, t)| *t)
                .reduce(f32::min);
            let window_end = match next_kill_time {
                Some(t) => t.min(kill_time + REJUV_ATTRIBUTION_WINDOW_S),
                None => kill_time + REJUV_ATTRIBUTION_WINDOW_S,
            };

            // Count event_type==6 rejuv grants per user_team within [kill_time, window_end].
            let mut grants_by_team: HashMap<i32, u32> =
                HashMap::new();
            for rejuv in &self.rejuv_events {
                if rejuv.event_type == REJUV_GRANT_EVENT_TYPE
                    && rejuv.matchtime_s >= kill_time
                    && rejuv.matchtime_s <= window_end
                {
                    *grants_by_team.entry(rejuv.user_team).or_insert(0) += 1;
                }
            }

            // Team with >= REJUV_GRANT_THRESHOLD grants wins claimed credit.
            let claimed = grants_by_team
                .into_iter()
                .filter(|(_, count)| *count >= REJUV_GRANT_THRESHOLD)
                .max_by_key(|(_, count)| *count)
                .map(|(team, _)| team);

            if let Some(claiming_team) = claimed {
                if claiming_team != kill.team {
                    info!(
                        "[mid_boss_tracker] Kill steal: spawn_cycle={} team_killed={} team_claimed={}",
                        kill.spawn_cycle, kill.team, claiming_team
                    );
                }
                kill.team_claimed = claiming_team;
            } else {
                // No clear grant winner -- fallback to killing team.
                kill.team_claimed = kill.team;
            }
        }

        // Populate post_match summaries from kill events.
        for kill in &self.kill_events {
            self.post_match.push(MidBossPostMatch {
                team_killed: kill.team,
                team_claimed: kill.team_claimed,
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

    /// Returns all collected mid-boss data. Must be called after finalize().
    pub fn get_output(&self) -> MidBossData {
        MidBossData {
            boss_name_hash: self.boss_name_hash.clone(),
            max_health: self.max_health,
            spawn_events: self.spawn_events.clone(),
            kill_events: self.kill_events.clone(),
            rejuv_events: self.rejuv_events.clone(),
            fight_windows: self.fight_windows.clone(),
            post_match: self.post_match.clone(),
        }
    }
}

#[cfg(test)]
mod tests;
