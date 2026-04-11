use serde::Serialize;

/// Direction of a damage event against a sinner during its life.
#[derive(Debug, Serialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SinnerDamageKind {
    /// Damage a player dealt TO the sinner (e.g. light melee punches).
    Dealt,
    /// Damage the sinner dealt BACK TO a player (cursed return / retaliation).
    Retaliated,
}

/// A single damage exchange between one player and this sinner, at one point in time.
///
/// Multiple events at the same `time_s` are possible when more than one player is attacking
/// a sinner simultaneously. Events are appended in the order the parser observes them on the
/// damage-event stream; no time-window bucketing is performed at the parser layer.
#[derive(Debug, Serialize, Clone)]
pub struct SinnerDamageEvent {
    /// Match second at which the event was observed.
    pub time_s: u32,
    /// The player slot involved in this event. For `Dealt`, this is the attacker's slot;
    /// for `Retaliated`, this is the victim's slot.
    pub player_slot: u32,
    /// Direction of the event.
    pub kind: SinnerDamageKind,
    /// Raw damage amount emitted on the underlying Valve damage event.
    pub damage: i32,
}

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
    /// DEPRECATED: retained for backwards compatibility during the frontend transition.
    /// Prefer `damage_events` filtered by `kind == Retaliated`, grouped by `player_slot`,
    /// summing `damage`. This field will be removed once `PlayerCards.tsx` migrates.
    ///
    /// Retaliation damage this sinner dealt to each player, keyed by lobby player slot.
    /// HashMap<u32, _> serializes keys as strings in JSON: {"0": 160, "4": 80}.
    /// Empty map if no player hit this sinner during its lifetime.
    pub retaliation_damage: std::collections::HashMap<u32, i32>,
    /// Timestamped log of every damage exchange involving this sinner during its life.
    /// Contains both damage dealt TO the sinner (`kind == Dealt`) and retaliation the sinner
    /// dealt back to players (`kind == Retaliated`). Events are in observation order.
    /// Use this field for time-window bucketing and dealt/retaliated trade metrics.
    pub damage_events: Vec<SinnerDamageEvent>,
}
