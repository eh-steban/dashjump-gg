use super::*;
use crate::entities::constants::{CAGE_ENTITY_HEALTH, LIFE_ALIVE, LIFE_DEAD, NPC_STATE_COMBAT, NPC_STATE_IDLE, NPC_STATE_INERT};

/// Typical health value for a real lane creep (fighting unit).
const NORMAL_HEALTH: i32 = 350;

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
        tracker.handle_creep_create(i, 1, 2, 100.0 * i as f32, 100.0, 45, NPC_STATE_IDLE, LIFE_ALIVE, NORMAL_HEALTH);
    }

    // Second wave: 4 creeps spawning at second 75 (30s later -- beyond grouping window)
    for i in 4..8_i32 {
        tracker.handle_creep_create(i, 1, 2, 100.0 * i as f32, 500.0, 75, NPC_STATE_IDLE, LIFE_ALIVE, NORMAL_HEALTH);
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
        tracker.handle_creep_create(i, 2, 3, 0.0, 0.0, 45 + i as u32, NPC_STATE_IDLE, LIFE_ALIVE, NORMAL_HEALTH);
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
        tracker.handle_creep_create(i, 1, 2, i as f32 * 50.0, 200.0, 45, NPC_STATE_IDLE, LIFE_ALIVE, NORMAL_HEALTH);
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
    tracker.handle_creep_update(3, 1, 2, 999.0, 888.0, 63, NPC_STATE_COMBAT, LIFE_ALIVE, NORMAL_HEALTH);
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
    tracker.handle_creep_create(1, 1, 2, 0.0, 0.0, 30, NPC_STATE_IDLE, LIFE_ALIVE, NORMAL_HEALTH);

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
    tracker.handle_creep_create(1, 1, 2, 50.0, 50.0, 0, NPC_STATE_IDLE, LIFE_ALIVE, NORMAL_HEALTH);

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
    tracker2.handle_creep_create(99, 1, 2, 10.0, 10.0, 0, NPC_STATE_IDLE, LIFE_ALIVE, NORMAL_HEALTH);
    tracker2.build_creep_snapshot(0, &player_positions);
    tracker2.handle_creep_delete(99, 0);
    tracker2.build_creep_snapshot(1, &player_positions);

    let t2 = tracker2.creep_timelines.get(&99).expect("timeline should exist");
    assert!(t2[0].is_some(), "alive at sec 0");
    assert!(t2[1].is_none(), "dead at sec 1");
}

// -------------------------------------------------------------------------
// NPC_STATE_INERT: CREATE with INERT state must not register the creep
// -------------------------------------------------------------------------
#[test]
fn test_create_with_inert_state_skips_registration() {
    let mut tracker = make_tracker();

    tracker.handle_creep_create(1, 1, 2, 100.0, 100.0, 30, NPC_STATE_INERT, LIFE_ALIVE, NORMAL_HEALTH);

    assert!(
        tracker.active_creeps.is_empty(),
        "INERT creep should not be added to active_creeps"
    );
    assert!(
        tracker.creep_timelines.is_empty(),
        "INERT creep should not produce a timeline entry"
    );
}

// -------------------------------------------------------------------------
// LIFE_DEAD: UPDATE on registered creep with LIFE_DEAD keeps it in active_creeps
// but suppresses snapshot emission.
// -------------------------------------------------------------------------
#[test]
fn test_life_dead_suppresses_snapshot_but_keeps_tracking() {
    let mut tracker = make_tracker();
    let no_players: Vec<(i32, f32, f32)> = vec![];

    // Register a normal creep
    tracker.handle_creep_create(1, 1, 2, 100.0, 100.0, 30, NPC_STATE_IDLE, LIFE_ALIVE, NORMAL_HEALTH);

    // Emit snapshot while ALIVE -- should be Some
    tracker.build_creep_snapshot(30, &no_players);
    let timeline = tracker.creep_timelines.get(&1).expect("timeline should exist");
    assert!(
        timeline[30].is_some(),
        "snapshot at sec 30 should be Some while ALIVE"
    );

    // Transition to DEAD -- creep stays in active_creeps
    tracker.handle_creep_update(1, 1, 2, 100.0, 100.0, 31, NPC_STATE_IDLE, LIFE_DEAD, NORMAL_HEALTH);
    assert!(
        tracker.active_creeps.contains_key(&1),
        "creep should remain in active_creeps after DEAD update (no eviction)"
    );

    // Emit snapshot while DEAD -- should be None (suppressed)
    tracker.build_creep_snapshot(31, &no_players);
    let timeline = tracker.creep_timelines.get(&1).expect("timeline should exist");
    assert!(
        timeline[31].is_none(),
        "snapshot at sec 31 should be None while DEAD"
    );
}

