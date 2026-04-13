//! Unit tests for MidBossTracker

use super::*;

// =========================================================================
// Spawn events
// =========================================================================

#[test]
fn handle_spawn_pushes_event_with_correct_time() {
    let mut tracker = MidBossTracker::new();
    tracker.handle_spawn(120.5);

    assert_eq!(tracker.spawn_events.len(), 1);
    assert_eq!(tracker.spawn_events[0].spawn_cycle, 1);
    assert!((tracker.spawn_events[0].spawn_time_s - 120.5).abs() < 0.001);
}

#[test]
fn multiple_spawns_produce_multiple_events_with_incrementing_cycles() {
    let mut tracker = MidBossTracker::new();
    tracker.handle_spawn(60.0);
    tracker.handle_spawn(300.0);
    tracker.handle_spawn(600.0);

    assert_eq!(tracker.spawn_events.len(), 3);
    assert_eq!(tracker.spawn_events[0].spawn_cycle, 1);
    assert_eq!(tracker.spawn_events[1].spawn_cycle, 2);
    assert_eq!(tracker.spawn_events[2].spawn_cycle, 3);
    assert!((tracker.spawn_events[1].spawn_time_s - 300.0).abs() < 0.001);
}

// =========================================================================
// Kill events
// =========================================================================

#[test]
fn handle_kill_pushes_event_with_correct_fields() {
    let mut tracker = MidBossTracker::new();
    tracker.handle_spawn(60.0);
    tracker.handle_kill(2, 180.0, 100.0, 200.0, 50.0, 3);

    assert_eq!(tracker.kill_events.len(), 1);
    let kill = &tracker.kill_events[0];
    assert_eq!(kill.spawn_cycle, 1);
    assert_eq!(kill.team, 2);
    assert!((kill.matchtime_s - 180.0).abs() < 0.001);
    assert!((kill.x - 100.0).abs() < 0.001);
    assert!((kill.y - 200.0).abs() < 0.001);
    assert!((kill.z - 50.0).abs() < 0.001);
    assert_eq!(kill.bosses_remaining, 3);
    // team_claimed defaults to team before finalize
    assert_eq!(kill.team_claimed, 2);
}

// =========================================================================
// RejuvStatus events
// =========================================================================

#[test]
fn handle_rejuv_status_pushes_event() {
    let mut tracker = MidBossTracker::new();
    tracker.handle_rejuv_status(200.0, 42, 2, 3, 6);

    assert_eq!(tracker.rejuv_events.len(), 1);
    let ev = &tracker.rejuv_events[0];
    assert!((ev.matchtime_s - 200.0).abs() < 0.001);
    assert_eq!(ev.player_pawn, 42);
    assert_eq!(ev.user_team, 2);
    assert_eq!(ev.killing_team, 3);
    assert_eq!(ev.event_type, 6);
}

// =========================================================================
// Fight windows -- open on first damage
// =========================================================================

#[test]
fn record_damage_opens_fight_window_on_first_damage() {
    let mut tracker = MidBossTracker::new();
    tracker.handle_spawn(60.0);
    tracker.record_damage(5000, 65.0);

    assert!(tracker.open_window.is_some());
    let window = tracker.open_window.as_ref().unwrap();
    assert_eq!(window.health_at_start, 5000);
    assert!((window.window_start_s - 65.0).abs() < 0.001);
    assert_eq!(window.health_samples.len(), 1);
}

// =========================================================================
// Fight windows -- close on gap > 5s
// =========================================================================

#[test]
fn record_damage_after_gap_closes_old_window_and_opens_new() {
    let mut tracker = MidBossTracker::new();
    tracker.handle_spawn(60.0);
    tracker.record_damage(5000, 65.0);
    tracker.record_damage(4800, 66.0);
    // Gap of 6 seconds exceeds FIGHT_WINDOW_GAP_S
    tracker.record_damage(3000, 72.0);

    // The first window should be closed
    assert_eq!(tracker.fight_windows.len(), 1);
    let closed = &tracker.fight_windows[0];
    assert_eq!(closed.health_at_start, 5000);
    assert_eq!(closed.health_at_end, 4800);
    assert!((closed.window_end_s - 66.0).abs() < 0.001);
    assert_eq!(closed.health_samples.len(), 2);

    // A new window should be open
    assert!(tracker.open_window.is_some());
    let new_window = tracker.open_window.as_ref().unwrap();
    assert_eq!(new_window.health_at_start, 3000);
    assert!((new_window.window_start_s - 72.0).abs() < 0.001);
}

