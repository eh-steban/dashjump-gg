use super::*;

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

    // No damage events fired -- boss deleted immediately (rare edge case)
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
