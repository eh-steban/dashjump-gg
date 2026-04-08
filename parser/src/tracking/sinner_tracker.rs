//! Sinner Sacrifice entity tracking

use std::collections::HashMap;

use crate::domain::SinnerSnapshot;
use crate::entities::constants::MAX_HEALTH_KEY;

/// Tracks Sinner Sacrifice entities throughout the match.
///
/// Entity indices are recycled on sinner respawn. Each spawn (initial or
/// respawn) produces a separate `SinnerSnapshot`. The `last_health` and
/// `last_attacker` maps are keyed by entity index and are cleared on CREATE
/// so that stale state from a prior life does not carry over.
#[derive(Debug)]
pub struct SinnerTracker {
    snapshots: Vec<SinnerSnapshot>,
    /// Last observed health per entity index, used to detect the death signal
    /// (health transitions from >1 to 1).
    last_health: HashMap<i32, i32>,
    /// Last attacker entity index per sinner entity index, updated on every
    /// damage event. Snapshot at death time gives the killing attacker.
    last_attacker: HashMap<i32, i32>,
    /// Pre-computed field key for m_iMaxHealth.
    max_health_key: u64,
}

impl SinnerTracker {
    pub fn new() -> Self {
        Self {
            snapshots: Vec::new(),
            last_health: HashMap::new(),
            last_attacker: HashMap::new(),
            max_health_key: MAX_HEALTH_KEY,
        }
    }

    /// Called when a sinner entity is created (initial spawn or respawn after
    /// entity index recycling). Pushes a new snapshot and resets all per-entity
    /// state so the previous life's data does not carry over.
    pub fn handle_sinner_create(
        &mut self,
        entity_index: i32,
        x: f32,
        y: f32,
        z: f32,
        max_health: i32,
        spawn_time_s: u32,
    ) {
        // Clear stale state from any prior life at this entity index.
        self.last_attacker.remove(&entity_index);
        self.last_health.insert(entity_index, max_health);

        self.snapshots.push(SinnerSnapshot {
            entity_index,
            x,
            y,
            z,
            spawn_time_s,
            max_health,
            death_time_s: None,
            time_alive_s: None,
            killer_player_slot: None,
            retaliation_damage: HashMap::new(),
        });
    }

    /// Called on every damage event where the victim is a sinner. Records the
    /// attacker for later kill attribution. No-op if the sinner entity index is
    /// not currently tracked.
    pub fn record_damage(&mut self, victim_entity_index: i32, attacker_entity_index: i32) {
        if self.last_health.contains_key(&victim_entity_index) {
            self.last_attacker
                .insert(victim_entity_index, attacker_entity_index);
        }
    }

    /// Called on every damage event where the attacker is a sinner. Accumulates
    /// the raw outgoing damage into the current snapshot's retaliation map, keyed
    /// by the victim player's lobby slot. No-op if no snapshot exists for this
    /// sinner entity index.
    pub fn record_retaliation(&mut self, sinner_entity_index: i32, player_slot: u32, damage: i32) {
        // Find the last snapshot for this entity index (most recently pushed).
        if let Some(snapshot) = self
            .snapshots
            .iter_mut()
            .rev()
            .find(|s| s.entity_index == sinner_entity_index)
        {
            *snapshot
                .retaliation_damage
                .entry(player_slot)
                .or_insert(0) += damage;
        }
    }