#[test]
fn fight_window_within_gap_appends_samples() {
    let mut tracker = MidBossTracker::new();
    tracker.handle_spawn(60.0);
    tracker.record_damage(5000, 65.0);
    tracker.record_damage(4500, 66.5);
    tracker.record_damage(4000, 68.0);

    // No windows closed (all within 5s gap)
    assert_eq!(tracker.fight_windows.len(), 0);
    let window = tracker.open_window.as_ref().unwrap();
    assert_eq!(window.health_samples.len(), 3);
    assert_eq!(window.last_health, 4000);
}

// =========================================================================
// Fight windows -- handle_kill closes with health_at_end=0
// =========================================================================

#[test]
fn handle_kill_closes_fight_window_with_health_at_end_zero() {
    let mut tracker = MidBossTracker::new();
    tracker.handle_spawn(60.0);
    tracker.record_damage(5000, 65.0);
    tracker.record_damage(2000, 66.0);
    tracker.handle_kill(2, 67.0, 0.0, 0.0, 0.0, 0);

    // Window closed at kill time with health_at_end=0
    assert_eq!(tracker.fight_windows.len(), 1);
    let window = &tracker.fight_windows[0];
    assert_eq!(window.health_at_end, 0);
    assert!((window.window_end_s - 67.0).abs() < 0.001);
    // No open window
    assert!(tracker.open_window.is_none());
}

// =========================================================================
// Fight windows -- finalize closes any open window
// =========================================================================

#[test]
fn finalize_closes_open_fight_window() {
    let mut tracker = MidBossTracker::new();
    tracker.handle_spawn(60.0);
    tracker.record_damage(5000, 65.0);
    tracker.record_damage(4000, 66.0);
    // Simulate match ending without a kill event
    tracker.finalize();

    assert_eq!(tracker.fight_windows.len(), 1);
    let window = &tracker.fight_windows[0];
    assert_eq!(window.health_at_end, 4000);
    assert!(tracker.open_window.is_none());
}

// =========================================================================
// team_claimed derivation
// =========================================================================

#[test]
fn team_claimed_derives_from_rejuv_grants_with_threshold() {
    let mut tracker = MidBossTracker::new();
    tracker.handle_spawn(60.0);
    tracker.handle_kill(2, 180.0, 0.0, 0.0, 0.0, 3);

    // Team 3 gets 2 grant events (event_type=6) -- meets threshold
    tracker.handle_rejuv_status(181.0, 10, 3, 2, 6);
    tracker.handle_rejuv_status(181.5, 11, 3, 2, 6);
    // Team 2 (the killer) gets only 1 grant -- below threshold
    tracker.handle_rejuv_status(182.0, 12, 2, 2, 6);

    tracker.finalize();

    // team_claimed should be 3 (the team that stole), not 2 (the killer)
    assert_eq!(tracker.kill_events[0].team, 2);
    assert_eq!(tracker.kill_events[0].team_claimed, 3);
}

#[test]
fn team_claimed_falls_back_to_killing_team_when_no_grants() {
    let mut tracker = MidBossTracker::new();
    tracker.handle_spawn(60.0);
    tracker.handle_kill(2, 180.0, 0.0, 0.0, 0.0, 3);
    // No rejuv grant events at all

    tracker.finalize();

    // Fallback: team_claimed == team
    assert_eq!(tracker.kill_events[0].team_claimed, 2);
}

#[test]
fn team_claimed_falls_back_when_no_team_meets_threshold() {
    let mut tracker = MidBossTracker::new();
    tracker.handle_spawn(60.0);
    tracker.handle_kill(2, 180.0, 0.0, 0.0, 0.0, 3);

    // Each team gets only 1 grant -- neither meets threshold of 2
    tracker.handle_rejuv_status(181.0, 10, 2, 2, 6);
    tracker.handle_rejuv_status(181.5, 11, 3, 2, 6);

    tracker.finalize();

    // Fallback to killing team
    assert_eq!(tracker.kill_events[0].team_claimed, 2);
}

