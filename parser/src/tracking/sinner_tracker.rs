//! Sinner Sacrifice entity tracking

use std::collections::HashMap;

use crate::domain::SinnerSnapshot;

/// Tracks Sinner Sacrifice entities throughout the match.
///
/// Entity indices are recycled on sinner respawn. Each spawn (initial or respawn) produces a separate `SinnerSnapshot`. On CREATE the `last_attacker` entry for the recycled index is removed and `last_health` is overwritten with the new life's `max_health`, so stale attribution from a prior life does not carry over.
///
/// Assumes `max_health > 1` for every sinner. The death signal is `health == 1 && prev_health > 1`; a sinner spawned with `max_health == 1` would immediately look "dead" on its first CREATE. Confirmed `max_health == 500` in parsed replays.
#[derive(Debug)]
pub struct SinnerTracker {
    snapshots: Vec<SinnerSnapshot>,
    /// Last observed health per entity index, used to detect the death signal (health transitions from >1 to 1). Entries are inserted on CREATE and overwritten on UPDATE; they are never removed, so a key's presence means "has been tracked at some point in this match", not "currently alive".
    last_health: HashMap<i32, i32>,
    /// Last attacker entity index per sinner entity index, updated on every damage event. Snapshot at death time gives the killing attacker. Cleared on CREATE when an index is recycled.
    last_attacker: HashMap<i32, i32>,
}

impl SinnerTracker {
    pub fn new() -> Self {
        Self {
            snapshots: Vec::new(),
            last_health: HashMap::new(),
            last_attacker: HashMap::new(),
        }
    }

    /// Called when a sinner entity is created (initial spawn or respawn after entity index recycling). Pushes a new snapshot and resets per-entity state: `last_attacker` is removed so the prior life's killer is forgotten, and `last_health` is overwritten with `max_health` so the next UPDATE sees a fresh baseline.
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

    /// Called on every damage event where the victim is a sinner. Records the attacker for later kill attribution. No-op if the sinner entity index has never been tracked in this match.
    pub fn record_damage(&mut self, victim_entity_index: i32, attacker_entity_index: i32) {
        if self.is_tracked_sinner(victim_entity_index) {
            self.last_attacker
                .insert(victim_entity_index, attacker_entity_index);
        }
    }

    /// Called on every damage event where the attacker is a sinner. Accumulates raw outgoing damage into the most recent snapshot for this sinner, keyed by the victim player's lobby slot. No-op if the entity index has never been tracked.
    ///
    /// Performance: the reverse scan is O(lives) where `lives` is the number of snapshots pushed for this entity index so far (typically 1-5 per match). Accepted as cheap relative to the per-damage hot path.
    pub fn record_retaliation(&mut self, sinner_entity_index: i32, player_slot: u32, damage: i32) {
        if !self.is_tracked_sinner(sinner_entity_index) {
            return;
        }
        // Find the last snapshot for this entity index (most recently pushed). This routes retaliation to the current life after an index is recycled, since `handle_sinner_create` pushes a fresh snapshot at the end of the vector.
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

    /// Called on every health update for a sinner entity. Updates `last_health` and detects the death signal (`health == 1 && prev_health > 1`). On death, fills `death_time_s` and `time_alive_s` on the latest snapshot and returns the last known attacker entity index. Returns `None` if this is not a death transition, if the entity has never been tracked, or if no damage has been recorded against this sinner yet.
    ///
    /// Note: Deadlock sinners signal death via health==1, not health==0. Delta compression skips the zero-health packet when the entity slot is recycled, so health==0 is unreliable as a death signal.
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

    /// Called after kill attribution is resolved. Sets `killer_player_slot` on the most recent dead snapshot for this entity index that does not yet have a killer assigned. Idempotent: a second call for the same death is a no-op because the guard `killer_player_slot.is_none()` excludes the already-assigned snapshot.
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

    /// Returns `true` if the given entity index has been CREATEd as a sinner at any point in this match. Note this is "has been tracked", not "currently alive" -- entries in `last_health` are never removed, so this returns `true` for dead sinners as well as live ones. Used as a cheap gate on damage-event handlers to skip non-sinner entities.
    pub fn is_tracked_sinner(&self, entity_index: i32) -> bool {
        self.last_health.contains_key(&entity_index)
    }

    /// Returns all sinner snapshots collected during the match (all lives).
    pub fn get_output(&self) -> &Vec<SinnerSnapshot> {
        &self.snapshots
    }
}

#[cfg(test)]
mod tests;
