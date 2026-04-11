use super::*;
use crate::domain::BossSnapshot;

/// Insert a minimal tracked boss into `tracker.bosses` so death/update guards
/// recognise the entity. Tests that need to drive `record_death_health` or
/// `update_max_health` without going through a real Entity rely on this.
fn insert_test_boss(tracker: &mut BossTracker, entity_index: i32, max_health: i32) {
    tracker.bosses.insert(
        entity_index,
        BossSnapshot {
            entity_index,
            custom_id: 0,
            boss_name_hash: 0,
            team: 0,
            lane: 0,
            x: 0.0,
            y: 0.0,
            z: 0.0,
            spawn_time_s: 0,
            max_health,
            life_state_on_create: 0,
            death_time_s: None,
            life_state_on_delete: None,
        },
    );
}

// ---------------------------------------------------------------------------
// BT-1: Carry-forward -- before death, last known health is returned per window
// ---------------------------------------------------------------------------
#[test]
fn test_build_health_window_carries_forward_last_known_health() {
    let mut tracker = BossTracker::new();
    let entity_index = 100_i32;

    // Simulate two damage events: health 500 at t=10, then 13 at t=20
    tracker.health_samples.entry(entity_index).or_insert_with(Vec::new).push((10, 500));
    tracker.health_samples.entry(entity_index).or_insert_with(Vec::new).push((20, 13));

    // Build window at second 25 (after both samples)
    tracker.build_health_window(25);

    let output = tracker.get_output();
    let key = entity_index.to_string();
    assert_eq!(
        output["health_timeline"][0][key.as_str()].as_i64(),
        Some(13),
        "should carry forward the last known health (13) before death"
    );
}

// ---------------------------------------------------------------------------
// BT-2: record_death_health -- carry-forward after death shows 0
// ---------------------------------------------------------------------------
#[test]
fn test_record_death_health_causes_zero_in_health_timeline() {
    let mut tracker = BossTracker::new();
    let entity_index = 100_i32;
    let key = entity_index.to_string();

    // Boss must be tracked for record_death_health to record (guards against
    // creep/projectile delete pollution).
    insert_test_boss(&mut tracker, entity_index, 5500);

    // Pre-death health sample
    tracker.health_samples.entry(entity_index).or_insert_with(Vec::new).push((20, 13));

    // Window before death (t=25)
    tracker.build_health_window(25);

    // Boss dies at t=30
    tracker.record_death_health(entity_index, 30);

    // Window after death (t=35)
    tracker.build_health_window(35);

    let output = tracker.get_output();
    let timeline = output["health_timeline"].as_array().unwrap();

    assert_eq!(
        timeline[0][key.as_str()].as_i64(),
        Some(13),
        "window at t=25 (before death): should carry forward 13"
    );
    assert_eq!(
        timeline[1][key.as_str()].as_i64(),
        Some(0),
        "window at t=35 (after death at t=30): should carry forward 0"
    );
}

// ---------------------------------------------------------------------------
// BT-3: record_death_health -- no prior damage samples still records 0
// ---------------------------------------------------------------------------
#[test]
fn test_record_death_health_without_prior_damage_records_zero() {
    let mut tracker = BossTracker::new();
    let entity_index = 200_i32;

    // Boss tracked but no damage events fired before delete (rare edge case).
    insert_test_boss(&mut tracker, entity_index, 5500);

    tracker.record_death_health(entity_index, 50);
    tracker.build_health_window(55);

    let output = tracker.get_output();
    assert_eq!(
        output["health_timeline"][0][entity_index.to_string().as_str()].as_i64(),
        Some(0),
        "boss with no prior damage samples should still record health=0 at deletion"
    );
}

// ---------------------------------------------------------------------------
// BT-4: Window before first sample -- entity absent from health_timeline entry
// ---------------------------------------------------------------------------
#[test]
fn test_health_window_entity_absent_before_first_sample() {
    let mut tracker = BossTracker::new();
    let entity_index = 300_i32;

    // Sample at t=30; build window at t=10 (before any sample exists)
    tracker.health_samples.entry(entity_index).or_insert_with(Vec::new).push((30, 1000));
    tracker.build_health_window(10);

    let output = tracker.get_output();
    assert!(
        output["health_timeline"][0][entity_index.to_string().as_str()].is_null(),
        "entity should be absent from the timeline window before its first health sample"
    );
}