// -------------------------------------------------------------------------
// NPC_STATE_INERT: UPDATE on registered creep with INERT does not pin wave death
// or change wave_id (creep is still registered, snapshots suppressed at snapshot
// layer via LIFE_DEAD, not via npc_state).
// -------------------------------------------------------------------------
#[test]
fn test_inert_on_last_registered_creep_does_not_pin_wave_death() {
    let mut tracker = make_tracker();

    // Two creeps in the same wave
    tracker.handle_creep_create(1, 1, 2, 100.0, 100.0, 30, NPC_STATE_IDLE, LIFE_ALIVE, NORMAL_HEALTH);
    tracker.handle_creep_create(2, 1, 2, 200.0, 200.0, 30, NPC_STATE_IDLE, LIFE_ALIVE, NORMAL_HEALTH);

    // Both receive INERT npc_state but stay ALIVE in life_state -- wave death pin must NOT fire
    tracker.handle_creep_update(1, 1, 2, 101.0, 101.0, 35, NPC_STATE_INERT, LIFE_ALIVE, NORMAL_HEALTH);
    tracker.handle_creep_update(2, 1, 2, 201.0, 201.0, 35, NPC_STATE_INERT, LIFE_ALIVE, NORMAL_HEALTH);

    let meta = tracker.wave_meta.get("1_2_30").expect("wave meta should exist");
    assert!(
        meta.last_death_sec.is_none(),
        "death pin should not fire for npc_state INERT without a DEAD->ALIVE life_state transition"
    );
}

// -------------------------------------------------------------------------
// NPC_STATE_INERT: UPDATE on unregistered creep with INERT does not late-create
// -------------------------------------------------------------------------
#[test]
fn test_update_with_inert_on_unregistered_creep_does_not_create() {
    let mut tracker = make_tracker();

    // UPDATE arrives for an entity not yet in active_creeps, but with INERT state
    tracker.handle_creep_update(5, 1, 2, 100.0, 100.0, 30, NPC_STATE_INERT, LIFE_ALIVE, NORMAL_HEALTH);

    assert!(
        tracker.active_creeps.is_empty(),
        "INERT update on unregistered creep should not trigger late-create"
    );
}

// -------------------------------------------------------------------------
// NPC_STATE: late-create via UPDATE with active state works normally
// -------------------------------------------------------------------------
#[test]
fn test_update_with_active_state_on_unregistered_creep_triggers_late_create() {
    let mut tracker = make_tracker();

    // UPDATE arrives for an entity not yet tracked, with IDLE state and lane assigned
    tracker.handle_creep_update(7, 2, 3, 500.0, 500.0, 45, NPC_STATE_IDLE, LIFE_ALIVE, NORMAL_HEALTH);

    assert!(
        tracker.active_creeps.contains_key(&7),
        "IDLE update on unregistered creep with lane != 0 should trigger late-create"
    );
}

