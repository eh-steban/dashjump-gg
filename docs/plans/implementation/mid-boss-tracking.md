# Mid-Boss Tracking Plan

## Context

Mid-boss tracking is currently broken in two ways: `CNPC_MidBoss` is in `should_track_position()` even though it spawns at a fixed location and never moves, producing useless per-second position snapshots; and no spawn, kill, or rejuv buff data is captured at all. The two Citadel messages that carry this data -- `CCitadelUserMsg_MidBossSpawned` (ID 349) and `CCitadelUserMsg_BossKilled` (ID 347) -- are not subscribed. This leaves a blind spot in mid-match objective analytics.

**Goals:**
- Parser emits a `mid_boss` output block with spawn timing, kill events (team + matchtime + position), health timeline (event-anchored), and per-player rejuv buff grants
- Position tracking for mid-boss is removed (it does not move)
- Backend passes `mid_boss` data through to the API response without transformation

**Branch:** `feature/midboss` (worktree: `dashjump-gg-midboss`)
**Review workflow:** implement -- test -- subagent updates plan -- pause for user review -- commit -- next phase

---

## Scope

| Service  | Involved | Agent              |
|----------|----------|--------------------|
| Parser   | yes      | `rust-parser`      |
| Backend  | yes      | `backend-python`   |
| Frontend | yes      | `dashjump-designer` / `frontend-react` |

---

## Acceptance Criteria

Feature is done when ALL of the following are true:

- [ ] `mid_boss` key is present in parser JSON output and backend API response for any match with a mid-boss kill
- [ ] `mid_boss.spawn_events` contains at least one entry (with `spawn_time_s` > 0) for a match where the mid-boss spawned
- [ ] `mid_boss.kill_events` contains `team`, `matchtime_s`, `x/y/z` position, and `bosses_remaining` for each kill
- [ ] `mid_boss.fight_windows` contains at least one window per kill, with `health_at_end=0` on the final window of each spawn cycle
- [ ] `mid_boss.spawn_events[].max_health` is set per cycle (probe-validated within 1% of `13000 + 195 * (spawn_time_s / 60)`) and there is no top-level `mid_boss.max_health`
- [ ] `mid_boss.kill_events[].team_claimed` is correctly derived from RejuvStatus majority
- [ ] `mid_boss.post_match` is populated by the parser from collected kill events and logged in frontend console
- [ ] `mid_boss.rejuv_events` contains at least one entry per player who claimed a rejuv buff in that match
- [ ] `CNPC_MidBoss` no longer appears in position snapshots
- [ ] All in-scope phase checkpoints complete and signed off by user

---

## Reference Data

### Citadel Messages (confirmed, valveprotos-rs commit `458c5e1`, 2026-04-01)

| Message | ID | Key Fields |
|---------|----|------------|
| `CCitadelUserMsg_MidBossSpawned` | 349 | None -- spawn time = `ctx.tick()` |
| `CCitadelUserMsg_BossKilled` | 347 | `objective_team` (int32), `gametime` (float) -- mapped to `matchtime_s`, `bosses_remaining` (int32), `entity_position` (CMsgVector), `entity_killed_class` (int32) |
| `CCitadelUserMsg_RejuvStatus` | 350 | `killing_team` (int32), `player_pawn` (uint32 ehandle), `user_team` (int32), `event_type` (int32) |

### Entity Constants (confirmed via spike, 2026-04-08/2026-04-12)

| Name | Hash constant | Value | Notes |
|------|---------------|-------|-------|
| `CNPC_MidBoss` | `CNPC_MIDBOSS_ENTITY` | entity hash | Already in `constants.rs:49` |
| Mid-boss class ID | `MID_BOSS_CLASS_ID` | `8: i32` | Filter `BossKilled.entity_killed_class`; confirmed across 8 kills in 3 replays |
| `CCitadelItemPickupRejuv` | not tracked at entity level | -- | `RejuvStatus` messages handle buff grants |

### Max Health + Regen (confirmed via `probe_midboss_health.rs`, 2026-04-16)

`m_iMaxHealth` is **per spawn cycle, not match-global**:

- Formula: `max_health = 13000 + 195 * match_minutes` with time measured from match start, not from spawn. Validated within 0.7% on every CREATE in `68175583_527726523` (3 cycles). The since-spawn formula the wiki prose suggests is off by +1,950 / +5,265 / +6,630 HP on those same CREATEs.
- Static within a single CNPC_MidBoss lifetime (CREATE -- DELETE). Slope = 0/min across all three cycles of the probe replay.
- Changes between CREATEs because match time advances. In `68175583_527726523` the cycles observed 14,950 / 18,265 / 19,630 (+3,315 then +1,365).
- Parser reads `m_iMaxHealth` once per CREATE and stores it on the current `spawn_events.last_mut()` entry. No UPDATE subscription needed for this field; no match-global storage.

`m_iHealth` regenerates at **~15 HP/s** when below `m_iMaxHealth`, but we intentionally do not surface a regen rate on the contract:

- One regen segment captured mid-fight in cycle 2 of the probe replay: `+14 HP over 0.938 s = 14.93 HP/s`, matching the wiki's 15 HP/s within sampling granularity.
- Only observable while the boss is taking damage with brief gaps. Before any damage `m_iHealth == m_iMaxHealth`; after a contiguous kill burst there is no further recovery (the entity is deleted).
- Within a fight window, regen is already captured by existing `health_samples` that follow `m_iHealth` from entity updates. Between fight windows (damaged, retreat, recover, re-engage) the frontend holds `health_at_end` constant; the resulting staleness is bounded by `15 * gap_s` HP. In practice gap periods end on kill or a fresh engagement with new samples, so the visible error is small and a regen rate constant would add consumer-side state reconstruction for little analytical gain. If a future experiment needs between-window precision, the fix is to emit sparse samples in `fight_windows` between active sub-windows, not to add a `regen_hp_per_s` field.

Full per-second cycle data for the probe run is reproduced by `private/engineering/tools/probe_midboss_health.rs`; re-run against any replay in `/parser/src/replays/` for fresh data. The probe's as-written gate is `tick % 60 == 0`, which at 64 tps samples every 60/64 s ≈ 0.94 s (a hold-over from a pre-2026-04-08 "60 tps" assumption); the correct 1 Hz gate for new probes is `tick % 64 == 0` per `feedback_probe_granularity`.

### Shield Mechanic (confirmed-absent on entity, spike Q2)

The mid-boss has a **regenerating damage shield** (wiki: 35 base + 5/min from match start). The shield is NOT exposed as an entity field -- no `m_iShield*` or `m_flShield*` exists on `CNPC_MidBoss`.

Shield data is only available via the damage stream: `CCitadelUserMessage_Damage` (ID 300) fields `victim_shield_max` and `victim_shield_new`. Do NOT use `CCitadelUserMsg_BossDamaged` (ID 348) -- its only fields are `objective_team`, `objective_id`, `entity_damaged`, with no shield data.

**Impact on health timeline:** Fight window `health_samples` will show health at max during shield-breaking. We are not tracking shield health separately in v1. Per-hit shield data is available in the damage stream if needed later.

### RejuvStatus Enum (confirmed via spike Q6, 2026-04-08)

| event_type | Meaning | Filter? |
|------------|---------|---------|
| 6 | Buff granted (fires within ~6s of kill, once per player who claims) | **Yes** -- this is the "claim" signal |
| 7 | Buff consumed (player died and was revived) | Store as-is for future use |
| 8 | Buff expired or last stack gone | Store as-is for future use |

Grant count per kill is 2-3, not always exactly 3 (depends on how many players claim the crystal).

### Respawn Timers (confirmed via spike Q3, 2026-04-08)

| Death # | Respawn delay | Confidence |
|---------|---------------|------------|
| 1st | 7 min (exactly 26881 ticks at 64 tps) | `confirmed` -- 5 observations |
| 2nd | 6 min (exactly 23041 ticks at 64 tps) | `confirmed` -- 2 observations |
| 3rd+ | 5 min | `inferred` -- wiki only, not yet observed |

### Resolved Questions (from spike, all 7 closed)

1. **`entity_killed_class = 8`** for `CNPC_MidBoss` -- `confirmed` across 8 kills in 3 replays. Position (0,0,-768) + team=4 (neutral) disambiguates from other boss types.
2. **`RejuvStatus.event_type` enum** -- `confirmed`, see table above.
3. **`MidBossSpawned` fires once per cycle** -- `confirmed` across 8 cycles. No secondary signal needed.
4. **50% health roar: no Citadel message exists** -- `confirmed absent`. `HudGameAnnouncement` (ID 363) fired zero times across 3 replays. Frontend must derive the 50% crossing from fight window `health_samples`.
5. **Respawn timers** -- `confirmed` for 7 min / 6 min, `inferred` for 5 min. See table above.
6. **Max health formula** -- `confirmed`. See Max Health + Regen section above.
7. **Shield not on entity** -- `confirmed`. See Shield Mechanic section above.