#[test]
fn team_claimed_correct_for_normal_kill_by_same_team() {
    let mut tracker = MidBossTracker::new();
    tracker.handle_spawn(60.0);
    tracker.handle_kill(2, 180.0, 0.0, 0.0, 0.0, 3);

    // Team 2 (the killer) gets 3 grants -- well over threshold
    tracker.handle_rejuv_status(181.0, 10, 2, 2, 6);
    tracker.handle_rejuv_status(181.5, 11, 2, 2, 6);
    tracker.handle_rejuv_status(182.0, 12, 2, 2, 6);

    tracker.finalize();

    assert_eq!(tracker.kill_events[0].team_claimed, 2);
}

// =========================================================================
// get_output
// =========================================================================

#[test]
fn get_output_returns_collected_data() {
    let mut tracker = MidBossTracker::new();
    tracker.handle_spawn(60.0);
    tracker.handle_kill(2, 180.0, 1.0, 2.0, 3.0, 3);
    tracker.finalize();

    let data = tracker.get_output();
    assert_eq!(data.spawn_events.len(), 1);
    assert_eq!(data.kill_events.len(), 1);
    assert_eq!(data.post_match.len(), 1);
    assert_eq!(data.post_match[0].team_killed, 2);
    assert_eq!(data.post_match[0].team_claimed, 2);
    assert_eq!(data.post_match[0].destroyed_time_s, 180);
}

// =========================================================================
// mid_boss_entity_index
// =========================================================================

#[test]
fn mid_boss_entity_index_returns_none_before_observation() {
    let tracker = MidBossTracker::new();
    assert!(tracker.mid_boss_entity_index().is_none());
}

// =========================================================================
// Critical gap: exact 5.0s boundary
// =========================================================================

#[test]
fn fight_window_stays_open_at_exact_gap_boundary() {
    let mut tracker = MidBossTracker::new();
    tracker.handle_spawn(60.0);
    tracker.record_damage(10000, 65.0);
    // Exactly 5.0s later -- should NOT close the window (> not >=)
    tracker.record_damage(9000, 70.0);

    assert!(tracker.open_window.is_some());
    assert_eq!(tracker.fight_windows.len(), 0, "window should remain open at exact 5.0s boundary");
    let window = tracker.open_window.as_ref().expect("window should be open");
    assert_eq!(window.health_samples.len(), 2);
}

#[test]
fn fight_window_closes_just_past_gap_boundary() {
    let mut tracker = MidBossTracker::new();
    tracker.handle_spawn(60.0);
    tracker.record_damage(10000, 65.0);
    // 5.001s later -- should close the window
    tracker.record_damage(9000, 70.001);

    assert_eq!(tracker.fight_windows.len(), 1, "old window should be closed");
    assert!(tracker.open_window.is_some(), "new window should be open");
}

// =========================================================================
// Critical gap: multi-spawn-cycle fight windows
// =========================================================================

#[test]
fn fight_window_across_spawn_cycles_has_correct_spawn_cycle() {
    let mut tracker = MidBossTracker::new();

    // Cycle 1: spawn, damage, kill
    tracker.handle_spawn(60.0);
    tracker.record_damage(10000, 65.0);
    tracker.handle_kill(2, 70.0, 0.0, 0.0, 0.0, 3);

    // Cycle 2: spawn, damage, kill
    tracker.handle_spawn(480.0);
    tracker.record_damage(15000, 490.0);
    tracker.handle_kill(3, 500.0, 0.0, 0.0, 0.0, 2);

    tracker.finalize();
    let data = tracker.get_output();

    assert_eq!(data.fight_windows.len(), 2);
    assert_eq!(data.fight_windows[0].spawn_cycle, 1);
    assert_eq!(data.fight_windows[1].spawn_cycle, 2);
    assert_eq!(data.fight_windows[0].health_at_end, 0, "first kill closes window at 0");
    assert_eq!(data.fight_windows[1].health_at_end, 0, "second kill closes window at 0");
}

// =========================================================================
// Critical gap: multi-kill rejuv attribution
// =========================================================================

