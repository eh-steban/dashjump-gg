use super::*;

/// Build a CreepTracker using a dummy lane_key (0 -- not used by the new API).
fn make_tracker() -> CreepTracker {
    CreepTracker::new(0)
}

// -------------------------------------------------------------------------
// A4-1: Wave grouping -- 4 creeps at sec 45, then 4 at sec 75 -> two wave_ids
// -------------------------------------------------------------------------
#[test]
fn test_two_waves_far_apart_get_distinct_wave_ids() {
    let mut tracker = make_tracker();

    // First wave: 4 creeps spawning at second 45
    for i in 0..4_i32 {
        tracker.handle_creep_create(i, 1, 2, 100.0 * i as f32, 100.0, 45);
    }

    // Second wave: 4 creeps spawning at second 75 (30s later -- beyond grouping window)
    for i in 4..8_i32 {
        tracker.handle_creep_create(i, 1, 2, 100.0 * i as f32, 500.0, 75);
    }

    // Collect distinct wave_ids from active creeps
    let ids: std::collections::HashSet<String> = tracker
        .active_creeps
        .values()
        .map(|c| c.wave_id.clone())
        .collect();

    assert_eq!(ids.len(), 2, "Expected two distinct wave_ids, got: {:?}", ids);
    assert!(
        ids.contains("1_2_45"),
        "Expected wave_id '1_2_45', got: {:?}",
        ids
    );
    assert!(
        ids.contains("1_2_75"),
        "Expected wave_id '1_2_75', got: {:?}",
        ids
    );
}

// -------------------------------------------------------------------------
// A4-2: Same wave grouping -- 4 creeps at secs 45, 46, 47, 48 -> same wave_id
// -------------------------------------------------------------------------
#[test]
fn test_creeps_spawning_within_window_get_same_wave_id() {
    let mut tracker = make_tracker();

    // 4 creeps spawning 1 second apart -- all within WAVE_GROUPING_WINDOW_S (5)
    for i in 0..4_i32 {
        tracker.handle_creep_create(i, 2, 3, 0.0, 0.0, 45 + i as u32);
    }

    let ids: std::collections::HashSet<String> = tracker
        .active_creeps
        .values()
        .map(|c| c.wave_id.clone())
        .collect();

    assert_eq!(
        ids.len(),
        1,
        "Expected one wave_id for closely-spaced spawns, got: {:?}",
        ids
    );
    // All creeps should share the wave_id generated at sec 45 (first creep's spawn)
    assert!(
        ids.contains("2_3_45"),
        "Expected wave_id '2_3_45', got: {:?}",
        ids
    );
}

// -------------------------------------------------------------------------
// A4-3: Death pin -- last creep death sets wave_meta; earlier deaths do not
// -------------------------------------------------------------------------
#[test]
fn test_wave_death_pin_set_only_on_last_creep_death() {
    let mut tracker = make_tracker();

    // Spawn 4 creeps in the same wave at second 45
    for i in 0..4_i32 {
        tracker.handle_creep_create(i, 1, 2, i as f32 * 50.0, 200.0, 45);
    }

    // Kill first 3 -- last_death_sec should remain None
    tracker.handle_creep_delete(0, 60);
    tracker.handle_creep_delete(1, 61);
    tracker.handle_creep_delete(2, 62);

    let meta = tracker.wave_meta.get("1_2_45").expect("wave meta should exist");
    assert!(
        meta.last_death_sec.is_none(),
        "last_death_sec should be None while one creep is still alive"
    );

    // Kill the last creep at a specific position
    // Creep 3 was created at x=150.0, y=200.0. Move it first via update.
    tracker.handle_creep_update(3, 1, 2, 999.0, 888.0, 63);
    tracker.handle_creep_delete(3, 65);

    let meta = tracker.wave_meta.get("1_2_45").expect("wave meta should exist");
    assert_eq!(meta.last_death_sec, Some(65), "last_death_sec should be 65");
    assert_eq!(
        meta.last_death_x,
        Some(999.0),
        "last_death_x should match last position"
    );
    assert_eq!(
        meta.last_death_y,
        Some(888.0),
        "last_death_y should match last position"
    );
}

// -------------------------------------------------------------------------
// A4-4: Nearby player radius -- 1400 units included, 1600 units excluded
// -------------------------------------------------------------------------
#[test]
fn test_nearby_player_radius() {
    let mut tracker = make_tracker();

    // One creep at origin
    tracker.handle_creep_create(1, 1, 2, 0.0, 0.0, 30);

    // Player A is 1400 units away (within radius)
    // Player B is 1600 units away (outside radius)
    let player_positions: Vec<(i32, f32, f32)> = vec![(10, 1400.0, 0.0), (20, 1600.0, 0.0)];

    tracker.build_creep_snapshot(30, &player_positions);

    let timeline = tracker.creep_timelines.get(&1).expect("timeline should exist");
    let snapshot = timeline[30].as_ref().expect("snapshot should exist at sec 30");

    assert!(
        snapshot.nearby_players.contains(&10),
        "Player at 1400 units should be nearby"
    );
    assert!(
        !snapshot.nearby_players.contains(&20),
        "Player at 1600 units should NOT be nearby"
    );
}

// -------------------------------------------------------------------------
// Additional: Timeline alignment -- index 0 = match second 0
// -------------------------------------------------------------------------
#[test]
fn test_timeline_index_matches_match_second() {
    let mut tracker = make_tracker();

    // Spawn a creep at match second 0
    tracker.handle_creep_create(1, 1, 2, 50.0, 50.0, 0);

    let player_positions: Vec<(i32, f32, f32)> = vec![];

    tracker.build_creep_snapshot(0, &player_positions);
    tracker.build_creep_snapshot(1, &player_positions);
    tracker.build_creep_snapshot(2, &player_positions);

    let timeline = tracker.creep_timelines.get(&1).expect("timeline should exist");

    // timeline[0] = match second 0 -- creep is alive
    assert!(
        timeline[0].is_some(),
        "timeline[0] should be Some (creep alive at match second 0)"
    );
    // timeline[1] = match second 1 -- creep is alive
    assert!(
        timeline[1].is_some(),
        "timeline[1] should be Some (creep alive at match second 1)"
    );

    // Kill the creep at second 1 and emit second 2
    tracker.handle_creep_delete(1, 1);
    // Re-emit second 2 after death (timeline was already pushed for sec 2 above before death)
    // So let's use a fresh tracker to test this correctly
    let mut tracker2 = make_tracker();
    tracker2.handle_creep_create(99, 1, 2, 10.0, 10.0, 0);
    tracker2.build_creep_snapshot(0, &player_positions);
    tracker2.handle_creep_delete(99, 0);
    tracker2.build_creep_snapshot(1, &player_positions);

    let t2 = tracker2.creep_timelines.get(&99).expect("timeline should exist");
    assert!(t2[0].is_some(), "alive at sec 0");
    assert!(t2[1].is_none(), "dead at sec 1");
}
