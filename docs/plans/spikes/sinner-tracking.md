# Spike: Sinner Sacrifice Tracking -- Data Source Validation

**Status:** Complete
**Timebox:** 1 day
**Replay used:** `68182475_4609034.dem`, cross-checked against `68175583_527726523.dem`
**Probe:** `parser/src/bin/sinner_probe.rs` (branch `feature/sinner-tracking`)

---

## Findings

### Finding 1: 12 sinners per match, not 2 -- hypothesis invalidated

**confirmed** -- The original plan assumed 2 sinners (one per team side). Reality: **12 `CNPC_Neutral_SinnersSacrifice` entities** spawn per match, all within ~2 seconds of each other at ~8 minutes (461s replay 1, 483s replay 2). They are distributed across 6 symmetrical position pairs:

| Pair | Position A (x, y, z) | Position B (x, y, z) |
|------|----------------------|----------------------|
| 1 | (-3682, -1524, 562) | (-3968, -1728, 564) |
| 2 | (3840, 1520, 536) | (4032, 1832, 536) |
| 3 | (2368, -960, 384) | (-2336, 928, 384) |
| 4 | (-2624, -960, 248) | (2624, 960, 248) |
| 5 | (5760, -1136, 256) | (-5728, 1088, 256) |
| 6 | (-6016, -1184, 256) | (5984, 1216, 256) |

All 12 have `max_health=500`. Positions are consistent across replays.

Sinners **respawn** -- entity indices are recycled (CREATE re-fires on the same index after death). Example: entity 3378 creates at 461.3s, again at 777.5s, again at 1101.9s (approximately every 5 minutes).

**Impact on plan:** `SinnerTracker` must track all 12 entities. The "2 entries per match" verification check is wrong and must be removed. Snapshot output will be a vec of snapshots across all waves.

---

### Finding 2: Entity DELETE never fires

**confirmed** (0 DELETE events across 40+ CREATE events in one replay, 54+ in another)

Same unreliable pattern documented for lane creeps. DELETE either fires much later (triggered by an unrelated game event) or not at all. Cannot be used as a death signal.

---

### Finding 3: Position is available at CREATE

**confirmed** -- 100% of CREATE events had position populated (40/40 replay 1, 54/54 replay 2). `get_entity_position()` works reliably at entity creation time.

---

### Finding 4: CCitadelUserMsg_BossKilled does NOT fire for sinners -- hypothesis invalidated

**confirmed** -- This is the critical finding. All 20 BossKilled events in replay 1 (23 in replay 2) were `sinner=false` after cross-referencing `entity_killed` ehandle against tracked sinner entity indices.

BossKilled fires for: walkers (`entity_killed_class=5`, indices ~2527-2532), mid-boss (`class=29`, indices ~299-304), patron (`class=8`), and other structures (`class=28, 30, 31`). Never for sinner indices (3335-3379).

**Impact on plan:** The `record_boss_killed` handler, `killer_entity_index`, `killer_player_slot`, `killer_team`, and `bosses_remaining` fields in `SinnerSnapshot` are all invalid -- BossKilled is not the kill signal for sinners. The entire killer-tracking approach needs a different data source.

---

### Finding 5: `should_track_snapshot()` is the right abstraction

**confirmed** -- Sinners are stationary at CREATE and never need per-second position tracking. A `should_track_snapshot()` method for entities needing one-time position capture (distinct from `should_track_position()` for per-second tracking) fits the use case cleanly. Removing sinners from `should_track_position()` is still required.

---

## Resolved: Death Detection + Kill Attribution

**confirmed** -- Both validated by extended probe on `68182475_4609034.dem` (`sinner_probe.rs` on `feature/sinner-tracking`).

### Health transition pattern

Sinner health decrements in **100-unit steps** (500 → 400 → 300 → 200 → 100 → 1). The correct death signal is `health == 1 && prev > 1` on an UPDATE event.

Sinners play a ~3-second death animation after the killing blow before the entity is recycled and the player receives the buff. The probe confirmed this sequence for entity 3377: `health == 1` at tick 56472 (match_s 863.5), `health == 0` at tick 56555 (match_s 864.8) -- 83 ticks (~1.38 seconds) later, mid-animation. Health=0 is unreliable as a detection signal: it appeared in only 1 of 34 deaths, likely because Deadlock's delta compression skips the `health=0` packet when the entity is recycled in the same delta window. `health == 1` fired for all 34 deaths.

**Death signal:** `health == 1 && prev > 1`
**Death time semantics:** tick of the killing blow, not tick of entity recycling (which is ~1-3 seconds later)

Probe result: 34 death events detected across a ~30-minute match (40 total CREATEs = 12 initial + 28 respawns; 34 deaths = 28 respawn-triggering deaths + 6 end-of-match deaths where the match ended before the sinner could respawn).

### Kill attribution

`CCitadelUserMessageDamage` (ID 300) fields:
- `entindex_victim: i32` -- direct entity index (no ehandle conversion needed)
- `entindex_attacker: i32` -- direct entity index of attacker

Attribution approach: track the last `(tick, attacker_entity_index)` per sinner entity index from all Damage messages. When `health == 1 && prev > 1` fires, the stored attacker is the killer. Resolve attacker entity index to player slot via the pawn → controller → `m_unLobbyPlayerSlot` chain.

Probe result: player slot resolved successfully for all 34 detected deaths.

---

## Revised SinnerSnapshot (removing invalidated fields)

```rust
#[derive(Debug, Serialize, Clone)]
pub struct SinnerSnapshot {
    pub entity_index: i32,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub spawn_time_s: u32,
    pub max_health: i32,
    pub death_time_s: Option<u32>,      // pending: health=0 on UPDATE
    pub time_alive_s: Option<u32>,      // derived: death_time_s - spawn_time_s
    // Killer fields deferred -- BossKilled does not fire for sinners
    // damage_dealt_by_attacker deferred -- needs separate probe
}
```

Fields removed from original plan: `killer_entity_index`, `killer_player_slot`, `killer_team`, `bosses_remaining`, `damage_dealt_by_attacker`.

---

## Implementation Plan (revised)

The implementation plan can proceed with the confirmed data sources. The unresolved death-detection question should be addressed in a follow-up probe before implementing `handle_sinner_delete`.

**Step 1:** Add `should_track_snapshot()` to `replay_parser.rs`, remove sinner from `should_track_position()`

**Step 2:** Update `private/specs/contracts/parser-api.md` with revised `SinnerSnapshot` (no killer fields)

**Step 3:** `parser/src/domain/sinner.rs` -- implement revised struct

**Step 4:** `parser/src/tracking/sinner_tracker/` -- implement with CREATE handling and health=0 death detection (or deferred death detection)

**Step 5:** `parser/src/replay_parser.rs` -- wire CREATE, UPDATE (for health=0), remove from positions

**Step 6:** `backend/app/domain/match_analysis.py` -- add `sinners` to ParsedMatchResponse
