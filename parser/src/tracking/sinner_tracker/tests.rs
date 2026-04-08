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

// ST-10: handle_sinner_update on an untracked entity index returns None and pushes no state
#[test]
fn test_update_on_untracked_entity_returns_none() {
    let mut tracker = SinnerTracker::new();

    // No CREATE has been called for index 999
    let result = tracker.handle_sinner_update(999, 1, 50);
    assert!(result.is_none());
    assert!(tracker.snapshots.is_empty());
    assert!(!tracker.is_tracked_sinner(999));
}

// ST-11: time_alive_s handles zero-duration boundary (death at same tick as spawn)
#[test]
fn test_zero_duration_death() {
    let mut tracker = SinnerTracker::new();
    tracker.handle_sinner_create(100, 0.0, 0.0, 0.0, 500, 60);
    tracker.record_damage(100, 9);

    // Death at the exact same second as spawn
    tracker.handle_sinner_update(100, 1, 60);

    let s = &tracker.snapshots[0];
    assert_eq!(s.death_time_s, Some(60));
    assert_eq!(s.time_alive_s, Some(0));
}

// ST-12: record_sinner_death_killer is idempotent -- a second call does not overwrite
#[test]
fn test_record_death_killer_is_idempotent() {
    let mut tracker = SinnerTracker::new();
    tracker.handle_sinner_create(100, 0.0, 0.0, 0.0, 500, 60);
    tracker.handle_sinner_update(100, 1, 120);

    tracker.record_sinner_death_killer(100, 3);
    tracker.record_sinner_death_killer(100, 7); // second call, different slot

    assert_eq!(
        tracker.snapshots[0].killer_player_slot,
        Some(3),
        "second record_sinner_death_killer call must not overwrite"
    );
}

// ST-13: record_retaliation on an untracked entity is a silent no-op
#[test]
fn test_record_retaliation_untracked_is_noop() {
    let mut tracker = SinnerTracker::new();

    // No CREATE for index 999
    tracker.record_retaliation(999, 2, 80);

    assert!(tracker.snapshots.is_empty());
}

// ST-14: record_retaliation after a recycled CREATE routes damage to the new snapshot
#[test]
fn test_record_retaliation_after_recycle_routes_to_new_snapshot() {
    let mut tracker = SinnerTracker::new();
    tracker.handle_sinner_create(100, 0.0, 0.0, 0.0, 500, 60);
    tracker.record_retaliation(100, 2, 50); // routed to first life

    tracker.handle_sinner_update(100, 1, 120); // first life dies
    tracker.handle_sinner_create(100, 0.0, 0.0, 0.0, 500, 130); // recycled

    tracker.record_retaliation(100, 2, 75); // must route to second life

    assert_eq!(tracker.snapshots.len(), 2);
    assert_eq!(tracker.snapshots[0].retaliation_damage.get(&2), Some(&50));
    assert_eq!(tracker.snapshots[1].retaliation_damage.get(&2), Some(&75));
}

// ST-15: First-ever sinner death with no prior record_damage returns None (no attacker known)
#[test]
fn test_first_death_with_no_recorded_damage_returns_none() {
    let mut tracker = SinnerTracker::new();
    tracker.handle_sinner_create(100, 0.0, 0.0, 0.0, 500, 60);

    // No record_damage call -- death happens with no known attacker
    let result = tracker.handle_sinner_update(100, 1, 120);
    assert!(result.is_none());

    // Snapshot should still get its death_time_s / time_alive_s set
    let s = &tracker.snapshots[0];
    assert_eq!(s.death_time_s, Some(120));
    assert_eq!(s.time_alive_s, Some(60));
    assert!(s.killer_player_slot.is_none());
}
