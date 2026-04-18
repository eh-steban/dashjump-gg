//! Boss entity tracking for objectives (Guardians, Shrines, Walkers, etc.)

use std::collections::HashMap;

use anyhow::Result;
use haste::entities::{Entity, fkey_from_path};
use tracing::error;
use haste::parser::Context;

use crate::domain::BossSnapshot;
use crate::entities::constants::{
    CCITADEL_DESTROYABLE_BUILDING_ENTITY, CNPC_BARRACKBOSS_ENTITY, CNPC_BOSS_TIER2_ENTITY,
    CNPC_BOSS_TIER3_ENTITY, CNPC_TROOPERBOSS_ENTITY, TEAM_KEY,
};
use crate::utils::get_entity_position;

/// Tracks boss entities (objectives) throughout the match
#[derive(Debug)]
pub struct BossTracker {
    // Field keys (pre-computed)
    health_key: u64,
    max_health_key: u64,
    life_state_key: u64,
    lane_key: u64,

    // Active bosses by entity index -> snapshot
    bosses: HashMap<i32, BossSnapshot>,

    // Sparse damage-driven health samples: entity_index -> Vec<(time_s, health)>
    health_samples: HashMap<i32, Vec<(u32, i32)>>,

    // Per-second health timeline: Vec<HashMap<entity_index_string, current_health>>
    health_timeline: Vec<HashMap<String, i32>>,

    // Boss type hashes
    guardian_hash: u64,
    shrine_hash: u64,
    walker_hash: u64,
    base_guardian_hash: u64,
    patron_hash: u64,
}

// NOTE: Boss type <=> hash name constant mapping
impl BossTracker {
    pub fn new() -> Self {
        Self {
            health_key: fkey_from_path(&["m_iHealth"]),
            max_health_key: fkey_from_path(&["m_iMaxHealth"]),
            life_state_key: fkey_from_path(&["m_lifeState"]),
            lane_key: fkey_from_path(&["m_iLane"]),
            bosses: HashMap::new(),
            health_samples: HashMap::new(),
            health_timeline: Vec::new(),
            guardian_hash: CNPC_TROOPERBOSS_ENTITY,
            shrine_hash: CCITADEL_DESTROYABLE_BUILDING_ENTITY,
            walker_hash: CNPC_BOSS_TIER2_ENTITY,
            base_guardian_hash: CNPC_BARRACKBOSS_ENTITY,
            patron_hash: CNPC_BOSS_TIER3_ENTITY,
        }
    }

    /// Check if an entity hash corresponds to a boss type
    pub fn is_boss_entity(&self, hash: u64) -> bool {
        hash == self.guardian_hash
            || hash == self.shrine_hash
            || hash == self.walker_hash
            || hash == self.base_guardian_hash
            || hash == self.patron_hash
    }

    /// Handle boss entity creation
    pub fn handle_boss_create(
        &mut self,
        entity: &Entity,
        custom_id: u32,
        hash: u64,
        current_time_s: u32,
    ) {
        let position = get_entity_position(entity).unwrap_or_else(|| {
            error!("[boss_tracker] boss entity (index={}) has no position on CREATE, defaulting to origin", entity.index());
            [0.0, 0.0, 0.0]
        });
        let team = entity.get_value(&TEAM_KEY).unwrap_or(0);
        let lane = entity.get_value::<i32>(&self.lane_key).unwrap_or(0);
        let health = entity.get_value::<i32>(&self.health_key).unwrap_or(0);
        let max_health = entity.get_value::<i32>(&self.max_health_key).unwrap_or(0);
        let life_state = entity.get_value::<i32>(&self.life_state_key).unwrap_or(0);

        let snapshot = BossSnapshot {
            entity_index: entity.index(),
            custom_id,
            boss_name_hash: hash.to_string(),
            team,
            lane,
            x: position[0],
            y: position[1],
            z: position[2],
            spawn_time_s: current_time_s,
            max_health,
            life_state_on_create: life_state,
            death_time_s: None,
            life_state_on_delete: None,
        };

        // Record initial health sample
        self.health_samples
            .entry(entity.index())
            .or_insert_with(Vec::new)
            .push((current_time_s, health));

        self.bosses.insert(entity.index(), snapshot);
    }

    /// Handle boss entity deletion (death)
    pub fn handle_boss_delete(&mut self, entity: &Entity, current_time_s: u32) {
        let entity_index = entity.index();
        if let Some(boss) = self.bosses.get_mut(&entity_index) {
            boss.death_time_s = Some(current_time_s);
            boss.life_state_on_delete = entity.get_value::<i32>(&self.life_state_key);
        }
        // Record health=0 at death time so build_health_window carry-forward shows the
        // boss as dead for all subsequent seconds. Without this, the last non-zero damage
        // sample (e.g. health=13) persists indefinitely, making a dead objective appear alive.
        self.record_death_health(entity_index, current_time_s);
    }