    /// Called on every health update for a sinner entity. Updates `last_health`
    /// and detects the death signal (`health == 1 && prev_health > 1`). On death,
    /// fills `death_time_s` and `time_alive_s` on the latest snapshot and returns
    /// the last known attacker entity index. Returns `None` if this is not a death
    /// transition.
    ///
    /// Note: Deadlock sinners signal death via health==1, not health==0.
    /// Delta compression skips the zero-health packet when the entity slot is
    /// recycled, so health==0 is unreliable as a death signal.
    pub fn handle_sinner_update(
        &mut self,
        entity_index: i32,
        health: i32,
        current_time_s: u32,
    ) -> Option<i32> {
        let prev = *self.last_health.get(&entity_index)?;
        self.last_health.insert(entity_index, health);

        if health == 1 && prev > 1 {
            // Death detected -- find and update the latest snapshot for this entity.
            if let Some(snapshot) = self
                .snapshots
                .iter_mut()
                .rev()
                .find(|s| s.entity_index == entity_index)
            {
                snapshot.death_time_s = Some(current_time_s);
                snapshot.time_alive_s =
                    Some(current_time_s.saturating_sub(snapshot.spawn_time_s));
            }
            return self.last_attacker.get(&entity_index).copied();
        }

        None
    }

    /// Called after kill attribution is resolved. Sets `killer_player_slot` on
    /// the most recent dead snapshot for this entity index that does not yet have
    /// a killer assigned.
    pub fn record_sinner_death_killer(&mut self, entity_index: i32, killer_player_slot: u32) {
        if let Some(snapshot) = self
            .snapshots
            .iter_mut()
            .rev()
            .find(|s| s.entity_index == entity_index && s.death_time_s.is_some() && s.killer_player_slot.is_none())
        {
            snapshot.killer_player_slot = Some(killer_player_slot);
        }
    }

    /// Returns `true` if the given entity index is currently tracked as an active
    /// sinner (i.e., it has been created and not yet recycled out of `last_health`).
    /// Used in the damage handler to decide whether to record attacker/retaliation.
    pub fn is_tracked_sinner(&self, entity_index: i32) -> bool {
        self.last_health.contains_key(&entity_index)
    }

