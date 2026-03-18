//! Lane creep tracking domain models
//!
//! Each individual creep is tracked from spawn to death. Timelines are match-relative:
//! index 0 corresponds to match second 0, matching the player positions array.

use std::collections::HashMap;

use serde::Serialize;

/// Per-second snapshot for one individual creep while it is alive.
#[derive(Debug, Clone, Serialize)]
pub struct CreepSnapshot {
    /// World X/Y position
    pub x: f32,
    pub y: f32,
    /// Lane assignment (1 = yellow, 4 = blue, 6 green in Deadlock)
    pub lane: i32,
    /// Team (2 = Amber, 3 = Sapphire)
    pub team: u32,
    /// Wave identifier: "lane_team_spawnsec" e.g. "1_2_45"
    pub wave_id: String,
    /// Player custom_ids (lobby_player_slot) within 1500 world units of this creep
    pub nearby_players: Vec<i32>,
    /// True when this creep is still in its cage on the zipline (pre-lane-drop).
    /// Cage entities have m_iHealth == 1 and are visual carriers -- not fighting units.
    /// They appear on the minimap as "wave inbound" indicators.
    pub is_cage: bool,
}

/// Match-relative sparse timeline for one individual creep.
///
/// Index i corresponds to match second i (0 = match start). `None` means the creep was
/// not alive at that second (not yet spawned, or already dead).
pub type CreepTimeline = Vec<Option<CreepSnapshot>>;

/// Metadata computed across a wave's full lifetime.
#[derive(Debug, Clone, Serialize)]
pub struct WaveMeta {
    pub lane: i32,
    /// Team (2 = Amber, 3 = Sapphire)
    pub team: u32,
    /// Match-relative second the wave was first seen spawning
    pub spawn_sec: u32,
    /// Match-relative second the last creep in this wave died (None if any are still alive)
    pub last_death_sec: Option<u32>,
    /// World X/Y position of the last creep death
    pub last_death_x: Option<f32>,
    pub last_death_y: Option<f32>,
}

/// Container for all per-creep lane data.
#[derive(Debug, Clone, Serialize, Default)]
pub struct LaneCreepData {
    /// Per-creep timelines. Key: entity_index as string (JSON-compatible).
    /// Value: match-relative timeline where index i = match second i.
    pub creeps: HashMap<String, CreepTimeline>,
    /// Wave-level metadata. Key: wave_id "lane_team_spawnsec".
    pub wave_meta: HashMap<String, WaveMeta>,
}
