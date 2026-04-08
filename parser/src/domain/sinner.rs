use serde::Serialize;

/// Snapshot of one Sinner Sacrifice spawn event (initial spawn or respawn).
/// Entity indices are recycled on respawn, so `entity_index` is not unique
/// across snapshots in a single match.
#[derive(Debug, Serialize, Clone)]
pub struct SinnerSnapshot {
    pub entity_index: i32,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub spawn_time_s: u32,
    pub max_health: i32,
    pub death_time_s: Option<u32>,
    pub time_alive_s: Option<u32>,
    pub killer_player_slot: Option<u32>,
    /// Retaliation damage this sinner dealt to each player, keyed by lobby player slot.
    /// HashMap<u32, _> serializes keys as strings in JSON: {"0": 160, "4": 80}.
    /// Empty map if no player hit this sinner during its lifetime.
    pub retaliation_damage: std::collections::HashMap<u32, i32>,
}