// ---------------------------------------------------------------------------
// BT-5: Two bosses -- death of one does not affect the other's carry-forward
// ---------------------------------------------------------------------------
#[test]
fn test_death_of_one_boss_does_not_affect_sibling() {
    let mut tracker = BossTracker::new();
    let guardian = 100_i32;
    let walker = 200_i32;

    insert_test_boss(&mut tracker, guardian, 5500);
    insert_test_boss(&mut tracker, walker, 9000);

    tracker.health_samples.entry(guardian).or_insert_with(Vec::new).push((10, 5000));
    tracker.health_samples.entry(guardian).or_insert_with(Vec::new).push((20, 13));
    tracker.health_samples.entry(walker).or_insert_with(Vec::new).push((10, 8000));

    // Guardian dies at t=25; walker is still alive
    tracker.record_death_health(guardian, 25);
    tracker.build_health_window(30);

    let output = tracker.get_output();
    let window = &output["health_timeline"][0];

    assert_eq!(
        window[guardian.to_string().as_str()].as_i64(),
        Some(0),
        "dead guardian should carry forward 0"
    );
    assert_eq!(
        window[walker.to_string().as_str()].as_i64(),
        Some(8000),
        "living walker should still carry forward its last known health"
    );
}

// ---------------------------------------------------------------------------
// BT-6: record_death_health -- untracked entities are ignored
// ---------------------------------------------------------------------------
//
// Regression: the DELETE handler in replay_parser.rs fires for every entity
// (creeps, projectiles, breakable map props), not just bosses. Without this
// guard, each non-boss DELETE would create a spurious health=0 entry via
// `or_insert_with`, polluting health_timeline with thousands of phantom 0s.
#[test]
fn test_record_death_health_skips_untracked_entity() {
    let mut tracker = BossTracker::new();
    let creep_index = 2917_i32;

    // Entity is NOT in tracker.bosses (simulating a creep delete reaching the
    // boss tracker via the unguarded DELETE call site).
    tracker.record_death_health(creep_index, 30);
    tracker.build_health_window(35);

    let output = tracker.get_output();
    assert!(
        output["health_timeline"][0][creep_index.to_string().as_str()].is_null(),
        "untracked entity must not appear in health_timeline"
    );
    assert!(
        !tracker.health_samples.contains_key(&creep_index),
        "untracked entity must not create an entry in health_samples"
    );
}

// ---------------------------------------------------------------------------
// BT-7: update_max_health -- tracks Walker scaling on sibling death
// ---------------------------------------------------------------------------
//
// Walkers gain a flat +3000 max_health when a sibling Walker dies (6000 -> 9000
// -> 12000). Without UPDATE handling, BossSnapshot.max_health stays frozen at
// the create-time value and any downstream `health/max_health` ratio is wrong.
#[test]
fn test_update_max_health_reflects_walker_scaling() {
    let mut tracker = BossTracker::new();
    let walker = 304_i32;

    insert_test_boss(&mut tracker, walker, 6000);

    // Sibling Walker dies -- this Walker scales to 9000.
    tracker.update_max_health(walker, 9000);
    assert_eq!(
        tracker.bosses[&walker].max_health,
        9000,
        "max_health should update on Walker scaling"
    );

    // Last Walker standing -- scales to 12000.
    tracker.update_max_health(walker, 12000);
    assert_eq!(
        tracker.bosses[&walker].max_health,
        12000,
        "max_health should update on subsequent scaling"
    );
}

// ---------------------------------------------------------------------------
// BT-8: update_max_health -- ignores untracked entities
// ---------------------------------------------------------------------------
#[test]
fn test_update_max_health_ignores_untracked_entity() {
    let mut tracker = BossTracker::new();

    // Entity not in tracker.bosses; call must be a no-op.
    tracker.update_max_health(999, 9000);

    assert!(
        !tracker.bosses.contains_key(&999),
        "untracked entity must not be inserted by update_max_health"
    );
}

// ---------------------------------------------------------------------------
// BT-9: update_max_health -- ignores non-positive values from teardown
// ---------------------------------------------------------------------------
//
// During entity teardown, max_health can briefly read 0. We must not overwrite
// the snapshot's last good value with that transient.
#[test]
fn test_update_max_health_ignores_non_positive_value() {
    let mut tracker = BossTracker::new();
    let walker = 304_i32;
    insert_test_boss(&mut tracker, walker, 9000);

    tracker.update_max_health(walker, 0);
    assert_eq!(
        tracker.bosses[&walker].max_health,
        9000,
        "transient max_health=0 must not overwrite the stored value"
    );

    tracker.update_max_health(walker, -1);
    assert_eq!(
        tracker.bosses[&walker].max_health,
        9000,
        "negative max_health must not overwrite the stored value"
    );
}