// -------------------------------------------------------------------------
// DEAD->ALIVE transition triggers wave re-assignment and pins death metadata
// on the old wave when this is the last creep in the wave.
// -------------------------------------------------------------------------
#[test]
fn test_dead_to_alive_triggers_wave_reassignment_and_pins_old_wave_death() {
    let mut tracker = make_tracker();

    // Wave 1: creep spawns at sec 30
    tracker.handle_creep_create(1, 1, 2, 1000.0, 1000.0, 30, NPC_STATE_IDLE, LIFE_ALIVE, NORMAL_HEALTH);
    let wave_id_first = tracker.active_creeps[&1].wave_id.clone();
    assert_eq!(wave_id_first, "1_2_30");

    // Normal update -- stays ALIVE, same wave_id
    tracker.handle_creep_update(1, 1, 2, 1050.0, 1050.0, 35, NPC_STATE_COMBAT, LIFE_ALIVE, NORMAL_HEALTH);
    assert_eq!(
        tracker.active_creeps[&1].wave_id, wave_id_first,
        "wave_id should not change for a normal ALIVE update"
    );

    // Creep transitions to DEAD
    tracker.handle_creep_update(1, 1, 2, 1100.0, 1100.0, 60, NPC_STATE_IDLE, LIFE_DEAD, NORMAL_HEALTH);
    assert_eq!(
        tracker.active_creeps[&1].wave_id, wave_id_first,
        "wave_id should not change when transitioning to DEAD"
    );

    // DEAD->ALIVE: entity reused for the next wave at sec 75
    tracker.handle_creep_update(1, 1, 2, 500.0, 500.0, 75, NPC_STATE_IDLE, LIFE_ALIVE, NORMAL_HEALTH);

    let wave_id_second = tracker.active_creeps[&1].wave_id.clone();
    assert_ne!(
        wave_id_first, wave_id_second,
        "recycled entity should get a new wave_id after DEAD->ALIVE; first={} second={}",
        wave_id_first, wave_id_second
    );
    assert_eq!(
        wave_id_second, "1_2_75",
        "new wave_id should reflect the spawn second after DEAD->ALIVE"
    );

    // Old wave death metadata should be pinned at the DEAD->ALIVE transition second
    let meta = tracker.wave_meta.get("1_2_30").expect("old wave meta should exist");
    assert_eq!(
        meta.last_death_sec,
        Some(75),
        "old wave death sec should be pinned at the DEAD->ALIVE transition second"
    );
    assert_eq!(
        meta.last_death_x,
        Some(500.0),
        "old wave death x should be the incoming position"
    );
    assert_eq!(
        meta.last_death_y,
        Some(500.0),
        "old wave death y should be the incoming position"
    );

    // Creep is still tracked continuously -- no gap in active_creeps
    assert!(
        tracker.active_creeps.contains_key(&1),
        "creep should remain in active_creeps after DEAD->ALIVE wave re-assignment"
    );
}

// -------------------------------------------------------------------------
// DEAD->ALIVE: only the transitioning creep's old wave gets death pinned
// when other creeps from that wave are still registered (not yet transitioned).
// -------------------------------------------------------------------------
#[test]
fn test_dead_to_alive_does_not_pin_wave_death_when_other_creeps_remain() {
    let mut tracker = make_tracker();

    // Two creeps in the same wave
    tracker.handle_creep_create(1, 1, 2, 1000.0, 1000.0, 30, NPC_STATE_IDLE, LIFE_ALIVE, NORMAL_HEALTH);
    tracker.handle_creep_create(2, 1, 2, 1100.0, 1100.0, 30, NPC_STATE_IDLE, LIFE_ALIVE, NORMAL_HEALTH);

    // Creep 1 dies then gets DEAD->ALIVE -- creep 2 still holds the old wave
    tracker.handle_creep_update(1, 1, 2, 1100.0, 1100.0, 60, NPC_STATE_IDLE, LIFE_DEAD, NORMAL_HEALTH);
    tracker.handle_creep_update(1, 1, 2, 500.0, 500.0, 75, NPC_STATE_IDLE, LIFE_ALIVE, NORMAL_HEALTH);

    let meta = tracker.wave_meta.get("1_2_30").expect("old wave meta should exist");
    assert!(
        meta.last_death_sec.is_none(),
        "death pin should not fire while creep 2 still belongs to the old wave"
    );

    // Creep 2 also dies and transitions to ALIVE -- now the old wave is empty
    tracker.handle_creep_update(2, 1, 2, 1200.0, 1200.0, 61, NPC_STATE_IDLE, LIFE_DEAD, NORMAL_HEALTH);
    tracker.handle_creep_update(2, 1, 2, 600.0, 600.0, 76, NPC_STATE_IDLE, LIFE_ALIVE, NORMAL_HEALTH);

    let meta = tracker.wave_meta.get("1_2_30").expect("old wave meta should exist");
    assert_eq!(
        meta.last_death_sec,
        Some(76),
        "death pin should fire when the last creep in the old wave transitions DEAD->ALIVE"
    );
}

