use serde::Serialize;

/// Snapshot of a boss entity's state
#[derive(Debug, Serialize, Clone)]
pub struct BossSnapshot {
    /// Unique per entity instance
    pub entity_index: i32,
    /// Entity type ID (21, 25, 26, 27, 28)
    pub custom_id: u32,
    /// serializer_name fxhash as a decimal string.
    ///
    /// Serialized as a JSON string (not a number) to preserve the full u64 range
    /// across the JSON boundary to JavaScript, which cannot represent integers
    /// above 2^53 without precision loss. Mirrors `MidBossData.boss_name_hash`.
    pub boss_name_hash: String,
    pub team: u32,
    pub lane: i32,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub spawn_time_s: u32,
    pub max_health: i32,
    pub life_state_on_create: i32,
    pub death_time_s: Option<u32>,
    pub life_state_on_delete: Option<i32>,
}

#[cfg(test)]
mod tests;