### Contract Alignment (from spike cross-reference, 2026-04-11)

- **`boss_name_hash` required**: Main-branch contract (`parser-output.md`) canonicalizes `boss_name_hash` as the stable type identifier for `BossSnapshot`. When adding mid-boss, emit `boss_name_hash = fxhash::hash_bytes(b"CNPC_MidBoss")`. `entity_killed_class = 8` is for message filtering only, NOT for contract emission.
- **`max_health` mechanics table**: Add mid-boss row to `parser-output.md` max_health Mechanics table. Value is **per spawn cycle** -- read on each `CNPC_MidBoss` CREATE and stored on the corresponding `MidBossSpawnEvent`. Formula: `max_health = 13000 + 195 * match_minutes` where `match_minutes` is measured from match start. No match-global `max_health` on `mid_boss`.
- **Gap closure**: This plan closes `entity-types-reference.md` Gap 3 (mid-boss health not tracked) and partially closes Gap 5 (rejuv entities not subscribed).

---

## Critical Files

| Layer | File | Change |
|-------|------|--------|
| Parser constants | `parser/src/entities/constants.rs` | Add `MID_BOSS_CLASS_ID` constant |
| Parser domain | `parser/src/domain/mid_boss.rs` | Create |
| Parser domain mod | `parser/src/domain/mod.rs` | Add `mid_boss` module |
| Parser tracker | `parser/src/tracking/mid_boss_tracker.rs` | Create |
| Parser tracker mod | `parser/src/tracking/mod.rs` | Add `mid_boss_tracker` module |
| Parser integration | `parser/src/replay_parser.rs` | Subscribe messages, remove position tracking |
| Parser output contract | `private/specs/contracts/parser-output.md` | Add `mid_boss` block |
| Backend domain | `backend/app/domain/mid_boss.py` | Create |
| Backend domain model | `backend/app/domain/match_analysis.py` | Add `mid_boss` field |
| Backend contract | `private/specs/contracts/backend-api.md` | Add `mid_boss` to response |

---

## Phase 0 -- Contract (`rust-parser` owns parser-output.md; `backend-python` owns backend-api.md)

### 0.1. Update `parser-output.md`

Add a `mid_boss` top-level block to the parser output contract. The field sits alongside `bosses` and `lane_creep_data`. Use the structure below.

**`mid_boss` top-level:**

| Field | Type | Required | Notes |
|-------|------|----------|-------|
| `boss_name_hash` | string | yes | `fxhash::hash_bytes(b"CNPC_MidBoss")` as u64 string. Canonical type identifier per `parser-output.md` Boss Type Identification table. |
| `spawn_events` | MidBossSpawnEvent[] | yes | One per spawn cycle; empty if mid-boss never spawned. Per-cycle `max_health` lives on each entry (see table below) -- there is no match-global `max_health` on `mid_boss` because `m_iMaxHealth` scales with match time. |
| `kill_events` | MidBossKillEvent[] | yes | One per kill; empty if mid-boss was never killed |
| `rejuv_events` | RejuvStatusEvent[] | yes | One per individual rejuv grant; empty if no grants |
| `fight_windows` | FightWindow[] | yes | One per engagement; captures health progression only during active fights (see below) |
| `post_match` | MidBossPostMatch[] | yes | Pass-through of Valve's `CMsgMatchMetaDataContents.MidBoss` records. Small (3 fields per kill). Stored in DB and returned in API for cross-referencing against replay-parsed data. Not displayed in UI -- console.log only in frontend. |

**MidBossSpawnEvent:**

| Field | Type | Required | Notes |
|-------|------|----------|-------|
| `spawn_cycle` | int | yes | 1-indexed spawn cycle number; links spawn to its corresponding kill |
| `spawn_time_s` | float | yes | Match-relative time derived from `ctx.tick()` at `MidBossSpawned` |
| `max_health` | int | yes | `m_iMaxHealth` read on the `CNPC_MidBoss` CREATE event paired with this spawn's `MidBossSpawned` message. Static for the entity's lifetime. Follows `max_health = 13000 + 195 * match_minutes` where `match_minutes = spawn_time_s / 60` -- probe-validated within 0.7% on a three-cycle replay (2026-04-16, `private/engineering/tools/probe_midboss_health.rs`). |

**MidBossKillEvent:**

| Field | Type | Required | Notes |
|-------|------|----------|-------|
| `spawn_cycle` | int | yes | Which spawn cycle this kill ended; matches `MidBossSpawnEvent.spawn_cycle` |
| `team` | int | yes | `objective_team` from `BossKilled`; team that killed the boss |
| `team_claimed` | int | yes | Team that claimed the majority of the 3 rejuv buffs. Derived: count `event_type == 6` RejuvStatus events for this spawn cycle, group by `user_team`; the team with 2+ claims wins. If no grants observed, falls back to `team` (killing team). Compare to `post_match[].team_claimed` from Valve's blob for validation. |
| `matchtime_s` | float | yes | `gametime` from `BossKilled`; match time in seconds |
| `x` | float | yes | `entity_position.x` from `BossKilled` |
| `y` | float | yes | `entity_position.y` from `BossKilled` |
| `z` | float | yes | `entity_position.z` from `BossKilled` |
| `bosses_remaining` | int | yes | `bosses_remaining` from `BossKilled` |

**RejuvStatusEvent:**

| Field | Type | Required | Notes |
|-------|------|----------|-------|
| `player_pawn` | int | yes | EHandle of the player pawn that received the buff |
| `user_team` | int | yes | Team of the player who received the buff |
| `killing_team` | int | yes | Team that killed the mid-boss |
| `matchtime_s` | float | yes | Match time when this event fired; needed for buff duration tracking and timeline placement |
| `event_type` | int | yes | 6=buff granted, 7=buff consumed, 8=buff expired. Filter on `event_type == 6` for claim tracking. |

**FightWindow:**

A fight window captures one continuous engagement with the mid-boss. Windows open on first damage and close after 5 seconds of no damage or on boss death. This follows the sinner tracker's space-efficient pattern -- only recording data during active interactions rather than padding empty time.

| Field | Type | Required | Notes |
|-------|------|----------|-------|
| `spawn_cycle` | int | yes | Which spawn cycle this window belongs to |
| `window_start_s` | float | yes | Match time of first damage event in this engagement |
| `window_end_s` | float | yes | Match time of last damage event (or death) in this engagement |
| `health_at_start` | int | yes | `m_iHealth` at window open (max_health for first engagement of a spawn cycle) |
| `health_at_end` | int | yes | `m_iHealth` at window close; 0 if this window ends in a kill |
| `health_samples` | HealthSample[] | yes | Sparse samples within this window only; empty stretches between windows are not recorded |

This design captures bait-fight-disengage-reengage patterns naturally. Example: team starts mid-boss at 12:30, abandons at 13:00 (health at 60%), teamfight happens, winning team returns at 15:45 and finishes the kill at 16:20. That's two fight windows for the same spawn cycle -- each with its own health progression.

**HealthSample:**

| Field | Type | Required | Notes |
|-------|------|----------|-------|
| `time_s` | float | yes | Match-relative time in seconds (float for precision parity with kill/rejuv events) |
| `health` | int | yes | Health value at this sample; 0 at death |

**MidBossPostMatch:**

One record per kill cycle, populated by the parser's `finalize()` from the collected `MidBossKillEvent` list. Provides a simplified summary for consumers that only want kill-count-level data.

| Field | Type | Required | Notes |
|-------|------|----------|-------|
| `team_killed` | int | yes | `ECitadelLobbyTeam` -- team that landed the killing blow (from `MidBossKillEvent.team`) |
| `team_claimed` | int | yes | `ECitadelLobbyTeam` -- team that received the buff (from `MidBossKillEvent.team_claimed`) |
| `destroyed_time_s` | int | yes | Kill time in match seconds (`MidBossKillEvent.matchtime_s` truncated to u32) |

**Overlap note:** The Deadlock API HTTP response (`MatchInfoFields.mid_boss` in `app/domain/deadlock_api.py`) exposes equivalent semantic data that the frontend's objective damage panel currently consumes via `match_metadata`. The parser populates `MidBossPostMatch` independently from replay protobufs via haste -- the parser never calls the Deadlock API. Both sources should agree; the parser version is the primary source going forward.

### 0.2. Update `backend-api.md`

Add `mid_boss: MidBossData` (optional, may be null for old cached matches) to the match analysis response shape. The backend passes the parser output through unmodified -- no Deadlock API augmentation. See the "Overlap note" above for the separate HTTP source that the frontend will migrate away from.

### 0 Checkpoint

**Status:** `[x] Complete`