// -------------------------------------------------------------------------
// Cage entity flag: CREATE with health == CAGE_ENTITY_HEALTH registers with is_cage=true
// -------------------------------------------------------------------------
#[test]
fn test_create_with_cage_health_registers_as_cage() {
    let mut tracker = make_tracker();
    let no_players: Vec<(i32, f32, f32)> = vec![];

    tracker.handle_creep_create(1, 1, 2, 500.0, 1415.0, 30, NPC_STATE_IDLE, LIFE_ALIVE, CAGE_ENTITY_HEALTH);

    assert!(
        tracker.active_creeps.contains_key(&1),
        "cage entity must be added to active_creeps"
    );
    assert!(
        tracker.active_creeps[&1].is_cage,
        "cage entity must have is_cage=true in active_creeps"
    );

    tracker.build_creep_snapshot(30, &no_players);
    let timeline = tracker.creep_timelines.get(&1).expect("timeline should exist");
    let snapshot = timeline[30].as_ref().expect("snapshot should exist at sec 30");
    assert!(snapshot.is_cage, "CreepSnapshot must have is_cage=true for cage entity");
}

// -------------------------------------------------------------------------
// Wave grouping: real creep arriving 13 seconds after cage wave joins that wave
// -------------------------------------------------------------------------
#[test]
fn test_real_creep_joins_preceding_cage_wave() {
    let mut tracker = make_tracker();

    // Cage entity spawns at t=0 (health=1, is_cage=true)
    tracker.handle_creep_create(1, 1, 2, 100.0, 100.0, 0, NPC_STATE_IDLE, LIFE_ALIVE, CAGE_ENTITY_HEALTH);

    // Real creep lands 13 seconds later (health=350, is_cage=false)
    // REAL_CREEP_GROUPING_WINDOW_S=18 means it should look back 13 s and join the cage wave
    tracker.handle_creep_create(2, 1, 2, 200.0, 200.0, 13, NPC_STATE_IDLE, LIFE_ALIVE, NORMAL_HEALTH);

    let cage_wave_id = tracker.active_creeps[&1].wave_id.clone();
    let real_wave_id = tracker.active_creeps[&2].wave_id.clone();

    assert_eq!(
        cage_wave_id, real_wave_id,
        "real creep at t=13 should join the cage wave created at t=0; cage={} real={}",
        cage_wave_id, real_wave_id
    );
    assert_eq!(cage_wave_id, "1_2_0", "wave_id should be anchored to the cage entity's spawn second");
}

// -------------------------------------------------------------------------
// Cage entity flag: CREATE with normal health registers with is_cage=false
// -------------------------------------------------------------------------
#[test]
fn test_create_with_normal_health_not_cage() {
    let mut tracker = make_tracker();
    let no_players: Vec<(i32, f32, f32)> = vec![];

    tracker.handle_creep_create(1, 1, 2, 500.0, 300.0, 30, NPC_STATE_IDLE, LIFE_ALIVE, NORMAL_HEALTH);

    assert!(
        tracker.active_creeps.contains_key(&1),
        "real creep must be added to active_creeps"
    );
    assert!(
        !tracker.active_creeps[&1].is_cage,
        "real creep must have is_cage=false in active_creeps"
    );
    assert_eq!(tracker.wave_meta.len(), 1, "real creep must create one wave_meta entry");

    tracker.build_creep_snapshot(30, &no_players);
    let timeline = tracker.creep_timelines.get(&1).expect("timeline should exist");
    let snapshot = timeline[30].as_ref().expect("snapshot should exist at sec 30");
    assert!(!snapshot.is_cage, "CreepSnapshot must have is_cage=false for normal creep");
}
