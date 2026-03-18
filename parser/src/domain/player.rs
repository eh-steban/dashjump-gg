use serde::Serialize;

/// Player metadata extracted from replay
#[derive(Default, Debug, Serialize)]
pub struct Player {
    pub entity_id: String,
    pub custom_id: String,
    pub name: String,
    pub steam_id_32: u32,
    pub hero_id: u32,
    pub lobby_player_slot: u32,
    pub team: u32,
    pub lane: i8,
}

/// Per-tick player/entity position
#[derive(Default, Debug, Serialize)]
pub struct PlayerPosition {
    pub custom_id: String,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub is_npc: bool,
}