    /// Insert a health=0 sample for `entity_index` at `current_time_s`.
    ///
    /// Called by `handle_boss_delete` to ensure carry-forward correctly reports
    /// the boss as dead. Also callable directly in tests.
    ///
    /// Skips untracked entities. The DELETE event in `replay_parser` fires for
    /// every entity (creeps, projectiles, map props), so without this guard
    /// `or_insert_with` would create spurious `health=0` entries for thousands of
    /// non-boss entities, polluting `health_timeline`.
    fn record_death_health(&mut self, entity_index: i32, current_time_s: u32) {
        if !self.bosses.contains_key(&entity_index) {
            return;
        }
        self.health_samples
            .entry(entity_index)
            .or_insert_with(Vec::new)
            .push((current_time_s, 0));
    }

    /// Handle a boss entity UPDATE event.
    ///
    /// Captures both `m_iMaxHealth` and `m_iHealth` on every update, so the
    /// health timeline reflects non-damage HP changes documented in the wiki:
    ///
    /// - Walker sibling-death scaling: flat +3000 max_health and full heal
    ///   (6000 -> 9000 -> 12000). See <https://deadlock.wiki/Walker>.
    /// - Shrine sibling scaling: the surviving Shrine jumps from 5000 to 10000
    ///   with a matching heal. See <https://deadlock.wiki/Shrine>.
    /// - Patron Phase 1 -> Phase 2 transition: health resets to the phase 2
    ///   starting value even though `m_iMaxHealth` may not change (both phases
    ///   start at 12000). Also covers Patron time-based scaling (+250/min in
    ///   phase 1 after 20:00, +450/min in phase 2). See
    ///   <https://deadlock.wiki/Patron>.
    /// - Out-of-combat regen (Patron 120/s, Base Guardian 65/s during backdoor
    ///   protection). Captured whenever an UPDATE fires on the entity.
    ///
    /// Without sampling current health here, the previous damage-driven sample
    /// carries forward through scaling/heal/regen events and the timeline shows
    /// a stale (too low) value until the next damage hit.
    pub fn handle_boss_update(&mut self, entity: &Entity, current_time_s: u32) {
        let entity_index = entity.index();
        let new_max_health = entity.get_value::<i32>(&self.max_health_key).unwrap_or(0);
        self.update_max_health(entity_index, new_max_health);

        let current_health = entity.get_value::<i32>(&self.health_key).unwrap_or(0);
        self.record_current_health(entity_index, current_time_s, current_health);
    }

    /// Update the stored `max_health` for a tracked boss when the live value changes.
    ///
    /// No-op for untracked entities and for non-positive `new_max_health` values
    /// (which can briefly appear during entity teardown).
    fn update_max_health(&mut self, entity_index: i32, new_max_health: i32) {
        if new_max_health <= 0 {
            return;
        }
        if let Some(boss) = self.bosses.get_mut(&entity_index) {
            if boss.max_health != new_max_health {
                boss.max_health = new_max_health;
            }
        }
    }

    /// Append a current-health sample for a tracked boss.
    ///
    /// Complements `record_boss_damage` (which samples on damage game events)
    /// by capturing non-damage health changes observed through entity UPDATEs.
    /// Untracked entities are skipped so unrelated entity updates cannot create
    /// spurious entries.
    fn record_current_health(&mut self, entity_index: i32, current_time_s: u32, health: i32) {
        if !self.bosses.contains_key(&entity_index) {
            return;
        }
        self.health_samples
            .entry(entity_index)
            .or_insert_with(Vec::new)
            .push((current_time_s, health));
    }

    /// Record boss damage event for health tracking
    pub fn record_boss_damage(
        &mut self,
        victim_entity_index: i32,
        ctx: &Context,
        current_time_s: u32,
    ) -> Result<()> {
        // Only record if this is a tracked boss
        if !self.bosses.contains_key(&victim_entity_index) {
            return Ok(());
        }

        let entities = ctx.entities().unwrap();
        if let Some(victim) = entities.get(&victim_entity_index) {
            let health = victim.get_value::<i32>(&self.health_key).unwrap_or(0);

            self.health_samples
                .entry(victim_entity_index)
                .or_insert_with(Vec::new)
                .push((current_time_s, health));
        }

        Ok(())
    }

    /// Build health snapshot for current window (carry-forward last known health)
    pub fn build_health_window(&mut self, window_s: u32) {
        let mut window_health: HashMap<String, i32> = HashMap::new();

        for (entity_index, samples) in &self.health_samples {
            // Find the most recent sample <= window_s (carry-forward last known health)
            if let Some((_, health)) = samples.iter().rev().find(|(time, _)| *time <= window_s) {
                window_health.insert(entity_index.to_string(), *health);
            }
        }

        self.health_timeline.push(window_health);
    }

    /// Get boss tracker output for JSON serialization
    pub fn get_output(&self) -> serde_json::Value {
        serde_json::json!({
            "snapshots": self.bosses.values().collect::<Vec<_>>(),
            "health_timeline": self.health_timeline,
        })
    }

    /// Get the lane field key (needed by other trackers)
    pub fn lane_key(&self) -> u64 {
        self.lane_key
    }
}

#[cfg(test)]
mod tests;
