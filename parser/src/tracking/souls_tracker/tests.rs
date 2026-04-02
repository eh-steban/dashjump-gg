use super::*;

fn make_tracker() -> SoulsTracker {
    SoulsTracker::new()
}

// -------------------------------------------------------------------------
// Balance carry-forward: player with no update this second keeps last balance
// -------------------------------------------------------------------------
#[test]
fn test_balance_carry_forward() {
    let mut tracker = make_tracker();

    // Player slot 0 gets a balance at sec 0
    tracker.handle_pawn_update(1, 0, 1000);
    tracker.build_snapshot(0);

    // Sec 1: player 0 gets no update, player 1 appears
    tracker.handle_pawn_update(2, 1, 500);
    tracker.build_snapshot(1);

    let output = tracker.get_output();
    assert_eq!(output.timeline.len(), 2);

    // Sec 0: only player 0 was present
    let sec0 = &output.timeline[0];
    assert_eq!(sec0.balances.get(&0), Some(&1000));
    assert_eq!(sec0.balances.get(&1), None);

    // Sec 1: player 0 still has 1000 (carry-forward), player 1 has 500
    let sec1 = &output.timeline[1];
    assert_eq!(sec1.balances.get(&0), Some(&1000));
    assert_eq!(sec1.balances.get(&1), Some(&500));
}

// -------------------------------------------------------------------------
// Snapshot alignment: snapshot at second N contains all players seen so far
// -------------------------------------------------------------------------
#[test]
fn test_snapshot_contains_all_players() {
    let mut tracker = make_tracker();

    for slot in 0u32..12 {
        tracker.handle_pawn_update(slot as i32 + 1, slot, (slot as i32 + 1) * 100);
    }
    tracker.build_snapshot(0);

    let output = tracker.get_output();
    let sec0 = &output.timeline[0];
    assert_eq!(sec0.balances.len(), 12);

    for slot in 0u32..12 {
        assert_eq!(
            sec0.balances.get(&slot),
            Some(&((slot as i32 + 1) * 100)),
            "slot {} missing or wrong balance",
            slot
        );
    }
}

// -------------------------------------------------------------------------
// Kill event recorded correctly
// -------------------------------------------------------------------------
#[test]
fn test_kill_event_recorded() {
    let mut tracker = make_tracker();

    tracker.handle_hero_killed(5, 7, 120);
    tracker.handle_hero_killed(3, 2, 200);

    let output = tracker.get_output();
    assert_eq!(output.kill_events.len(), 2);

    assert_eq!(output.kill_events[0].scorer_entindex, 5);
    assert_eq!(output.kill_events[0].victim_entindex, 7);
    assert_eq!(output.kill_events[0].match_sec, 120);

    assert_eq!(output.kill_events[1].scorer_entindex, 3);
    assert_eq!(output.kill_events[1].victim_entindex, 2);
    assert_eq!(output.kill_events[1].match_sec, 200);
}

// -------------------------------------------------------------------------
// Empty match: get_output() returns empty timeline and events
// -------------------------------------------------------------------------
#[test]
fn test_empty_match() {
    let tracker = make_tracker();
    let output = tracker.get_output();
    assert!(output.timeline.is_empty());
    assert!(output.kill_events.is_empty());
}

// -------------------------------------------------------------------------
// Balance update replaces previous value for the same entity
// -------------------------------------------------------------------------
#[test]
fn test_balance_update_replaces_previous() {
    let mut tracker = make_tracker();

    tracker.handle_pawn_update(1, 0, 500);
    tracker.build_snapshot(0);

    tracker.handle_pawn_update(1, 0, 750);
    tracker.build_snapshot(1);

    let output = tracker.get_output();
    assert_eq!(output.timeline[0].balances.get(&0), Some(&500));
    assert_eq!(output.timeline[1].balances.get(&0), Some(&750));
}
