//! Unit tests for MidBossTracker

use super::*;

// =========================================================================
// Test helpers
// =========================================================================

/// Simulate a kill preceded by a damage burst, so the fight window anchors
/// rejuv attribution on last-damage time. Matches the real parser flow where
/// record_damage always fires before handle_kill.
fn simulate_damage_and_kill(
    tracker: &mut MidBossTracker,
    last_damage_time_s: f32,
    kill_time_s: f32,
) {
    tracker.record_damage(10_000, last_damage_time_s - 5.0);
    tracker.record_damage(5_000, last_damage_time_s);
    tracker.handle_kill(kill_time_s, 0.0, 0.0, 0.0, 0);
}

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
// Kill events: shape and placeholder team fields
// =========================================================================

#[test]
fn handle_kill_pushes_event_with_placeholder_teams_until_finalize() {
    let mut tracker = MidBossTracker::new();
    tracker.handle_spawn(60.0);
    tracker.handle_kill(180.0, 100.0, 200.0, 50.0, 3);

    assert_eq!(tracker.kill_events.len(), 1);
    let kill = &tracker.kill_events[0];
    assert_eq!(kill.spawn_cycle, 1);
    assert!((kill.matchtime_s - 180.0).abs() < 0.001);
    assert_eq!(kill.x, 100.0);
    assert_eq!(kill.y, 200.0);
    assert_eq!(kill.z, 50.0);
    assert_eq!(kill.bosses_remaining, 3);
    // Placeholders until finalize() populates from rejuv events.
    assert_eq!(kill.team_killed, 0);
    assert_eq!(kill.team_claimed, 0);
    assert_eq!(kill.rejuvs_by_team.get("2"), Some(&0));
    assert_eq!(kill.rejuvs_by_team.get("3"), Some(&0));
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
// Fight windows -- handle_kill closes at last-damage time (not kill time)
// =========================================================================

#[test]
fn handle_kill_closes_fight_window_at_last_damage_time() {
    let mut tracker = MidBossTracker::new();
    tracker.handle_spawn(60.0);
    tracker.record_damage(5000, 65.0);
    tracker.record_damage(2000, 66.0);
    // Kill message fires several seconds after last damage (BossKilled.gametime lag).
    tracker.handle_kill(73.0, 0.0, 0.0, 0.0, 0);

    assert_eq!(tracker.fight_windows.len(), 1);
    let window = &tracker.fight_windows[0];
    assert_eq!(window.health_at_end, 0);
    // window_end_s must be the last-damage time, not the kill message time --
    // this is the anchor used for rejuv attribution.
    assert!((window.window_end_s - 66.0).abs() < 0.001);
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
// team_killed sourced from RejuvStatus.killing_team
// =========================================================================

#[test]
fn team_killed_sourced_from_rejuv_killing_team() {
    let mut tracker = MidBossTracker::new();
    tracker.handle_spawn(60.0);
    simulate_damage_and_kill(&mut tracker, 175.0, 180.0);

    // Three clean grants to team 2, killing_team=2 stamped on each.
    tracker.handle_rejuv_status(176.0, 10, 2, 2, 6);
    tracker.handle_rejuv_status(176.5, 11, 2, 2, 6);
    tracker.handle_rejuv_status(177.0, 12, 2, 2, 6);

    tracker.finalize();

    let kill = &tracker.kill_events[0];
    assert_eq!(kill.team_killed, 2, "team_killed must come from rejuv.killing_team");
    assert_eq!(kill.team_claimed, 2);
    assert_eq!(kill.rejuvs_by_team.get("2"), Some(&3));
    assert_eq!(kill.rejuvs_by_team.get("3"), Some(&0));
}

#[test]
fn team_killed_falls_back_to_zero_when_no_rejuv_events() {
    let mut tracker = MidBossTracker::new();
    tracker.handle_spawn(60.0);
    simulate_damage_and_kill(&mut tracker, 175.0, 180.0);
    // No rejuv events at all.

    tracker.finalize();

    let kill = &tracker.kill_events[0];
    assert_eq!(kill.team_killed, 0);
    assert_eq!(kill.team_claimed, 0);
    assert_eq!(kill.rejuvs_by_team.get("2"), Some(&0));
    assert_eq!(kill.rejuvs_by_team.get("3"), Some(&0));
}

// =========================================================================
// team_claimed derivation (strict majority 2-of-3)
// =========================================================================

#[test]
fn team_claimed_equals_team_killed_on_clean_kill() {
    let mut tracker = MidBossTracker::new();
    tracker.handle_spawn(60.0);
    simulate_damage_and_kill(&mut tracker, 175.0, 180.0);

    // Clean kill: team 2 kills and gets all 3 grants.
    tracker.handle_rejuv_status(176.0, 10, 2, 2, 6);
    tracker.handle_rejuv_status(176.5, 11, 2, 2, 6);
    tracker.handle_rejuv_status(177.0, 12, 2, 2, 6);

    tracker.finalize();

    let kill = &tracker.kill_events[0];
    assert_eq!(kill.team_killed, 2);
    assert_eq!(kill.team_claimed, 2);
}

#[test]
fn team_claimed_diverges_from_team_killed_on_2v1_steal() {
    let mut tracker = MidBossTracker::new();
    tracker.handle_spawn(60.0);
    simulate_damage_and_kill(&mut tracker, 175.0, 180.0);

    // Steal: team 2 killed (killing_team=2) but team 3 got 2 of 3 grants.
    tracker.handle_rejuv_status(176.0, 10, 3, 2, 6);
    tracker.handle_rejuv_status(176.5, 11, 3, 2, 6);
    tracker.handle_rejuv_status(177.0, 12, 2, 2, 6);

    tracker.finalize();

    let kill = &tracker.kill_events[0];
    assert_eq!(kill.team_killed, 2);
    assert_eq!(kill.team_claimed, 3, "team with strict majority (2 of 3) wins");
    assert_eq!(kill.rejuvs_by_team.get("2"), Some(&1));
    assert_eq!(kill.rejuvs_by_team.get("3"), Some(&2));
}

#[test]
fn non_grant_event_types_do_not_affect_team_claimed() {
    let mut tracker = MidBossTracker::new();
    tracker.handle_spawn(60.0);
    simulate_damage_and_kill(&mut tracker, 175.0, 180.0);

    // Event types 7/8 are consume/expire -- should not count for claiming.
    // killing_team is still observed (so team_killed is populated), but no
    // event_type=6 grants means fallback to killing team.
    tracker.handle_rejuv_status(176.0, 10, 3, 2, 7);
    tracker.handle_rejuv_status(177.0, 11, 3, 2, 8);
    tracker.handle_rejuv_status(178.0, 12, 3, 2, 7);

    tracker.finalize();

    let kill = &tracker.kill_events[0];
    assert_eq!(kill.team_killed, 2);
    assert_eq!(kill.team_claimed, 2, "no grants -> fallback to killing team");
    assert_eq!(kill.rejuvs_by_team.get("2"), Some(&0));
    assert_eq!(kill.rejuvs_by_team.get("3"), Some(&0));
}

// =========================================================================
// Attribution window: last-damage anchor + 30s
// =========================================================================

#[test]
fn rejuv_events_before_last_damage_anchor_not_attributed() {
    let mut tracker = MidBossTracker::new();
    tracker.handle_spawn(60.0);
    // Last damage at 175, kill message at 180.
    simulate_damage_and_kill(&mut tracker, 175.0, 180.0);

    // Rejuv BEFORE the last-damage anchor -- should be excluded.
    tracker.handle_rejuv_status(170.0, 100, 3, 3, 6);
    tracker.handle_rejuv_status(171.0, 101, 3, 3, 6);

    tracker.finalize();

    let kill = &tracker.kill_events[0];
    assert_eq!(kill.team_killed, 0, "no rejuv inside window -> no killing team");
    assert_eq!(kill.rejuvs_by_team.get("3"), Some(&0));
}

#[test]
fn rejuv_events_past_30s_lookahead_not_attributed() {
    let mut tracker = MidBossTracker::new();
    tracker.handle_spawn(60.0);
    // Anchor = 175 (last damage); window closes at 205.
    simulate_damage_and_kill(&mut tracker, 175.0, 180.0);

    // Rejuv grants at 210 (beyond 30s lookahead) -- excluded.
    tracker.handle_rejuv_status(210.0, 100, 2, 2, 6);
    tracker.handle_rejuv_status(210.5, 101, 2, 2, 6);

    tracker.finalize();

    let kill = &tracker.kill_events[0];
    assert_eq!(kill.rejuvs_by_team.get("2"), Some(&0));
}

#[test]
fn rejuv_attribution_across_two_kills_assigns_to_correct_kill() {
    let mut tracker = MidBossTracker::new();

    // Kill 1: last damage at 175, kill message at 180.
    tracker.handle_spawn(60.0);
    simulate_damage_and_kill(&mut tracker, 175.0, 180.0);
    // Grants for kill 1 (team 2 clean).
    tracker.handle_rejuv_status(182.0, 100, 2, 2, 6);
    tracker.handle_rejuv_status(183.0, 101, 2, 2, 6);
    tracker.handle_rejuv_status(184.0, 102, 2, 2, 6);

    // Kill 2: last damage at 595, kill message at 600.
    tracker.handle_spawn(420.0);
    simulate_damage_and_kill(&mut tracker, 595.0, 600.0);
    // Grants for kill 2 (2-1 steal: team 3 wins majority, team 2 was killer).
    tracker.handle_rejuv_status(602.0, 200, 3, 3, 6);
    tracker.handle_rejuv_status(603.0, 201, 3, 3, 6);
    tracker.handle_rejuv_status(604.0, 202, 2, 3, 6);

    tracker.finalize();
    let data = tracker.get_output();

    assert_eq!(data.kill_events.len(), 2);

    let k1 = &data.kill_events[0];
    assert_eq!(k1.team_killed, 2);
    assert_eq!(k1.team_claimed, 2);
    assert_eq!(k1.rejuvs_by_team.get("2"), Some(&3));
    assert_eq!(k1.rejuvs_by_team.get("3"), Some(&0));

    let k2 = &data.kill_events[1];
    assert_eq!(k2.team_killed, 3);
    assert_eq!(k2.team_claimed, 3);
    assert_eq!(k2.rejuvs_by_team.get("2"), Some(&1));
    assert_eq!(k2.rejuvs_by_team.get("3"), Some(&2));
}

// =========================================================================
// get_output
// =========================================================================

#[test]
fn get_output_populates_post_match_without_team_claimed() {
    let mut tracker = MidBossTracker::new();
    tracker.handle_spawn(60.0);
    simulate_damage_and_kill(&mut tracker, 175.0, 180.0);
    tracker.handle_rejuv_status(176.0, 10, 2, 2, 6);
    tracker.handle_rejuv_status(176.5, 11, 2, 2, 6);
    tracker.handle_rejuv_status(177.0, 12, 2, 2, 6);
    tracker.finalize();

    let data = tracker.get_output();
    assert_eq!(data.post_match.len(), 1);
    let pm = &data.post_match[0];
    assert_eq!(pm.team_killed, 2);
    assert_eq!(pm.destroyed_time_s, 180);
    assert_eq!(pm.rejuvs_by_team.get("2"), Some(&3));
    assert_eq!(pm.rejuvs_by_team.get("3"), Some(&0));
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
    tracker.handle_kill(70.0, 0.0, 0.0, 0.0, 3);

    // Cycle 2: spawn, damage, kill
    tracker.handle_spawn(480.0);
    tracker.record_damage(15000, 490.0);
    tracker.handle_kill(500.0, 0.0, 0.0, 0.0, 2);

    tracker.finalize();
    let data = tracker.get_output();

    assert_eq!(data.fight_windows.len(), 2);
    assert_eq!(data.fight_windows[0].spawn_cycle, 1);
    assert_eq!(data.fight_windows[1].spawn_cycle, 2);
    assert_eq!(data.fight_windows[0].health_at_end, 0, "first kill closes window at 0");
    assert_eq!(data.fight_windows[1].health_at_end, 0, "second kill closes window at 0");
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

// Mid-boss spawns at the 10-minute mark in every match. The differentiator is
// whether it gets killed -- a match where it spawns but survives is the real
// empty-downstream path. finalize() must emit the spawn event and leave
// kill/rejuv/fight-window/post-match collections empty, with no open window.
#[test]
fn finalize_after_spawn_without_kill_emits_spawn_only_output() {
    let mut tracker = MidBossTracker::new();
    tracker.handle_spawn(600.0);
    tracker.finalize();

    assert_eq!(tracker.spawn_events.len(), 1);
    assert_eq!(tracker.spawn_events[0].spawn_cycle, 1);
    assert!((tracker.spawn_events[0].spawn_time_s - 600.0).abs() < 0.001);
    assert_eq!(tracker.kill_events.len(), 0);
    assert_eq!(tracker.rejuv_events.len(), 0);
    assert_eq!(tracker.fight_windows.len(), 0);
    assert_eq!(tracker.post_match.len(), 0);
    assert!(tracker.open_window.is_none());

    let data = tracker.get_output();
    assert_eq!(data.spawn_events.len(), 1);
    assert_eq!(data.kill_events.len(), 0);
    assert_eq!(data.rejuv_events.len(), 0);
    assert_eq!(data.fight_windows.len(), 0);
    assert_eq!(data.post_match.len(), 0);
}
