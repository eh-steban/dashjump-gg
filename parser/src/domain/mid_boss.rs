//! Domain types for mid-boss tracking

use serde::Serialize;

/// All mid-boss data collected during a match.
#[derive(Debug, Serialize, Default, Clone)]
pub struct MidBossData {
    pub boss_name_hash: String,
    pub max_health: Option<i32>,
    pub spawn_events: Vec<MidBossSpawnEvent>,
    pub kill_events: Vec<MidBossKillEvent>,
    pub rejuv_events: Vec<RejuvStatusEvent>,
    pub fight_windows: Vec<FightWindow>,
    pub post_match: Vec<MidBossPostMatch>,
}

/// Fired when the mid-boss spawns (CCitadelUserMsg_MidBossSpawned, ID 349).
#[derive(Debug, Serialize, Clone)]
pub struct MidBossSpawnEvent {
    pub spawn_cycle: u32,
    pub spawn_time_s: f32,
}

/// Fired when the mid-boss is killed (CCitadelUserMsg_BossKilled, ID 347,
/// filtered to entity_killed_class == MID_BOSS_CLASS_ID).
#[derive(Debug, Serialize, Clone)]
pub struct MidBossKillEvent {
    pub spawn_cycle: u32,
    /// Team that dealt the killing blow (objective_team from proto).
    pub team: i32,
    /// Team that claims the kill benefit. Derived by finalize() from rejuv grants;
    /// defaults to `team` when no unambiguous grant is observed.
    pub team_claimed: i32,
    pub matchtime_s: f32,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub bosses_remaining: i32,
}

/// Fired on each rejuvenation status event (CCitadelUserMsg_RejuvStatus, ID 350).
#[derive(Debug, Serialize, Clone)]
pub struct RejuvStatusEvent {
    pub matchtime_s: f32,
    pub player_pawn: u32,
    pub user_team: i32,
    pub killing_team: i32,
    pub event_type: i32,
}

/// A window of continuous damage to the mid-boss (gap > FIGHT_WINDOW_GAP_S closes a window).
#[derive(Debug, Serialize, Clone)]
pub struct FightWindow {
    pub spawn_cycle: u32,
    pub window_start_s: f32,
    pub window_end_s: f32,
    pub health_at_start: i32,
    pub health_at_end: i32,
    pub health_samples: Vec<HealthSample>,
}

/// A single health observation within a fight window.
#[derive(Debug, Serialize, Clone)]
pub struct HealthSample {
    pub time_s: f32,
    pub health: i32,
}

/// Post-match summary for a single mid-boss kill cycle (populated by finalize()).
#[derive(Debug, Serialize, Default, Clone)]
pub struct MidBossPostMatch {
    pub team_killed: i32,
    pub team_claimed: i32,
    pub destroyed_time_s: u32,
}