// ---------------------------------------------------------------------------
// BT-10: Walker sibling-death heal -- current health jump is captured
// ---------------------------------------------------------------------------
//
// Wiki: https://deadlock.wiki/Walker -- "The Walkers get a flat 3000 HP
// increase and heal when the other Walkers die." Previously the heal was
// invisible to the timeline: max_health updated but current health samples
// stayed damage-driven, so carry-forward kept the pre-scaling value.
#[test]
fn test_record_current_health_captures_walker_heal() {
    let mut tracker = BossTracker::new();
    let walker = 304_i32;
    let key = walker.to_string();
    insert_test_boss(&mut tracker, walker, 6000);

    // Pre-heal damage sample -- walker at 2000/6000 at t=100.
    tracker
        .health_samples
        .entry(walker)
        .or_insert_with(Vec::new)
        .push((100, 2000));
    tracker.build_health_window(105);

    // Sibling Walker dies at t=110. UPDATE fires: max_health -> 9000 and
    // current health -> 9000 (full heal). Simulate the combined update.
    tracker.update_max_health(walker, 9000);
    tracker.record_current_health(walker, 110, 9000);
    tracker.build_health_window(115);

    let output = tracker.get_output();
    let timeline = output["health_timeline"].as_array().unwrap();

    assert_eq!(
        timeline[0][key.as_str()].as_i64(),
        Some(2000),
        "window at t=105 (pre-heal): should show last damage sample"
    );
    assert_eq!(
        timeline[1][key.as_str()].as_i64(),
        Some(9000),
        "window at t=115 (post-heal): should reflect full heal to new max"
    );
}

// ---------------------------------------------------------------------------
// BT-11: Patron phase 1 -> 2 transition -- health reset captured
// ---------------------------------------------------------------------------
//
// Wiki: https://deadlock.wiki/Patron -- phase 2 starts at 12000 HP even though
// phase 1 and phase 2 share the same starting max. `update_max_health` sees no
// change in this case, but `record_current_health` still captures the jump
// from ~low HP to 12000.
#[test]
fn test_record_current_health_captures_patron_phase_transition() {
    let mut tracker = BossTracker::new();
    let patron = 295_i32;
    let key = patron.to_string();
    insert_test_boss(&mut tracker, patron, 12000);

    // Pre-transition: patron at ~6000/12000 (phase 1 halfway).
    tracker
        .health_samples
        .entry(patron)
        .or_insert_with(Vec::new)
        .push((1500, 6000));
    tracker.build_health_window(1505);

    // Phase transition at t=1510. max_health stays 12000, current health
    // resets to 12000.
    tracker.update_max_health(patron, 12000);
    tracker.record_current_health(patron, 1510, 12000);
    tracker.build_health_window(1515);

    let output = tracker.get_output();
    let timeline = output["health_timeline"].as_array().unwrap();

    assert_eq!(
        timeline[0][key.as_str()].as_i64(),
        Some(6000),
        "window at t=1505 (pre-transition): shows halfway phase 1 health"
    );
    assert_eq!(
        timeline[1][key.as_str()].as_i64(),
        Some(12000),
        "window at t=1515 (post-transition): must reflect phase 2 reset to full"
    );
    assert_eq!(
        tracker.bosses[&patron].max_health,
        12000,
        "max_health unchanged across the phase boundary"
    );
}

// ---------------------------------------------------------------------------
// BT-12: record_current_health -- untracked entities are ignored
// ---------------------------------------------------------------------------
//
// Same defensive guard as `record_death_health`: the UPDATE path in
// `replay_parser` prefilters by `is_boss_entity(hash)`, but a tracked entity
// could be missing from `bosses` if CREATE was skipped. Don't pollute
// health_samples with phantom entries.
#[test]
fn test_record_current_health_skips_untracked_entity() {
    let mut tracker = BossTracker::new();
    let phantom = 777_i32;

    tracker.record_current_health(phantom, 50, 5000);

    assert!(
        !tracker.health_samples.contains_key(&phantom),
        "untracked entity must not create a health_samples entry"
    );
    tracker.build_health_window(55);
    let output = tracker.get_output();
    assert!(
        output["health_timeline"][0][phantom.to_string().as_str()].is_null(),
        "untracked entity must not appear in health_timeline"
    );
}