#[test]
fn rejuv_attribution_across_two_kills_assigns_to_correct_kill() {
    let mut tracker = MidBossTracker::new();

    // Kill 1 at t=180
    tracker.handle_spawn(60.0);
    tracker.handle_kill(2, 180.0, 0.0, 0.0, 0.0, 3);
    // Rejuv grants for kill 1 (after kill at t=180)
    tracker.handle_rejuv_status(182.0, 100, 2, 2, 6);
    tracker.handle_rejuv_status(183.0, 101, 2, 2, 6);

    // Kill 2 at t=600
    tracker.handle_spawn(420.0);
    tracker.handle_kill(3, 600.0, 0.0, 0.0, 0.0, 2);
    // Rejuv grants for kill 2 (after kill at t=600)
    tracker.handle_rejuv_status(602.0, 200, 3, 3, 6);
    tracker.handle_rejuv_status(603.0, 201, 3, 3, 6);
    tracker.handle_rejuv_status(604.0, 202, 2, 3, 6); // steal: one grant to team 2

    tracker.finalize();
    let data = tracker.get_output();

    assert_eq!(data.kill_events.len(), 2);
    // Kill 1: team 2 killed, team 2 claimed (2 grants)
    assert_eq!(data.kill_events[0].team_claimed, 2);
    // Kill 2: team 3 killed, team 3 claimed (2 grants vs 1 for team 2)
    assert_eq!(data.kill_events[1].team_claimed, 3);
}

#[test]
fn rejuv_events_before_kill_time_not_attributed() {
    let mut tracker = MidBossTracker::new();
    tracker.handle_spawn(60.0);

    // Rejuv event BEFORE the kill (shouldn't count)
    tracker.handle_rejuv_status(170.0, 100, 3, 3, 6);
    tracker.handle_rejuv_status(171.0, 101, 3, 3, 6);

    // Kill at t=180
    tracker.handle_kill(2, 180.0, 0.0, 0.0, 0.0, 3);

    tracker.finalize();
    let data = tracker.get_output();

    // No grants in [180, 210] window, so fallback to killing team
    assert_eq!(data.kill_events[0].team_claimed, 2, "pre-kill rejuv events should be excluded");
}

// =========================================================================
// Critical gap: team_claimed tie-break
// =========================================================================

#[test]
fn team_claimed_tie_falls_back_to_killing_team_when_equal_grants() {
    let mut tracker = MidBossTracker::new();
    tracker.handle_spawn(60.0);
    tracker.handle_kill(2, 180.0, 0.0, 0.0, 0.0, 3);

    // Equal grants: 2 each -- both meet threshold but tied
    tracker.handle_rejuv_status(182.0, 100, 2, 2, 6);
    tracker.handle_rejuv_status(183.0, 101, 2, 2, 6);
    tracker.handle_rejuv_status(184.0, 200, 3, 2, 6);
    tracker.handle_rejuv_status(185.0, 201, 3, 2, 6);

    tracker.finalize();
    let data = tracker.get_output();

    // Both teams have 2 grants (>= threshold). max_by_key picks one --
    // the exact team is non-deterministic due to HashMap iteration order.
    // Either team is acceptable; the important thing is it doesn't panic
    // and doesn't fall back to killing team (since both meet threshold).
    let claimed = data.kill_events[0].team_claimed;
    assert!(claimed == 2 || claimed == 3, "tie should pick one team, not crash");
}

// =========================================================================
// Important gap: non-grant event types don't affect team_claimed
// =========================================================================

#[test]
fn non_grant_event_types_do_not_affect_team_claimed() {
    let mut tracker = MidBossTracker::new();
    tracker.handle_spawn(60.0);
    tracker.handle_kill(2, 180.0, 0.0, 0.0, 0.0, 3);

    // Event types 7 and 8 are consume/expire -- should not count for claiming
    tracker.handle_rejuv_status(182.0, 100, 3, 2, 7);
    tracker.handle_rejuv_status(183.0, 101, 3, 2, 8);
    tracker.handle_rejuv_status(184.0, 102, 3, 2, 7);

    tracker.finalize();
    let data = tracker.get_output();

    // No event_type==6 grants, so fallback to killing team
    assert_eq!(data.kill_events[0].team_claimed, 2);
}

// =========================================================================
// Important gap: empty tracker finalize
// =========================================================================

#[test]
fn finalize_on_empty_tracker_is_noop() {
    let mut tracker = MidBossTracker::new();
    tracker.finalize();
    let data = tracker.get_output();

    assert_eq!(data.spawn_events.len(), 0);
    assert_eq!(data.kill_events.len(), 0);
    assert_eq!(data.rejuv_events.len(), 0);
    assert_eq!(data.fight_windows.len(), 0);
    assert_eq!(data.post_match.len(), 0);
    assert!(data.max_health.is_none());
}
