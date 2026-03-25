use serde::Serialize;

/// Single damage event record
#[derive(Default, Debug, Serialize)]
pub struct DamageRecord {
    pub damage: i32,
    pub pre_damage: f32,
    #[serde(rename = "type")]
    pub damage_type: i32,
    pub citadel_type: i32,
    pub entindex_inflictor: i32,
    pub entindex_ability: i32,
    pub damage_absorbed: f32,
    pub victim_health_max: i32,
    pub victim_health_new: i32,
    pub flags: u64,
    pub ability_id: u32,
    pub attacker_class: u32,
    pub victim_class: u32,
    pub victim_shield_max: i32,
    pub victim_shield_new: i32,
    pub hits: i32,
    pub health_lost: i32,
}