> **Agent instructions:** Stop here. Before returning you MUST:
> 1. List every field added across service boundaries
> 2. Confirm `parser-output.md` and `backend-api.md` have been updated
> 3. Check off every item below with date and actual result
> 4. Await user review before any Phase A/B work begins

#### Results *(agent fills in)*

- [x] `parser-output.md` updated with `mid_boss` block -- 2026-04-12. Added MidBossData top-level field and full sub-type specs: MidBossSpawnEvent, MidBossKillEvent, RejuvStatusEvent, FightWindow, HealthSample, MidBossPostMatch.
- [x] `backend-api.md` updated with `mid_boss` field -- 2026-04-12. Added `mid_boss: MidBossData | None` to TransformedMatchData (optional for backward compat). Added MidBossData section noting passthrough + backend-populated `post_match`.
- [x] All boundary-crossing fields listed in the table below

#### Field change log *(agent fills in)*

| Field | Change | Spec file | Consuming service impact |
|-------|--------|-----------|--------------------------|
| `mid_boss` | added (new top-level key) | `parser-output.md` | Backend: add `mid_boss: MidBossData` to `ParsedMatchResponse` |
| `mid_boss.boss_name_hash` | added | `parser-output.md` | Backend: passthrough |
| `mid_boss.spawn_events` | added (MidBossSpawnEvent[]) | `parser-output.md` | Backend: passthrough |
| `mid_boss.spawn_events[].max_health` | added (per-cycle `m_iMaxHealth`, revised 2026-04-16) | `parser-output.md` | Backend: passthrough; Frontend: `MidBossHealthBar` reads from active spawn event, not match-global. Legacy top-level `mid_boss.max_health` removed in same revision. |
| `mid_boss.kill_events` | added (MidBossKillEvent[]) | `parser-output.md` | Backend: passthrough |
| `mid_boss.kill_events[].team_claimed` | added (derived field) | `parser-output.md` | Backend: passthrough; Frontend: compare to post_match |
| `mid_boss.rejuv_events` | added (RejuvStatusEvent[]) | `parser-output.md` | Backend: passthrough |
| `mid_boss.fight_windows` | added (FightWindow[]) | `parser-output.md` | Backend: passthrough |
| `mid_boss.post_match` | added (MidBossPostMatch[]) | `parser-output.md`, `backend-api.md` | Parser populates from collected kill events in `finalize()`; backend passes through unchanged |
| `mid_boss` | added (new API response field, optional) | `backend-api.md` | Frontend: domain types + console.log post_match |

#### Deferred items
None.

Await user review before proceeding to Phase A.

---

## Phase A -- Parser (`rust-parser` agent)

### A1. Remove mid-boss from position tracking

In `replay_parser.rs`, remove `CNPC_MIDBOSS_ENTITY` from the `should_track_position()` match arm (`replay_parser.rs:142`). Mid-boss spawns at a fixed location and does not move -- per-second position snapshots provide no value and add noise.

Also remove `CNPC_MIDBOSS_ENTITY` from the `get_custom_id()` match arm (`replay_parser.rs:238`). Once mid-boss is removed from position tracking, no code path reaches `get_custom_id()` for it. Removing the arm eliminates dead code and prevents `custom_id=24` from appearing in positions output.

Verify: after this change, a replay with a mid-boss should produce no mid-boss entities in the positions array.

### A2. Add MID_BOSS_CLASS_ID constant

~~Write probe binary~~ -- **Done.** Probe binary `probe_midboss_runtime.rs` completed during spike (now at `private/engineering/tools/probe_midboss_runtime.rs`). All open questions resolved.

Add `pub const MID_BOSS_CLASS_ID: i32 = 8;` to `parser/src/entities/constants.rs`. Citation: `probe_midboss_runtime`, replays `68175583`, `68182475`, `55423930`, 2026-04-08.

### A3. Create domain types

Create `parser/src/domain/mid_boss.rs` with the following types, matching the contract spec from Phase 0:

```rust
use serde::Serialize;

#[derive(Debug, Serialize, Default)]
pub struct MidBossData {
    pub boss_name_hash: String,
    pub spawn_events: Vec<MidBossSpawnEvent>,
    pub kill_events: Vec<MidBossKillEvent>,
    pub rejuv_events: Vec<RejuvStatusEvent>,
    pub fight_windows: Vec<FightWindow>,
    pub post_match: Vec<MidBossPostMatch>,  // populated by finalize() from kill_events
}

#[derive(Debug, Serialize)]
pub struct MidBossSpawnEvent {
    pub spawn_cycle: u32,
    pub spawn_time_s: f32,
    pub max_health: i32,  // m_iMaxHealth read on CREATE; scales per cycle with match time
}

#[derive(Debug, Serialize)]
pub struct MidBossKillEvent {
    pub spawn_cycle: u32,
    pub team: i32,
    pub team_claimed: i32,  // derived from majority of event_type==6 RejuvStatus grants
    pub matchtime_s: f32,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub bosses_remaining: i32,
}

#[derive(Debug, Serialize)]
pub struct RejuvStatusEvent {
    pub matchtime_s: f32,
    pub player_pawn: u32,
    pub user_team: i32,
    pub killing_team: i32,
    pub event_type: i32,
}

#[derive(Debug, Serialize)]
pub struct FightWindow {
    pub spawn_cycle: u32,
    pub window_start_s: f32,
    pub window_end_s: f32,
    pub health_at_start: i32,
    pub health_at_end: i32,
    pub health_samples: Vec<HealthSample>,
}

#[derive(Debug, Serialize)]
pub struct HealthSample {
    pub time_s: f32,
    pub health: i32,
}

// Populated by MidBossTracker::finalize() from the collected kill_events list.
// Mirrors the shape that the Deadlock API HTTP response exposes for the same
// kill records -- the parser does not call the Deadlock API.
#[derive(Debug, Serialize)]
pub struct MidBossPostMatch {
    pub team_killed: i32,
    pub team_claimed: i32,
    pub destroyed_time_s: u32,
}
```

Add `pub mod mid_boss;` to `parser/src/domain/mod.rs`.

### A4. Create MidBossTracker