    /// Returns all sinner snapshots collected during the match (all lives).
    pub fn get_output(&self) -> &Vec<SinnerSnapshot> {
        &self.snapshots
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ST-1: handle_sinner_create pushes a snapshot with correct fields
    #[test]
    fn test_create_pushes_snapshot_with_correct_fields() {
        let mut tracker = SinnerTracker::new();
        tracker.handle_sinner_create(100, 1.0, 2.0, 3.0, 500, 60);

        assert_eq!(tracker.snapshots.len(), 1);
        let s = &tracker.snapshots[0];
        assert_eq!(s.entity_index, 100);
        assert_eq!(s.x, 1.0);
        assert_eq!(s.y, 2.0);
        assert_eq!(s.z, 3.0);
        assert_eq!(s.max_health, 500);
        assert_eq!(s.spawn_time_s, 60);
        assert!(s.death_time_s.is_none());
        assert!(s.time_alive_s.is_none());
        assert!(s.killer_player_slot.is_none());
        assert!(s.retaliation_damage.is_empty());
    }

    // ST-2: handle_sinner_update returns None for non-death health changes
    #[test]
    fn test_update_returns_none_for_non_death_health_change() {
        let mut tracker = SinnerTracker::new();
        tracker.handle_sinner_create(100, 0.0, 0.0, 0.0, 500, 10);

        // Health drops from 500 to 300 -- not a death signal
        let result = tracker.handle_sinner_update(100, 300, 15);
        assert!(result.is_none());
    }

    // ST-3: handle_sinner_update returns Some(attacker) on health==1 transition
    #[test]
    fn test_update_returns_attacker_on_death_signal() {
        let mut tracker = SinnerTracker::new();
        tracker.handle_sinner_create(100, 0.0, 0.0, 0.0, 500, 10);

        // Record attacker
        tracker.record_damage(100, 42);

        // Health drops to 1 from > 1 -- death signal
        let result = tracker.handle_sinner_update(100, 1, 70);
        assert_eq!(result, Some(42));
    }

    // ST-4: handle_sinner_update sets death_time_s and time_alive_s correctly
    #[test]
    fn test_update_sets_death_time_and_alive_duration() {
        let mut tracker = SinnerTracker::new();
        tracker.handle_sinner_create(100, 0.0, 0.0, 0.0, 500, 60);

        tracker.handle_sinner_update(100, 1, 120);

        let s = &tracker.snapshots[0];
        assert_eq!(s.death_time_s, Some(120));
        assert_eq!(s.time_alive_s, Some(60)); // 120 - 60
    }

    // ST-5: record_sinner_death_killer sets killer_player_slot on the right snapshot
    #[test]
    fn test_record_death_killer_sets_slot() {
        let mut tracker = SinnerTracker::new();
        tracker.handle_sinner_create(100, 0.0, 0.0, 0.0, 500, 60);
        tracker.handle_sinner_update(100, 1, 120);

        tracker.record_sinner_death_killer(100, 3);

        assert_eq!(tracker.snapshots[0].killer_player_slot, Some(3));
    }

    // ST-6: handle_sinner_create on a recycled index clears stale attacker state
    #[test]
    fn test_create_on_recycled_index_clears_stale_attacker() {
        let mut tracker = SinnerTracker::new();
        tracker.handle_sinner_create(100, 0.0, 0.0, 0.0, 500, 60);
        tracker.record_damage(100, 7); // attacker from first life

        // Sinner dies (entity index 100 recycled), new life begins
        tracker.handle_sinner_update(100, 1, 120);
        tracker.handle_sinner_create(100, 1.0, 2.0, 3.0, 500, 130);

        // Attacker state should be gone -- a damage from the new life should be needed
        let result = tracker.handle_sinner_update(100, 1, 200);
        assert!(result.is_none(), "stale attacker from prior life should be cleared on CREATE");
    }

    // ST-7: record_retaliation accumulates damage from multiple players into the correct snapshot
    #[test]
    fn test_record_retaliation_accumulates_across_hits_and_players() {
        let mut tracker = SinnerTracker::new();
        tracker.handle_sinner_create(100, 0.0, 0.0, 0.0, 500, 60);

        // Player slot 2 hits twice, player slot 5 hits once
        tracker.record_retaliation(100, 2, 80);
        tracker.record_retaliation(100, 2, 80);
        tracker.record_retaliation(100, 5, 80);

        let s = &tracker.snapshots[0];
        assert_eq!(s.retaliation_damage.get(&2), Some(&160));
        assert_eq!(s.retaliation_damage.get(&5), Some(&80));
    }

    // ST-8: handle_sinner_create on a recycled index starts with an empty retaliation_damage map
    #[test]
    fn test_create_on_recycled_index_starts_with_empty_retaliation() {
        let mut tracker = SinnerTracker::new();
        tracker.handle_sinner_create(100, 0.0, 0.0, 0.0, 500, 60);
        tracker.record_retaliation(100, 4, 80); // first life took retaliation

        // Simulate death and recycling
        tracker.handle_sinner_update(100, 1, 120);
        tracker.handle_sinner_create(100, 1.0, 2.0, 3.0, 500, 130);

        // Second snapshot should have empty retaliation_damage
        let s = &tracker.snapshots[1];
        assert!(
            s.retaliation_damage.is_empty(),
            "second life snapshot should start with empty retaliation_damage"
        );
    }

    // ST-9: Sinner alive at match end has all optional death fields as None and
    //        retaliation_damage reflecting all hits taken
    #[test]
    fn test_alive_at_match_end_has_none_death_fields_and_retaliation() {
        let mut tracker = SinnerTracker::new();
        tracker.handle_sinner_create(100, 0.0, 0.0, 0.0, 500, 60);
        tracker.record_retaliation(100, 3, 80);
        tracker.handle_sinner_update(100, 400, 80); // non-death health drop

        let s = &tracker.snapshots[0];
        assert!(s.death_time_s.is_none());
        assert!(s.time_alive_s.is_none());
        assert!(s.killer_player_slot.is_none());
        assert_eq!(s.retaliation_damage.get(&3), Some(&80));
    }
}