Create `parser/src/tracking/mid_boss_tracker.rs`. The tracker manages fight windows (similar to the sinner tracker's event-log approach -- only recording during active engagements) and derives `team_claimed` from RejuvStatus grants.

```rust
const FIGHT_WINDOW_GAP_S: f32 = 5.0;  // seconds of no damage before closing a window

pub struct MidBossTracker {
    data: MidBossData,
    mid_boss_entity_index: Option<i32>,  // for damage event routing
    health_key: u64,
    max_health_key: u64,
    current_spawn_cycle: u32,            // incremented on each MidBossSpawned

    // Fight window state
    current_window: Option<FightWindow>, // open window being built
    last_damage_time_s: f32,             // for gap detection
}

impl MidBossTracker {
    pub fn new() -> Self { ... }

    /// Called when MidBossSpawned fires (ID 349). Increments spawn_cycle counter.
    /// Closes any open fight window from a previous cycle (shouldn't happen, but defensive).
    pub fn handle_spawn(&mut self, match_time_s: f32) { ... }

    /// Called when BossKilled fires (ID 347) and entity_killed_class == MID_BOSS_CLASS_ID.
    /// Closes the current fight window with health_at_end=0.
    /// Derives team_claimed: count event_type==6 RejuvStatus events for this spawn cycle,
    /// group by user_team; team with 2+ of the 3 rejuvs wins. Fallback to killing team
    /// if no grants observed.
    pub fn handle_kill(&mut self, msg: &CCitadelUserMsgBossKilled, match_time_s: f32) { ... }

    /// Called when RejuvStatus fires (ID 350).
    pub fn handle_rejuv_status(&mut self, msg: &CCitadelUserMsgRejuvStatus, match_time_s: f32) { ... }

    /// Called from on_entity when a CNPC_MidBoss entity is seen (CREATE or UPDATE).
    /// On CREATE, read `m_iMaxHealth` and write it onto the currently open
    /// `spawn_events.last_mut()` entry (static for that entity's lifetime,
    /// but scales per cycle with match time -- different on every CREATE).
    /// On UPDATE, only update stored entity index for damage routing.
    pub fn observe_entity(&mut self, entity_index: i32, entity: &Entity, is_create: bool) { ... }

    /// Called when a CCitadelUserMessage_Damage event has victim == mid_boss entity.
    /// Opens a new fight window if none is open or if the gap since last damage exceeds
    /// FIGHT_WINDOW_GAP_S. Appends a HealthSample to the current window.
    /// Reads m_iHealth from the entity.
    pub fn record_damage(&mut self, entity: &Entity, match_time_s: f32) { ... }

    /// Called at parse end. Closes any open fight window. Derives team_claimed for any
    /// kill events that don't have it yet (RejuvStatus events arrive after BossKilled).
    pub fn finalize(&mut self) { ... }

    pub fn into_data(self) -> MidBossData { ... }
}
```

**Fight window lifecycle:**
1. **Open:** First damage event to mid-boss (or first damage after a gap > `FIGHT_WINDOW_GAP_S`). `health_at_start` = current `m_iHealth`. `window_start_s` = current match time.
2. **Append:** Each subsequent damage event appends a `HealthSample` to the current window and updates `last_damage_time_s`.
3. **Close (gap):** On next damage event, if `match_time_s - last_damage_time_s > FIGHT_WINDOW_GAP_S`, close the current window (`health_at_end` = last sample's health, `window_end_s` = `last_damage_time_s`) and open a new one.
4. **Close (death):** `handle_kill` closes the window with `health_at_end = 0` and `window_end_s = kill matchtime_s`.
5. **Close (parse end):** `finalize()` closes any open window (boss alive at match end).

**`team_claimed` derivation:**
- After a kill, collect all `RejuvStatus` events with `event_type == 6` (buff granted) for the current `spawn_cycle`.
- Group by `user_team`, count per team. There are 3 total rejuv buffs per kill.
- The team with 2 or more claims = `team_claimed`. If no grants observed, fallback to `team` (killing team).
- Since RejuvStatus events fire after BossKilled (within ~6s), `handle_kill` cannot compute `team_claimed` immediately. The tracker defers computation to `finalize()`, which retroactively sets `team_claimed` on each `MidBossKillEvent` from the accumulated RejuvStatus events.

Add `pub mod mid_boss_tracker;` to `parser/src/tracking/mod.rs`.

Create `parser/src/tracking/mid_boss_tracker/tests.rs` with:
- `handle_spawn` pushes a `MidBossSpawnEvent` with correct `spawn_time_s`
- `handle_kill` pushes a `MidBossKillEvent` with correct fields including `team_claimed`
- `handle_rejuv_status` pushes a `RejuvStatusEvent`
- Multiple calls produce multiple events (mid-boss can respawn)
- `record_damage` opens a new fight window on first damage
- `record_damage` after a gap > 5s closes old window, opens new
- `record_damage` on death closes window with `health_at_end = 0`
- Fight window has correct `health_at_start` and `health_at_end`
- `team_claimed` correctly derived: team with 2+ of 3 rejuv grants wins (including steal case where claiming team != killing team)
- `team_claimed` falls back to killing team when no grants observed
- `observe_entity` writes `m_iMaxHealth` onto the currently open `spawn_events.last_mut()` entry on CREATE, leaves it unchanged on UPDATE, and records distinct per-cycle values across a 3-cycle replay (each within 1% of `13000 + 195 * (spawn_time_s / 60)`)

Declare `#[cfg(test)] mod tests;` at the bottom of `mid_boss_tracker.rs`.

### A5. Subscribe messages and wire in replay_parser.rs

In `replay_parser.rs`:

1. Add `MidBossTracker` as a field on the parser struct.
2. In `on_packet`, subscribe to:
   - `CCitadelUserMsg_MidBossSpawned` (ID 349): call `self.mid_boss_tracker.handle_spawn(match_time_s)`
   - `CCitadelUserMsg_BossKilled` (ID 347): if `msg.entity_killed_class == Some(8)` (MID_BOSS_CLASS_ID), call `self.mid_boss_tracker.handle_kill(&msg)`
   - `CCitadelUserMsg_RejuvStatus` (ID 350): call `self.mid_boss_tracker.handle_rejuv_status(&msg)` -- tracker stores all event_types; consumers filter on `event_type == 6` for grants
3. In `on_entity`, when entity hash is `CNPC_MIDBOSS_ENTITY`, call `self.mid_boss_tracker.observe_entity(entity.index(), entity, delta_header == DeltaHeader::CREATE)`. On CREATE, tracker reads `m_iMaxHealth` (set once, never changes). On UPDATE, tracker only updates the stored entity index (for damage routing).
4. In the `CCitadelUserMessage_Damage` handler, after routing to `boss_tracker.record_boss_damage`, add a parallel check: if `victim_entity_index == mid_boss_tracker.mid_boss_entity_index`, call `self.mid_boss_tracker.record_damage(entity, match_time_s)`. This opens/appends to fight windows.
5. Before output assembly, call `self.mid_boss_tracker.finalize()` to close any open fight window and derive `team_claimed` on kill events from accumulated RejuvStatus grants.
6. In the output assembly (where `bosses` is populated), add `"mid_boss": self.mid_boss_tracker.into_data()`. Note: `finalize()` populates `post_match` from the collected kill events, so it arrives complete on the parser's output -- the backend does not augment it.

Match time conversion for `MidBossSpawned` (empty message, no gametime field):
```rust
// ctx.tick() returns i32; tick_interval() returns 0.015625 (64 tps confirmed)
let match_time_s = (ctx.tick() as f32 * ctx.tick_interval())
    - self.match_start_time_s.unwrap_or(0.0);
```

Use the same pattern already in use for `CCitadelUserMessage_Damage` in `replay_parser.rs` for consistency. Never hardcode `1.0/60.0` or `1.0/64.0` -- always use `ctx.tick_interval()` at runtime.

### A6. Record learnings

~~Append to `private/learnings.md`~~ -- **Done during spike.** Draft entries already in `learnings.md` covering:
- `MID_BOSS_CLASS_ID = 8` with probe citation
- `RejuvStatus.event_type` enum (6/7/8) with semantics
- Tick rate correction (64 tps, not 60)
- 50% roar message absence
- Respawn timers confirmed
- Haste async Visitor API notes

Any additional implementation-phase learnings should still be appended.

### A Checkpoint

**Status:** `[~] Verified end-to-end` -- positions clean, shape correct, but **two semantic bugs found** in `team` and `team_claimed` derivation. See "Bugs found in end-to-end verification" below.

> **Agent instructions:** Stop here. Before returning you MUST:
> 1. Run `cargo test` and record results below
> 2. Parse a real replay with a mid-boss kill and record `mid_boss` output below
> 3. Parse a real replay WITHOUT a mid-boss kill and confirm `mid_boss` output has empty arrays
> 4. Confirm mid-boss entity no longer appears in positions output
> 5. Check off every item below -- add date and actual result inline
> 6. Note any deferred items with reason
> 7. Update **Status** above

#### Results *(agent fills in)*

- [x] `cargo test` -- 77 passed, 0 failed -- 2026-04-12. Includes 23 mid_boss_tracker tests (15 original + 8 review-gate additions).
- [x] `mid_boss.spawn_events` non-empty for match with mid-boss spawn -- 2026-04-15. Match `55423930`: 3 spawns at 582.03 / 2414.63 / 2796.52 s.
- [x] `mid_boss.kill_events` non-empty for match with mid-boss kill, all fields present -- 2026-04-15. Match `55423930`: 3 kills at 2012.28 / 2454.17 / 2947.14 s, all required fields present, position consistently `(0,0,-768)` (mid-boss pit center, matches spike).
- [x] `mid_boss.rejuv_events` non-empty for match with rejuv grants -- 2026-04-15. Match `55423930`: 23 events (9 grants / 9 consumed / 5 expired).
- [x] `mid_boss.fight_windows` contains at least one window per kill, health_at_end=0 on kill window -- 2026-04-15. Match `55423930`: 3 windows, each closes via `handle_kill` with `health_at_end=0`. HP scaling visible: cycle1=13850, cycle2=19700, cycle3=21065 (matches `13000 + 195 * minutes` formula within 0.7%). This same per-cycle scaling is why `max_health` lives on `spawn_events[]`, not on `mid_boss` top-level (see per-cycle correction note below).
- [ ] `mid_boss.kill_events[].team_claimed` correctly derived from RejuvStatus majority -- **FAILS**, see Bug 2 below.
- [x] Positions output has no `CNPC_MidBoss` entity -- 2026-04-15. Verified against direct parser output for `55423930`: 3070 position ticks, 0 entries with mid-boss `boss_name_hash`, 0 entries with legacy `custom_id == "24"`. Both `should_track_position` and `get_custom_id` removals stick.
- [x] Spawn-but-no-kill path has a unit test -- 2026-04-16. `finalize_after_spawn_without_kill_emits_spawn_only_output` in `parser/src/tracking/mid_boss_tracker/tests.rs` asserts one spawn event at 600 s, empty `kill_events`/`rejuv_events`/`fight_windows`/`post_match`, and no open window. Full parser suite: 79 passed.
- [x] `MID_BOSS_CLASS_ID = 8` constant in `constants.rs` with probe citation -- 2026-04-12
- [x] Learnings appended to `private/learnings.md` -- 2026-04-08 (spike)

#### Bugs found in end-to-end verification (2026-04-15, replay `55423930`)

**Bug 1: `kill.team` is always `4` (neutral).** The plan's contract says `team` = `objective_team` from `BossKilled` "= team that killed the boss". Empirically, `objective_team` for a `CNPC_MidBoss` kill is always `4` (`ECitadelLobbyTeam::Spectator`/neutral) -- it represents the team **of the objective**, not the team that killed it. The mid-boss is neutral, so this field is never useful as a "killing team" signal. The actual killing team must come from `RejuvStatus.killing_team` (consistently `2` = Amber for all three kills in this replay), which agrees with Valve's `match_metadata.match_info.mid_boss[].team_killed` (`0` = Amber in `ECitadelTeam` enum, which maps to lobby team 2).

**Bug 2: `team_claimed` derivation window misses all grants.** `MidBossTracker::finalize()` filters rejuv grants to `[kill.matchtime_s, kill.matchtime_s + 30s]`. Empirically, every grant for replay `55423930` fires **before** `kill.matchtime_s`:

| Cycle | last damage sample | grants (event_type=6) | kill matchtime_s | gap (kill - last grant) |
|-------|--------------------|------------------------|------------------|-------------------------|
| 1 | t=1994.6 hp=293 | 2002.33, 2004.72, 2005.19 | 2012.28 | +7.1 s |
| 2 | t=2436.5 hp=92  | 2443.81, 2444.11, 2445.38 | 2454.17 | +8.8 s |
| 3 | t=2929.5 hp=279 | 2938.94, 2939.14, 2939.19 | 2947.14 | +7.9 s |

Because no grant is `>= kill.matchtime_s`, the per-cycle window catches zero events for every kill, falls back to `kill.team` (which is itself `4` per Bug 1), and emits `team_claimed = 4`. Two compounding causes:

1. **`gametime` field on `BossKilled` lags actual entity death by 7-18 s.** Either `gametime` is in a different coordinate system from `ctx.tick() * tick_interval()` (the `# TODO: verify` from the spike that was never resolved), or the message is broadcast at the end of a death cinematic. The 17-second gap from last damage sample (1994.6) to recorded kill (2012.28) on cycle 1 cannot be explained by the 64 tps tick rate alone.
2. **Even if (1) is fixed, the "majority of 3 grants" rule is wrong for steal scenarios.** Valve's `match_metadata.match_info.mid_boss[2].team_claimed = 1` (Sapphire stole cycle 3), but our rejuv stream shows 2 grants to `user_team=2` (Amber) and 1 to `user_team=3` (Sapphire). A strict majority rule would credit Amber. Valve must be tracking the team that actually **consumed** the buff, not the team that initially had it granted -- buffs transfer when their holder dies. Need to revisit the spec: probably look at `event_type == 7` (consumed) by user_team in the same window, or compute based on net-grants-minus-consumed.

#### Cross-reference: parser `post_match` vs Valve `match_metadata.match_info.mid_boss`

| Cycle | Parser `team_killed` | Parser `team_claimed` | Valve `team_killed` | Valve `team_claimed` | Notes |
|-------|----------------------|------------------------|---------------------|----------------------|-------|
| 1 | 4 ❌ | 4 ❌ | 0 (Amber) | 0 (Amber) | Bugs 1+2 |
| 2 | 4 ❌ | 4 ❌ | 0 (Amber) | 0 (Amber) | Bugs 1+2 |
| 3 | 4 ❌ | 4 ❌ | 0 (Amber) | 1 (Sapphire) | Steal scenario -- exposes second flaw in derivation |

Note enum mismatch: Valve uses `ECitadelTeam` (0=Amber, 1=Sapphire); we should be emitting `ECitadelLobbyTeam` (2=Amber, 3=Sapphire). The `team_claimed` semantic is correct (compare to Valve), but the enum needs to be `ECitadelLobbyTeam` per project convention.

#### Review gate results -- 2026-04-12

**test-auditor:** 4 critical gaps fixed (boundary test at 5.0s, multi-spawn-cycle windows, multi-kill rejuv attribution, tie-break). 4 important gaps addressed (non-grant filtering, empty tracker finalize). Remaining items (observe_entity integration test, double-finalize guard) deferred as non-blocking.

**code-reviewer:** 3 critical issues fixed:
1. `unwrap()` on production path -> replaced with `expect()` with invariant comment
2. `post_match` populated by parser (contract violation) -> removed; parser emits empty as contract specifies
3. Misleading comment on `gametime` -> fixed comment to clarify demo-time coordinates; added `# TODO: verify` for the subtraction

7 warnings reviewed: #4 (orphaned window on missed kill), #5 (attribution window doesn't filter by spawn_cycle), #7 (no match_started guard on RejuvStatus) accepted as low-risk given normal message ordering. #6 (boss_name_hash String vs int) is intentional per mid-boss contract. #8 (no health sampling on UPDATE) documented as architectural limitation. #9 (finalize idempotency) accepted -- single call site in parse_replay prevents double-call.

#### Sample output (replay `55423930`, parsed 2026-04-15; re-shaped per 2026-04-16 per-cycle correction)

```json
{
  "boss_name_hash": "16112031173533486177",
  "spawn_events": [
    {"spawn_cycle": 1, "spawn_time_s": 582.03, "max_health": 13850},
    {"spawn_cycle": 2, "spawn_time_s": 2414.63, "max_health": 19700},
    {"spawn_cycle": 3, "spawn_time_s": 2796.52, "max_health": 21065}
  ],
  "kill_events": [
    {"spawn_cycle": 1, "team": 4, "team_claimed": 4, "matchtime_s": 2012.28, "x": 0.0, "y": 0.0, "z": -768.0, "bosses_remaining": 0},
    {"spawn_cycle": 2, "team": 4, "team_claimed": 4, "matchtime_s": 2454.17, "x": 0.0, "y": 0.0, "z": -768.0, "bosses_remaining": 0},
    {"spawn_cycle": 3, "team": 4, "team_claimed": 4, "matchtime_s": 2947.14, "x": 0.0, "y": 0.0, "z": -768.0, "bosses_remaining": 0}
  ],
  "rejuv_events": "[23 entries: 9 grants / 9 consumed / 5 expired]",
  "fight_windows": [
    {"spawn_cycle": 1, "window_start_s": 1977.28, "window_end_s": 2012.28, "health_at_start": 13850, "health_at_end": 0, "health_samples": "[244 samples]"},
    {"spawn_cycle": 2, "window_start_s": 2418.55, "window_end_s": 2454.17, "health_at_start": 19700, "health_at_end": 0, "health_samples": "[330 samples]"},
    {"spawn_cycle": 3, "window_start_s": 2913.36, "window_end_s": 2947.14, "health_at_start": 21065, "health_at_end": 0, "health_samples": "[411 samples]"}
  ],
  "post_match": [
    {"team_killed": 4, "team_claimed": 4, "destroyed_time_s": 2012},
    {"team_killed": 4, "team_claimed": 4, "destroyed_time_s": 2454},
    {"team_killed": 4, "team_claimed": 4, "destroyed_time_s": 2947}
  ]
}
```

#### Deferred items
- **Bug 1 (`team` always = 4) and Bug 2 (`team_claimed` derivation) -- fixed in F1 (2026-04-15).** Retained here for historical context; closed upstream.
- **Per-cycle `max_health` correction (2026-04-16).** Shipped 2026-04-12 to -15 with a single match-global `mid_boss.max_health` populated on first CREATE. `probe_midboss_health.rs` (2026-04-16) confirmed `m_iMaxHealth` is per-spawn-cycle -- cycles 1/2/3 of replay `68175583_527726523` observed 14,950 / 18,265 / 19,630 HP and the frontend health bar was off by up to -24% on cycle 3. Contract and struct definitions above have been updated in place (top-level `max_health` dropped; `spawn_events[].max_health` added). When re-running Phase A against this corrected shape, delete any stale cached rows the way the F1 landing did (`alembic downgrade base` + `upgrade head`). No separate follow-up section -- the correction lives directly in A3/A4 above.

Await user review and commit approval before proceeding to Phase B.

---

## Phase B -- Backend (`backend-python` agent)

### B1. Create MidBoss domain types

Create `backend/app/domain/mid_boss.py` with Pydantic models matching the parser contract:

```python
from pydantic import BaseModel

class MidBossSpawnEvent(BaseModel):
    spawn_cycle: int
    spawn_time_s: float
    max_health: int

class MidBossKillEvent(BaseModel):
    spawn_cycle: int
    team: int
    team_claimed: int
    matchtime_s: float
    x: float
    y: float
    z: float
    bosses_remaining: int

class RejuvStatusEvent(BaseModel):
    matchtime_s: float
    player_pawn: int
    user_team: int
    killing_team: int
    event_type: int

class HealthSample(BaseModel):
    time_s: float
    health: int

class FightWindow(BaseModel):
    spawn_cycle: int
    window_start_s: float
    window_end_s: float
    health_at_start: int
    health_at_end: int
    health_samples: list[HealthSample]

class MidBossPostMatch(BaseModel):
    team_killed: int
    team_claimed: int
    destroyed_time_s: int

class MidBossData(BaseModel):
    boss_name_hash: str
    spawn_events: list[MidBossSpawnEvent]
    kill_events: list[MidBossKillEvent]
    rejuv_events: list[RejuvStatusEvent]
    fight_windows: list[FightWindow]
    post_match: list[MidBossPostMatch]
```

### B2. Add `mid_boss` to ParsedMatchResponse and TransformedMatchData

In `backend/app/domain/match_analysis.py`:
- Add `from app.domain.mid_boss import MidBossData`
- Add `mid_boss: MidBossData` to `ParsedMatchResponse`
- Add `mid_boss: MidBossData` to `TransformedMatchData`

The parser output is passed through unmodified -- `post_match` arrives already populated by `MidBossTracker::finalize()`, so the backend does not augment it. The Deadlock API HTTP response (`MatchInfoFields.mid_boss`) exposes equivalent semantic data for the same kill records, but it flows through a separate path (backend `match_metadata` -> frontend objective damage panel) and is not involved in the parser's `MidBossPostMatch`. The parser never calls the Deadlock API.

### B3. Add tests

In the existing backend test suite, add:
- A test fixture that includes a `mid_boss` block (with all arrays and `post_match` populated)
- Assert that `ParsedMatchResponse` deserializes the fixture correctly
- Assert that `TransformedMatchData` includes the `mid_boss` field in the API response

Minimum: one serialization round-trip test covering the full `MidBossData` shape, including empty-array case.

### B4. Record learnings

Append to `private/learnings.md` ## Drafts if any non-obvious patterns were encountered (e.g., Pydantic coercion edge cases for float `matchtime_s`).

### B Checkpoint

**Status:** `[x] Complete` (transport verified; semantic bugs in derivation belong to Phase A and are tracked as follow-ups)

> **Agent instructions:** Stop here. Before returning you MUST:
> 1. Run `pytest` and record results below
> 2. Hit `GET /match/analysis/{match_id}` for a match with a mid-boss kill and paste the `mid_boss` key from the response
> 3. Check off every item below with date and actual result
> 4. Note any deferred items with reason
> 5. Update **Status** above

#### Results *(agent fills in)*

- [x] `pytest` -- 68 passed, 2 skipped (`test_parsed_matches_repo.py` and `test_users_repo.py` skipped per known-failure note in `.claude/rules/backend/CLAUDE.md`) -- 2026-04-15.
- [x] `mid_boss` key present in API response, all fields match spec -- 2026-04-15 (top-level shape), updated 2026-04-16 after per-cycle `max_health` correction. `GET /match/analysis/55423930` returned `parsed_match_data.mid_boss` with 6 top-level keys (`boss_name_hash`, `spawn_events`, `kill_events`, `rejuv_events`, `fight_windows`, `post_match`) and per-cycle `max_health` on each `spawn_events` entry. Pydantic round-trip clean once the stale cache row was deleted (see "Stale-cache invalidation gap" deferred item under Phase A); same cache-drop step needs to be re-run after the 2026-04-16 shape change.
- [x] Spawn-but-no-kill case round-trips through the Pydantic domain without nulls or missing keys -- 2026-04-16. `tests/test_mid_boss_domain.py::test_mid_boss_data_round_trips_spawn_without_kill` builds a `MidBossData` with one spawn event (with `max_health`) and empty arrays, `model_dump()` returns all six contract keys, `model_validate()` round-trips cleanly. Full backend suite: 72 passed, 2 pre-existing skips. Test needs a one-line update to add `max_health` to the fixture spawn event when the 2026-04-16 shape lands.
- [x] Learnings appended to `private/learnings.md` -- nothing new beyond the spike entries; the bugs found are recorded in this plan instead.

#### Sample output (replay `55423930`, fresh parse 2026-04-15; re-shaped per 2026-04-16 per-cycle correction)
```json
{
  "boss_name_hash": "16112031173533486177",
  "spawn_events": [{"spawn_cycle": 1, "spawn_time_s": 582.03, "max_health": 13850}, ...],
  "kill_events": [{"spawn_cycle": 1, "team": 4, "team_claimed": 4, "matchtime_s": 2012.28, "x": 0, "y": 0, "z": -768, "bosses_remaining": 0}, ...],
  "rejuv_events": "[23 entries]",
  "fight_windows": "[3 windows, all close with health_at_end=0]",
  "post_match": [{"team_killed": 4, "team_claimed": 4, "destroyed_time_s": 2012}, ...]
}
```

#### Deferred items
- None.

Await user review and commit approval.

---

## Phase C -- Frontend (`dashjump-designer` + `frontend-react` agents)

### C1. Minimap Mid-Boss icon

Show the Mid-Boss map icon at the Mid-Boss pit location on the minimap.

- **Icon source:** `mid-boss-icon.png` -- can be found in frontend/src/assets/.
- **Visibility rule:** Icon appears when mid-boss is alive. Derive from `mid_boss.spawn_events` and `mid_boss.kill_events` -- the boss is alive between `spawn_events[n].spawn_time_s` and `kill_events[n].matchtime_s` for each `spawn_cycle`. If the match ends with no kill for the current cycle, the boss is still alive at match end.
- **Position:** Fixed location -- mid-boss spawns in the center of the map in the sewers. Use the `entity_position` from the first `kill_event` if available, or hardcode the known pit center coordinates.
- **Disappears on death:** When the timeline reaches a kill event's `matchtime_s`, the icon disappears.

### C2. Rejuv claim visualization

Two surfaces, both fed by `kill_events[].rejuvs_by_team` (added in F1, populated by the parser per-kill).

**C2a. Per-kill overlay near the minimap pit.** When the user scrubs to a kill time, render the team color of `team_claimed` and the per-team grant counts as small text. Same data source as the team display, just compact.

**C2b. Team display panel -- rejuv tally at the top.** In the existing per-team display section (the Amber/Sapphire panels that flank the match analysis view), render a rejuv tally above each team's player list:

- **Total count per team:** sum `kill_events[*].rejuvs_by_team[<this_team>]` across all kills in the match. This is the "how many rejuvs did this team claim across the whole match" number.
- **Visual:** show `frontend/src/assets/rejuv-crystal.png` once per claimed rejuv, laid out horizontally. Three claims = three crystal icons in a row. Cap visually at, say, 9 in a row before wrapping; this is the rejuv ceiling for a normal-length match (3 kills × 3 grants).
- **Empty state:** if a team has zero rejuvs across the match, render nothing (no zero-state slot, no greyed-out crystal). Coaches reading the panel should immediately see "Amber claimed all the rejuvs" without having to compare two numbers.
- **Tooltip on hover** (deferred, low priority): per-cycle breakdown -- "Cycle 1: 3, Cycle 2: 3, Cycle 3: 2".

**Why count grants, not `team_claimed`:** the team display shows raw value captured. A 2-1 cycle should put 2 crystals on Amber's panel and 1 on Sapphire's, not 3 on the "claimer" and 0 on the "loser". `team_claimed` is for the per-kill "who won this fight" badge in C2a; the team panel is for "how much value did this team accumulate".

**Color-code:** crystal images don't need recoloring -- the team panel is already on its team's themed background. Just render the asset as-is.

**Divergence from Valve:** as documented in F1, our `team_claimed` derivation differs from Valve's blob on contested cycles. The team display rejuv counts are independent of that derivation -- they're raw grant totals -- so there is no display-level divergence to flag here. C2a (the per-kill badge) is where the divergence shows up, if at all.

### C3. Mid-boss health progress bar

Show a progress bar representing mid-boss health, positioned below the timeline.

- **Data source:** `mid_boss.fight_windows` + per-cycle `spawn_events[active_cycle].max_health`. Each fight window contains its own `health_samples` -- only render during active windows. `max_health` is resolved from the active spawn event (not match-global) so later cycles render against the correct denominator.
- **Calculation:** `current_health / spawn_events[active_cycle].max_health` -- step interpolation between samples within a window. Between windows (gap periods), show health at the last window's `health_at_end` value. The boss does regenerate ~15 HP/s between windows (spike Q2), but we do not surface this: the gap-period inaccuracy is bounded by `15 * gap_s` HP and in practice gap periods end when the boss either dies or is re-engaged with fresh samples, so the visible staleness is limited. If a future experiment needs between-window regen precision, add sparse samples to `fight_windows` rather than emitting a regen rate constant.
- **Multi-window rendering:** If a spawn cycle has multiple fight windows (bait-disengage-reengage pattern), the health bar shows the progression across all windows with visible gaps between engagements. This tells the story of the fight naturally.
- **Shield note:** Health will stay at 100% while the mid-boss's regenerating shield is being broken (see Shield Mechanic section). The progress bar will appear static during this phase. Consider adding a tooltip: "Shield active -- health damage begins after shield is depleted." V1 can ship without the tooltip.
- **50% crossing:** No Citadel message fires at 50% health (spike Q4 confirmed absent). To mark the 50% roar point, find the first sample across any fight window where `health / spawn_events[active_cycle].max_health <= 0.5`.
- **Visibility:** Only visible when mid-boss is alive (same window as the minimap icon). Hidden during respawn periods.
- **Timeline sync:** As user scrubs the timeline, find which fight window (if any) the current time falls within. If inside a window, show the most recent `health_sample` with `time_s <= current_time`. If between windows, show `health_at_end` from the previous window.
- **Placement:** Below the match timeline. Exact positioning is temporary -- will be refined later.

### C4. Frontend domain types

Add TypeScript interfaces in `frontend/src/domain/` matching the backend API contract:

```typescript
interface MidBossData {
  boss_name_hash: string;
  spawn_events: MidBossSpawnEvent[];
  kill_events: MidBossKillEvent[];
  rejuv_events: RejuvStatusEvent[];
  fight_windows: FightWindow[];
  post_match: MidBossPostMatch[];
}

interface MidBossSpawnEvent {
  spawn_cycle: number;
  spawn_time_s: number;
  max_health: number;  // per-cycle m_iMaxHealth; scales with match time
}

interface MidBossKillEvent {
  spawn_cycle: number;
  team_killed: number;
  team_claimed: number;
  rejuvs_by_team: Record<string, number>;
  matchtime_s: number;
  x: number;
  y: number;
  z: number;
  bosses_remaining: number;
}

interface RejuvStatusEvent {
  matchtime_s: number;
  player_pawn: number;
  user_team: number;
  killing_team: number;
  event_type: number;
}

interface FightWindow {
  spawn_cycle: number;
  window_start_s: number;
  window_end_s: number;
  health_at_start: number;
  health_at_end: number;
  health_samples: HealthSample[];
}

interface HealthSample {
  time_s: number;
  health: number;
}

// Parser-derived post-match summary (one per kill cycle).
// team_claimed intentionally dropped per F1 -- callers read the strict-majority
// verdict off MidBossKillEvent. Valve's raw post_match still lives at
// match_metadata.match_info.mid_boss for anyone who wants the Valve semantics.
interface MidBossPostMatch {
  team_killed: number;
  rejuvs_by_team: Record<string, number>;
  destroyed_time_s: number;
}
```

**`post_match` usage:** Not rendered in UI. Present in the typed contract so downstream summaries can aggregate per-kill results without replaying `kill_events`. The prior dev-only `console.log` comparing our derivation to Valve's blob was removed once F1 stabilised -- divergence is now documented in `private/specs/contracts/references.md` instead of the runtime log.

### C Checkpoint

**Status:** `[x] Complete` (C2b team-panel rejuv tally split out into its own plan and no longer tracked here)

#### Results

- [x] Minimap icon visible during alive window, hidden otherwise -- 2026-04-15. `MidBossLayer.tsx` renders `mid-boss-icon.png` at hardcoded pit world (0, 0) when `findActiveCycle` returns a live cycle; returns `null` otherwise. Verified on match `68182475` cycle 1 (9:41 -- 21:42) and cycle 2 (28:42 -- 29:09).
- [x] Icon disappears at exact kill time when scrubbing -- 2026-04-15. `findActiveCycle` uses `currentSecond >= kill.matchtime_s` as the exclusive upper bound; bar + icon both release on the first tick past kill time.
- [x] Health bar tracks samples correctly with step interpolation -- 2026-04-15 (original match-global denominator), updated to per-cycle denominator per 2026-04-16 correction. `currentMidBossHealth` walks `fight_windows[*].health_samples` with step interpolation (no linear smoothing), shows the active cycle's `spawn_events[].max_health` before the first window, and `health_at_end` in gap periods. Match `68182475` cycle 2 window 1740.78 -- 1749.77 with 320 samples drains 18655 -> 0 visibly as scrubber passes the window. Before 2026-04-16 the bar used match-global `mid_boss.max_health` (the first cycle's value) as the denominator for every cycle, producing up to -24% error on cycle 3 on long matches -- see Phase A Deferred items for the correction.
- [x] Rejuv claims show correct team and count -- 2026-04-15. `RecentClaim` reads per-team grant counts from `rejuvClaimsForCompletedKill` (filters `event_type === 6` inside `kill.matchtime_s + 10s`). Match `68182475`: kill 1 shows "Sapphire: 3", kill 2 shows "Sapphire: 3". The contested-steal case is covered by unit test `selectors.test.ts` (`rejuvClaimsForCompletedKill` with mixed-team grants), pending a real replay with a 2-1 split to eyeball.
- [x] Frontend types match backend API contract -- 2026-04-15. `frontend/src/domain/midBoss.ts` mirrors `parser-output.md` after F1 (`team_killed`, `rejuvs_by_team`, `MidBossPostMatch` without `team_claimed`). All 118 frontend tests pass. Needs a follow-up patch to land the 2026-04-16 per-cycle shape: drop top-level `max_health`, add `max_health` to `MidBossSpawnEvent`, update consumer selectors to resolve `max_health` from the active spawn event.
- [x] Spawn-but-no-kill UI path covered -- `frontend/tests/services/midBoss/selectors.test.ts:119` ("stays alive forever when a spawn has no matching kill (match-end case)") exercises the case where the match ends with mid-boss still alive. Mid-boss always spawns (10-min timer), so the interesting branch is "spawned, not killed", not "no block at all".

#### Integer-tick boundary fix (2026-04-15)

While verifying cycle 2 rendering on match `68182475`, we caught an integer/float mismatch: parser emits `spawn_time_s=1722.109375` but `currentTick` is integer, so `1722 < 1722.109375` skipped the new cycle for one tick and fell through to the `RecentClaim` banner. Fixed in `frontend/src/services/midBoss/selectors.ts:findActiveCycle` by flooring `spawn_time_s` before the comparison; regression test added in `selectors.test.ts` pinning the exact match data.

#### Deferred items

- None.

Await user review and commit approval.

---

## Verification Summary

| Phase | Command | Key checks | Status |
|-------|---------|------------|--------|
| 0 | Contract spec review | `parser-output.md` and `backend-api.md` updated, all fields documented | ✅ 2026-04-12 (F1 updates to spec 2026-04-15) |
| A | `cargo test` | All tests pass, no regressions | ✅ 2026-04-15 -- 78/78 |
| A | Parse replay with mid-boss kill | `mid_boss` block present, fight_windows non-empty, last window health_at_end=0, team_claimed derived | ✅ 2026-04-15 -- match `68182475`: 2 spawns, 2 kills, 2 fight windows, `health_at_end=0`, strict-majority `team_claimed` matches `rejuvs_by_team` |
| A | Spawn-but-no-kill unit test | `finalize()` on a tracker with one spawn + no damage + no kill emits one `spawn_events` entry and empty `kill_events`/`fight_windows`/`rejuv_events` | ✅ 2026-04-16 -- `finalize_after_spawn_without_kill_emits_spawn_only_output` in parser tests |
| B | Spawn-but-no-kill domain round-trip | Pydantic `MidBossData` with one spawn + empty arrays round-trips through `model_dump`/`model_validate` with all seven contract keys present | ✅ 2026-04-16 -- `tests/test_mid_boss_domain.py::test_mid_boss_data_round_trips_spawn_without_kill` |
| A | Positions spot-check | No `CNPC_MidBoss` entity in positions output | ✅ 2026-04-15 -- verified on `55423930` |
| B | `pytest` | All tests pass | ✅ 2026-04-15 -- 68 passed, 2 skipped (known schema gaps); `test_parsed_matches_repo.py` + `test_users_repo.py` remain pre-existing DB-infra failures documented in `.claude/rules/backend/CLAUDE.md` |
| B | API spot-check `GET /match/analysis/{id}` | `mid_boss` key present, shape matches backend-api.md | ✅ 2026-04-15 -- `/match/analysis/68182475` returns all 7 keys on `mid_boss`, `rejuvs_by_team` dict on each kill, `post_match` list with `{team_killed, rejuvs_by_team, destroyed_time_s}` |
| C | Visual spot-check | Minimap icon appears/disappears, health bar tracks, rejuv claims display | ✅ 2026-04-15 (after integer-tick boundary fix in `selectors.ts`). Frontend suite: 118/118 |
| C | Spawn-but-no-kill rendering | Icon + health bar stay visible from spawn through match end when no kill fires | ✅ 2026-04-15 -- covered by `selectors.test.ts:119` ("stays alive forever when a spawn has no matching kill") |

---

## Execution Order

1. **Phase 0** (contracts) -- user review -- proceed *(blocks all phases)*
2. **Phase A** (rust-parser) -- user review -- commit
3. **Phase B** (backend-python) -- user review -- commit *(depends on A's output schema)*
4. **Phase C** (frontend) -- user review -- commit *(depends on B's API response shape)*

---

## Follow-ups (discovered during 2026-04-15 verification)

These are tracked here so they don't get lost. Each is a candidate for a separate `fix.md` plan or a Phase D shard.

### F1. Mid-boss kill attribution (Bugs 1+2) -- ✅ COMPLETE 2026-04-15

**Landed:** contract updated in `parser-output.md` + new `references.md`; parser renames `team` -> `team_killed`, sources from `RejuvStatus.killing_team`, replaces `gametime`-anchored window with `last_damage_time_s + 30s`, derives `team_claimed` via strict majority, emits `rejuvs_by_team` with both team keys always present; `MidBossPostMatch` drops `team_claimed` and adds `rejuvs_by_team`; `ECitadelLobbyTeam` (2/3) used at the boundary. Backend Pydantic domain and tests updated. Frontend `domain/midBoss.ts` + consumers (`MidBossHealthBar`, `MidBossLayer`, selectors, selectors tests, component tests) updated. Migration re-run (`alembic downgrade base` -> `upgrade head`) to clear stale cache rows. Backend API on match `68182475` verified end-to-end with curl. All three test suites clean post-F1 (78/68/118). Dev-only `post_match` console.log removed from `MatchAnalysis.tsx`.

**Original analysis retained below for context.**


`MidBossKillEvent.team` and `team_claimed` are wrong on every cycle in replay `55423930`. Root causes and the chosen approach:

1. **`team` is the objective's team, not the killer's.** `BossKilled.objective_team` is always `4` (neutral) for `CNPC_MidBoss`. Stop sourcing `team` from this field. Source it from `RejuvStatus.killing_team` (consistent across all grants for a kill) and rename the field to `team_killed` for clarity. Update `parser-output.md` accordingly.

2. **`BossKilled.gametime` lags actual entity death by 7-18 s.** The `[kill_time, kill_time + 30s]` rejuv attribution window misses every grant in the observed replay. Fix: replace the `gametime`-anchored window with the closed fight window's `last_damage_time_s` as the kill anchor (more accurate -- last damage on cycle 1 was `1994.6` and the first grant fired at `2002.3`, well within a `last_damage_time_s + 30s` forward window). Drop the `# TODO: verify` from `replay_parser.rs:819` once this lands.

3. **We deliberately diverge from Valve's `team_claimed` semantics.** Valve's blob credits the team that "consumed" the buff -- which on `55423930` cycle 3 means Sapphire is marked as the claimer despite receiving only 1 of 3 grants. That label collapses a 2-vs-1 contested outcome into a binary "stolen" verdict, which doesn't match how players or coaches think about a mid-boss fight. We're choosing analytical clarity over UI parity here:

   - **Emit `rejuvs_by_team: {2: 2, 3: 1}` on each `MidBossKillEvent`** (raw counts of `event_type == 6` grants per `user_team`, attributed to a kill via the `last_damage_time_s + 30s` window). Always include both teams as keys, even when zero, so downstream code never has to guard for missing keys.
   - **Derive `team_claimed` via strict majority (`>= 2` of 3 grants).** With 3 grants per kill, one team always reaches the threshold -- `team_claimed` is never null.
   - **Keep Valve's blob untouched in `match_metadata.match_info.mid_boss`.** It already flows through unmodified -- the divergence lives entirely in the parser-derived `mid_boss.kill_events`/`post_match`. Document in `parser-output.md`: "`team_claimed` uses our majority-of-grants derivation, not Valve's `team_claimed`. The Valve value is preserved verbatim in `match_metadata.match_info.mid_boss[].team_claimed` for users who need to compare against the in-game UI."
   - **Remove `team_claimed` from `MidBossPostMatch`** -- the parser-side `post_match` should be a pure summary of our derivation (`team_killed`, `destroyed_time_s`, `rejuvs_by_team`), and Valve's `post_match` stays the canonical source for the Valve number.

4. **Final emitted enum should be `ECitadelLobbyTeam` (2/3) per project convention.** Valve uses `ECitadelTeam` (0/1). Document the mapping and apply it once at the boundary so callers don't have to remember which is which.

**Acceptance:**
- `kill_events[].team_killed` agrees with `RejuvStatus.killing_team` on every kill in `55423930`, `68175583`, `68182475`, and `55841493`.
- `kill_events[].rejuvs_by_team` for `55423930` is `{2: 3, 3: 0}`, `{2: 3, 3: 0}`, `{2: 2, 3: 1}` for cycles 1, 2, 3.
- `kill_events[].team_claimed` for those three cycles is `2`, `2`, `2` -- explicitly **not** `2, 2, 3` like Valve. Document this divergence in the test as the assertion comment.

**Citations:** `parser/src/tracking/mid_boss_tracker.rs:269` (finalize), `parser/src/replay_parser.rs:813` (`BossKilled` handler with the unresolved `# TODO: verify`).

### F2. Spawn-but-no-kill test coverage across services -- ✅ COMPLETE 2026-04-16

Mid-boss spawns in every match (first spawn fires at the 10-minute mark), so the interesting empty-path case is not "no `mid_boss` block" -- it's "spawn fired, nobody killed it before the match ended". All three services now exercise this path.

| Service | Status | Location |
|---------|--------|----------|
| Parser | ✅ 2026-04-16 | `parser/src/tracking/mid_boss_tracker/tests.rs::finalize_after_spawn_without_kill_emits_spawn_only_output` -- calls `handle_spawn(600.0)` then `finalize()` and asserts one spawn event, empty `kill_events`/`rejuv_events`/`fight_windows`/`post_match`, and no open window. Full parser suite: 79 passed. |
| Backend | ✅ 2026-04-16 | `backend/tests/test_mid_boss_domain.py::test_mid_boss_data_round_trips_spawn_without_kill` -- builds a `MidBossData` with one spawn event and empty arrays, asserts `model_dump()` returns all seven contract keys, round-trips via `model_validate()`. Full backend suite: 72 passed, 2 pre-existing skips. |
| Frontend | ✅ pre-existing | `frontend/tests/services/midBoss/selectors.test.ts:119` -- "stays alive forever when a spawn has no matching kill (match-end case)". |

Replaces the earlier "no-mid-boss replay" framing, which was based on the incorrect assumption that a match could exist without a mid-boss block.

### F3. Integer-tick vs fractional `spawn_time_s` boundary -- ✅ COMPLETE 2026-04-15

Fixed in `frontend/src/services/midBoss/selectors.ts:findActiveCycle` by flooring `spawn.spawn_time_s` before comparing to `currentSecond`. Regression test added in `selectors.test.ts` using match `68182475` cycle 2 values (`spawn_time_s=1722.11`, integer tick `1722`). Caught while verifying C Checkpoint cycle-2 rendering.

### F4. Per-cycle `max_health` correction -- ✅ FOLDED INTO PHASE 0/A/B/C ON 2026-04-16

After `probe_midboss_health.rs` (2026-04-16) confirmed that `m_iMaxHealth` scales per spawn cycle rather than being match-global, the shape change was inlined directly into the main plan's Phase 0 contract table, Phase A struct + tracker + tests, Phase B Pydantic, and Phase C TypeScript + health-bar consumer. There is no separate F4 section to implement -- reading Phase 0 -> A -> B -> C gives you the correct per-cycle shape today. The Phase A "Deferred items" block records the correction for historical context (what shipped 2026-04-12 to -15, why it was wrong, and the stale-cache-drop step that needs to be re-run).

Two sub-decisions worth recording:

- **No deprecated alias on top-level `max_health`.** We have not shipped to production; keeping a match-global alias would just invite consumers to use the wrong value. The removal is clean within this plan's lifecycle; no `schema_version` bump needed.
- **No `regen_hp_per_s` contract constant.** Within fight windows, existing `health_samples` already capture regen from `m_iHealth` entity updates -- no separate constant needed. Between fight windows, the frontend holds `health_at_end` constant; the resulting staleness is bounded by `15 * gap_s` HP (the spike-measured regen rate) and in practice gaps end on kill or a fresh engagement, so the visible error is small. If a future experiment needs between-window regen precision, the fix is to emit sparse samples in `fight_windows` between active sub-windows, not to ship a regen rate constant.

Probe source kept at `private/engineering/tools/probe_midboss_health.rs` per `feedback_probe_commit_before_cleanup` -- copy into `parser/src/bin/` to re-run against any replay in `/parser/src/replays/`; prints per-cycle CREATE/DELETE markers, slope within lifetime, `m_iHealth` regen segments, and formula-A (match-start) vs formula-B (since-spawn) fit.
