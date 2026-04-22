# Entity Fields Reference
**Last Updated:** 2026-03-17 (probe_creep_fields run against 55423930_379917638.dem; cage-phase findings confirmed)
**Purpose:** High-signal reference for Deadlock entity fields used in replay parsing. Covers fields with non-obvious semantics, known gotchas, and fields directly relevant to creep tracking, lane pressure, and hero state features.

This is NOT an exhaustive field dump. Add a field only if it has a gotcha, is commonly misused, or is directly load-bearing for a feature we are building.

---

## Source Verification Key

| Label | URL |
|-------|-----|
| `[haste-lifestate]` | https://raw.githubusercontent.com/blukai/haste/main/examples/lifestate.rs |
| `[haste-gametime]` | https://raw.githubusercontent.com/blukai/haste/main/examples/deadlock-gametime.rs |
| `[haste-position]` | https://raw.githubusercontent.com/blukai/haste/main/examples/deadlock-position.rs |
| `[deadlock-CBaseEntity]` | https://raw.githubusercontent.com/SteamDatabase/GameTracking-Deadlock/master/DumpSource2/schemas/server/CBaseEntity.h |
| `[deadlock-MoveType_t]` | https://raw.githubusercontent.com/SteamDatabase/GameTracking-Deadlock/master/DumpSource2/schemas/client/MoveType_t.h |
| `[deadlock-NPC_STATE]` | https://raw.githubusercontent.com/SteamDatabase/GameTracking-Deadlock/master/DumpSource2/schemas/client/NPC_STATE.h |
| `[deadlock-CAI_BaseNPC]` | https://raw.githubusercontent.com/SteamDatabase/GameTracking-Deadlock/master/DumpSource2/schemas/server/CAI_BaseNPC.h |
| `[deadlock-CNPC_Trooper]` | https://raw.githubusercontent.com/SteamDatabase/GameTracking-Deadlock/master/DumpSource2/schemas/server/CNPC_Trooper.h |
| `[deadlock-CAI_CitadelNPC]` | https://raw.githubusercontent.com/SteamDatabase/GameTracking-Deadlock/master/DumpSource2/schemas/server/CAI_CitadelNPC.h |
| `[deadlock-api-gcmessages-common]` | https://raw.githubusercontent.com/deadlock-api/valveprotos-rs/9625c0784beca10634442ef11ede5f022ab186da/protos/deadlock/citadel_gcmessages_common.proto |

---

## Removed / Deprecated Fields

### m_eZipLineLaneColor (REMOVED)
**Parser constant:** `ZIPLINE_LANE_COLOR_KEY` (`fkey_from_path(&["m_eZipLineLaneColor"])`)
**Status:** Field no longer appears in haste-inspector entity snapshots. Confirmed removed as of recent Deadlock patches.

**What it was:** An enum field on player/zipline entities intended to encode which of the four lanes a player was assigned to via zipline color (Yellow=1, Green=3, Blue=4, Purple=6 per `ELaneColor` in `citadel_gcmessages_common.proto`). In practice it was unreliable even before removal — the FIXME in `parser/src/entities/constants.rs` notes it only appeared late in replays, intermittently, and often returned 0.

**Current replacement:** Use `CCitadelUserMsg_TeamMsg` (ID 352) which has a `lane_color: int32` field using the same `ELaneColor` enum values. This is delivered as a packet message, not entity state.

**Parser cleanup needed:** `parser/src/domain/player.rs` still has `zipline_lane_color: u32`, and `parser/src/replay_parser.rs:218` still reads `ZIPLINE_LANE_COLOR_KEY`. These should be removed when lane color is re-implemented via `TeamMsg`.

---

## m_lifeState: uint8

**Source:** `[haste-lifestate]` (defines and confirms values 0 and 2)
**Applies to:** All combat entities -- `CCitadelPlayerPawn`, `CNPC_Trooper`, `CNPC_TrooperBoss`, `CNPC_Boss_Tier2`, `CNPC_Boss_Tier3`, and other NPC classes. Not present on all entities -- `entity.get_value()` returns `None` for entities that do not have this field.

The life state of an entity in the Source Engine combat character hierarchy.

### Values / Semantics

| Value | Constant | Meaning |
|-------|----------|---------|
| `0` | `LIFE_ALIVE` | Entity is alive and active |
| `1` | `LIFE_DYING` | Playing death animation, or falling waiting to hit ground. Transient. |
| `2` | `LIFE_DEAD` | Dead and lying still |
| `3` | `LIFE_RESPAWNABLE` | Dead but eligible to respawn |
| `4` | `LIFE_DISCARDBODY` | Dead and corpse being discarded |

Values 0, 1, and 2 are confirmed in Deadlock demos. Values 3 and 4 are defined in the Source Engine enum but have not been observed in Deadlock demos. Value 1 (LIFE_DYING) co-occurs exactly with `m_NPCState = 10` (NPC_STATE_DYING_CITADEL) on CNPC_Trooper -- confirmed by probe (4,034 matching pairs).

### Gotchas

**`m_lifeState` is not the missing guard for dead creeps appearing on the map.** Adding a lifeState check to the DELETE path or the snapshot path does not fix ghost creeps. The DELETE event is the correct and unambiguous death signal for lane creeps. If dead creeps are appearing, the cause is elsewhere -- see the three candidate bugs documented below.

**Not all entities have this field.** `entity.get_value()` returns `None` for entities without it. Always use an `if let Some(...)` guard rather than `unwrap_or` -- defaulting to 0 (LIFE_ALIVE) would silently treat non-combat entities as alive.

**Health and lifeState do not change in the same delta.** `m_iHealth` and `m_lifeState` are owned by different engine code paths. A creep at `m_iHealth = 0` may still have `m_lifeState = 0` in the same delta. The lifeState `→ 2` transition arrives in a later delta (sometimes the same tick, sometimes the next). This is why `m_iHealth == 0` is not a reliable death signal either.

**For heroes, lifeState is the right signal.** `CCitadelPlayerPawn` entities are never `DELETE`d mid-match (they persist for corpse rendering and respawn). The `0 → 2` transition is the correct hero death signal.

**CNPC_Trooper entities are never DELETE'd during match play.** Zero DELETE events were observed across a full match (55423930_379917638.dem). Entities cycle indefinitely via DEAD->ALIVE->DYING->DEAD life state transitions and are never deallocated. The `handle_creep_delete` code path is not triggered during normal match operation. Any logic that gates on a DELETE event to detect creep death will never fire for CNPC_Trooper. Use the `m_lifeState -> 2 (LIFE_DEAD)` or `m_NPCState -> 12 (NPC_STATE_DEAD_CITADEL)` transition instead.

### Known Ghost-Creep Bug Candidates

These are the three real causes to investigate when dead creeps appear on the map. None involve `m_lifeState`.

**Candidate 1: The `on_tick_end` entity scan is independent of `active_creeps`.**
`on_tick_end` in `replay_parser.rs` iterates all live entities in `ctx.entities()` and pushes any `CNPC_Trooper` with `lane != 0` into `positions_window`. This scan runs every second and does not consult `active_creeps`. A creep in the transient LIFE_DYING state is still present in `ctx.entities()` with a valid lane, so it will produce a position entry for that tick. If the ghost creep appears in the general NPC positions layer (not in the `laneCreepData.creeps` timelines), this is the cause.

**Candidate 2: The `handle_creep_update` late-CREATE path has no lane=0 guard.**
If an UPDATE arrives for a creep not in `active_creeps`, `handle_creep_update` calls `handle_creep_create` directly. If the lane field has been zeroed by the engine during teardown (the entity is dying and the engine is clearing its fields), that call registers a lane-0 creep. This creep then persists in `creep_timelines` and emits snapshots until its DELETE arrives. Ghost appears at (0,0) or at last valid position with `lane = 0` in the snapshot.

**Candidate 3: Entity index reuse (CONFIRMED -- NOT a candidate, it is the design).**
CNPC_Trooper entity indices ARE reused within a match. Deadlock recycles entity slots in-place via a `life_state = DEAD → ALIVE` transition (no DELETE/CREATE pair). The `CreepTracker` handles this by detecting DEAD→ALIVE transitions in `handle_creep_update` and assigning a new `wave_id` to the recycled slot. This is expected behavior -- not a bug. Any new entity tracker for lane creeps must account for this recycling pattern. See `parser-mental-model.md` -- "Entity Lifecycle in Deadlock Demos".

### Correct Usage Pattern

For hero alive/dead detection:
```rust
const LIFE_STATE_KEY: u64 = fkey_from_path(&["m_lifeState"]);
const LIFE_ALIVE: u8 = 0;
const LIFE_DEAD: u8 = 2;

// in on_entity, for CCitadelPlayerPawn only:
let Some(life_state): Option<u8> = entity.get_value(&LIFE_STATE_KEY) else {
    return Ok(());
};
// life_state == LIFE_ALIVE  => hero is alive
// life_state == LIFE_DEAD   => hero has died (corpse still visible)
// life_state == 3           => hero is dead and respawn-eligible (unconfirmed in Deadlock)
```

---

## m_iHealth: int32

**Source:** Used in `[haste-lifestate]` implicitly (the example ignores it deliberately); field path observed directly in parser codebase (`parser/src/tracking/boss_tracker.rs`).
**Applies to:** All combat entities.

Current health value of the entity.

### Gotchas

**Not a reliable death signal on its own.** Health reaches 0 before `m_lifeState` transitions. A creep or hero at `m_iHealth = 0` with `m_lifeState = 0` is in the transient LIFE_DYING window (value 1 may not even network before the entity dies or is deleted). See `m_lifeState` entry above.

**Type can vary.** The field is `int32` in the entity network but in practice the parser reads it as `i32`. Boss health has been observed at values requiring full 32-bit range; creep health fits in `u16` but should be read as `i32` to avoid type mismatch panics.

**Does not reset to 0 before DELETE.** For entities that are `DELETE`d at death (lane creeps), `m_iHealth` at the time of the `DELETE` delta is not guaranteed to be 0. The engine may issue `DELETE` without a preceding health-to-zero `UPDATE` in the same tick.

### Correct Usage Pattern

Use for health percentage displays and boss health timelines. Do not use as a death condition:
```rust
// Good: health for display / timeline
let health: i32 = entity.get_value(&fkey_from_path(&["m_iHealth"])).unwrap_or(0);

// Bad: death detection
if health == 0 { /* WRONG -- entity may still be alive */ }
```

---

## m_iLane: int32

**Source:** Used in `parser/src/entities/constants.rs` as `NPC_LANE_KEY`; field path `&["m_iLane"]`.
**Applies to:** NPC entities -- `CNPC_Trooper`, `CNPC_TrooperBoss`, `CNPC_Boss_Tier2`, and other lane-assigned NPCs.

The lane index assigned to the NPC. Maps to the four Deadlock lanes.

### Values / Semantics

| Value | Meaning |
|-------|---------|
| `0` | No lane assigned -- pre-match or unassigned entity |
| `1`–`4` | Lane indices (Yellow, Green, Blue, Purple -- exact color-to-index mapping depends on team) |

### Gotchas

**Lane 0 means pre-match / invalid.** Creeps that exist before match start carry `m_iLane = 0`. The parser filters these with `lane != 0` in `should_track_position`. If you skip this filter you will get ghost creeps tracked at position (0, 0) in the pre-match period, inflating wave counts.

**`m_n*` player lane fields are wire-type `i8` despite `int32` schema.** `m_nAssignedLane`, `m_nOriginalLaneAssignment` (on `CCitadelPlayerController`), and `m_nDeducedLane` (on `CCitadelPlayerPawn`) are all declared `int32` in GameTracking schemas but encoded as `i8` on the demo wire. Reading with `entity.get_value::<i32>()` returns `None` (type tag mismatch in haste), causing `unwrap_or(0)` to silently zero every value. Fix: read as `i8`, widen with `i32::from()`. Rule: any `m_n*` integer field that consistently returns 0 for a known non-zero game value should be tried as `i8` first. `m_iLane` on NPC entities is `int32` and has not shown this issue.

### Correct Usage Pattern

```rust
const NPC_LANE_KEY: u64 = fkey_from_path(&["m_iLane"]);
let lane: i32 = entity.get_value(&NPC_LANE_KEY).unwrap_or(0);
if lane == 0 {
    return Ok(()); // pre-match / unassigned, skip
}
```

---

## m_iTeamNum: uint32

**Source:** Used throughout the parser as `TEAM_KEY`; field path `&["m_iTeamNum"]`.
**Applies to:** All team-affiliated entities -- player pawns, NPCs, objectives.

The team affiliation of an entity.

### Values / Semantics

| Value | Meaning |
|-------|---------|
| `0` | Unassigned / spectator |
| `2` | Team Amber (team 0 in game UI) |
| `3` | Team Sapphire (team 1 in game UI) |

The exact label-to-number mapping (Amber=2, Sapphire=3) is consistent across Deadlock demos observed to date but is not formally defined in a proto enum accessible from replay data.

### Gotchas

**Team 0 and team 2/3 are different things.** `m_iTeamNum = 0` does not mean "team index 0" in the player sense -- it means unassigned. The game uses 2 and 3 as the actual team numbers. Indexing a two-element array with the raw value will panic. Always map: `team_index = m_iTeamNum - 2`.

**Player controller and player pawn carry the same team.** Both `CCitadelPlayerController` and `CCitadelPlayerPawn` network `m_iTeamNum`. Reading it from either is equivalent for team assignment. The NPC entities (`CNPC_Trooper`, etc.) also carry `m_iTeamNum` indicating which team owns that creep wave or objective.

### Correct Usage Pattern

```rust
const TEAM_KEY: u64 = fkey_from_path(&["m_iTeamNum"]);
let team: u32 = entity.get_value(&TEAM_KEY).unwrap_or(0);
if team < 2 {
    return Ok(()); // unassigned, skip
}
let team_index: usize = (team - 2) as usize; // 0 = Amber, 1 = Sapphire
```

---

## m_flGameStartTime: float32 (nested)

**Source:** `[haste-gametime]` -- field path `&["m_pGameRules", "m_flGameStartTime"]`
**Applies to:** `CCitadelGameRulesProxy` only.

The wall-clock time (in demo tick seconds) at which the match started. This is the base value for converting absolute tick time into match-relative time.

### Gotchas

**Reads as 0.0 before match start.** The field is present and networkable before the match starts, returning a near-zero float. Comparisons like `< 0.001` are used to detect the uninitialized state. Do not use `== 0.0` for float comparison -- use a small epsilon.

**Rounded to seconds loses precision.** The raw value is a float (e.g., `26.866669`). The parser rounds it to `u32` seconds for match-relative indexing. This means the first second of the timeline (index 0) may actually represent 0–1.133 seconds of real game time depending on when `m_flGameStartTime` is first set. This is an accepted imprecision for per-second creep timeline buckets.

**Must be read from the `m_pGameRules` nested path.** The entity is `CCitadelGameRulesProxy` but the actual game rules fields are one level deeper under `m_pGameRules`. Omitting the intermediate path component returns `None`.

### Correct Usage Pattern

```rust
const GAME_START_TIME_KEY: u64 =
    fkey_from_path(&["m_pGameRules", "m_flGameStartTime"]);

// In on_entity, when entity.serializer_name_heq(DEADLOCK_GAMERULES_ENTITY):
let raw: f32 = entity
    .try_get_value(&GAME_START_TIME_KEY)
    .unwrap_or(0.0);
if raw < 0.001 {
    return Ok(()); // match not yet started
}
let match_start_time_s: u32 = raw.round() as u32;
```

---

## Body Component Position Fields (nested)

**Source:** `[haste-position]` -- full paths shown verbatim in example source
**Applies to:** Any entity with a physical body -- player pawns, NPCs, objectives.

World position is encoded as a cell+vector pair per axis. The cell is a coarse grid index (`u16`) and the vector is a sub-cell offset (`f32`). Both must be combined using `deadlock_coord_from_cell(cell, vec)` to get a usable world coordinate.

### Field Paths

```
X: ["CBodyComponent", "m_skeletonInstance", "m_vecOrigin", "m_cellX"]  (u16)
   ["CBodyComponent", "m_skeletonInstance", "m_vecOrigin", "m_vecX"]   (f32)

Y: ["CBodyComponent", "m_skeletonInstance", "m_vecOrigin", "m_cellY"]  (u16)
   ["CBodyComponent", "m_skeletonInstance", "m_vecOrigin", "m_vecY"]   (f32)

Z: ["CBodyComponent", "m_skeletonInstance", "m_vecOrigin", "m_cellZ"]  (u16)
   ["CBodyComponent", "m_skeletonInstance", "m_vecOrigin", "m_vecZ"]   (f32)
```

### Gotchas

**deadlock-api/haste uses 2-level path, not 4-level.** haste-inspector displays the full 4-level hierarchy (`["CBodyComponent", "m_skeletonInstance", "m_vecOrigin", "m_cellX"]`), but `deadlock-api/haste` stores keys at the 2-level path (`["CBodyComponent", "m_cellX"]`). The 4-level path returns `None` on every tick with the deadlock-api fork. See `parser/src/utils/entity_position.rs` for the validated implementation and `private/specs/deadlock-api-haste-reference.md` for the key-hash explanation.

**Reading only the `m_vec*` float gives wrong coordinates.** The float is a sub-cell offset, not an absolute world position. Without the cell component, positions cluster around origin. Always combine both with `deadlock_coord_from_cell`.

**Z axis is rarely needed for lane pressure.** Creep and player positions for 2D lane pressure calculations only require X and Y. Z is only needed for vertical separation (e.g., determining if a player is on a zipline above lane level). The existing `get_entity_position` utility in `parser/src/utils/entity_position.rs` returns all three.

### Correct Usage Pattern

```rust
use haste::entities::{deadlock_coord_from_cell, fkey_from_path};

fn get_xy(entity: &Entity) -> Option<(f32, f32)> {
    const CX: u64 = fkey_from_path(&["CBodyComponent","m_skeletonInstance","m_vecOrigin","m_cellX"]);
    const VX: u64 = fkey_from_path(&["CBodyComponent","m_skeletonInstance","m_vecOrigin","m_vecX"]);
    const CY: u64 = fkey_from_path(&["CBodyComponent","m_skeletonInstance","m_vecOrigin","m_cellY"]);
    const VY: u64 = fkey_from_path(&["CBodyComponent","m_skeletonInstance","m_vecOrigin","m_vecY"]);

    let x = deadlock_coord_from_cell(entity.get_value::<u16>(&CX)?, entity.get_value::<f32>(&VX)?);
    let y = deadlock_coord_from_cell(entity.get_value::<u16>(&CY)?, entity.get_value::<f32>(&VY)?);
    Some((x, y))
}
```

---

## m_pGameRules.m_bGamePaused / m_nPauseStartTick / m_nTotalPausedTicks

**Source:** `[haste-gametime]` -- field paths verified verbatim
**Applies to:** `CCitadelGameRulesProxy` only.

Fields required to compute accurate match-relative game time when pauses occur.

### Values / Semantics

| Field | Type | Meaning |
|-------|------|---------|
| `m_pGameRules.m_bGamePaused` | `bool` | True while the match is currently paused |
| `m_pGameRules.m_nPauseStartTick` | `i32` | Tick at which the current (or last) pause began |
| `m_pGameRules.m_nTotalPausedTicks` | `i32` | Cumulative ticks spent paused across all pauses in the match |

Correct game time formula (from `[haste-gametime]`):
```
game_time = ((net_tick - total_paused_ticks) * tick_interval) - game_start_time
```
where `net_tick` is from the `CnetMsgTick` packet (not `ctx.tick()`).

### Gotchas

**`ctx.tick()` does not account for pauses.** The context tick counter increments continuously including during pauses. For match-relative time used in analytics timelines, the formula above is required. The dashjump parser currently uses `ctx.tick()` with `tick_interval` and `match_start_time_s` for match-relative seconds, which means paused time is included in the timeline. This is a known imprecision acceptable for current features but would need fixing for pause-aware analytics.

### Correct Usage Pattern

See `[haste-gametime]` for the full implementation including `CnetMsgTick` handling via `on_packet`.

---

## m_NPCState: uint32

**Source:** `[deadlock-CAI_BaseNPC]` (field declaration with `MNetworkEnable`, type `NPC_STATE`); `[deadlock-NPC_STATE]` (enum definition, underlying type `uint32_t`)
**Applies to:** `CAI_BaseNPC` and all subclasses -- `CNPC_Trooper`, `CNPC_TrooperBoss`, `CNPC_Boss_Tier2`, `CNPC_Boss_Tier3`, and other NPC types. Not present on `CCitadelPlayerPawn` (heroes use `m_lifeState` instead).

The current state of the NPC AI state machine. This is the primary field for understanding what an NPC is currently doing at the AI level -- whether it is idle in its spawn area, alert to a threat, actively in combat, or dead.

### Values / Semantics

| Value | Constant | Meaning |
|-------|----------|---------|
| `-1` | `NPC_STATE_INVALID` | Invalid / uninitialized |
| `0` | `NPC_STATE_INIT` | Initializing; NPC exists but AI has not started (replaces SDK NONE) |
| `1` | `NPC_STATE_IDLE` | No threat detected; patrolling or standing |
| `2` | `NPC_STATE_ALERT` | Threat detected but not yet engaged |
| `3` | `NPC_STATE_COMBAT` | Actively fighting |
| `4` | `NPC_STATE_SCRIPT` | Executing a scripted sequence (suppressed in UI) |
| `5` | `NPC_STATE_DEAD` | Dead (corpse, pending DELETE) -- standard SDK value |
| `6` | `NPC_STATE_INERT` | Inert / frozen (spawning, stunned, or disabled) |
| `7` | `NPC_STATE_SYNCHRONIZED_SECONDARY` | Synchronized to a primary NPC (suppressed in UI) |
| `8` | `NUM_NPC_STATES` | Sentinel / count only, not a valid state |
| `10` | `NPC_STATE_DYING_CITADEL` | **Deadlock-specific extension.** Playing death animation. Co-occurs exactly with `m_lifeState = 1` (LIFE_DYING). Confirmed by probe (4,034 exact pairs). |
| `12` | `NPC_STATE_DEAD_CITADEL` | **Deadlock-specific extension.** Dead and still. Co-occurs exactly with `m_lifeState = 2` (LIFE_DEAD). Confirmed by probe (3,563 exact pairs). All pre-spawned CNPC_Trooper entities start in this state at match load. |

### Gotchas

**`NPC_STATE_INIT` (0) is rarely seen on live creep entities.** Lane creeps arrive in demos already in IDLE or COMBAT state. Value 0 should be treated as "not yet active" and filtered.

**`NPC_STATE_ALERT` (2) is valid and observed on CNPC_Trooper.** This state fires when the NPC AI detects an enemy at range but has not yet engaged. It is a known, confirmed enum value -- NOT an unknown or out-of-range value. In the zipline-phase context, creeps may enter ALERT state during descent if enemies are visible at long range. ALERT is NOT a pre-spawn / cage-travel indicator on its own; it can also appear on fully deployed in-lane creeps that are marching and have spotted an enemy before getting close enough to attack. Do not use ALERT as a zipline-phase filter.

**Values 10 and 12 are confirmed Deadlock-specific extensions to the NPC_STATE enum.** The GameTracking schema defines `NUM_NPC_STATES = 8` as the maximum standard value, but Valve has added two additional states beyond that range used exclusively by CNPC_Trooper lifecycle transitions.

- **Value 10 = NPC_STATE_DYING_CITADEL (CONFIRMED).** Co-occurs exactly with `m_lifeState = 1` (LIFE_DYING) on every dying CNPC_Trooper. Confirmed by probe: 4,034 exact pairs across a full match (55423930_379917638.dem). This is the death animation / falling state. The death sequence is: `npc_state 2->10, life_state 0->1` (DYING transition), then `npc_state 10->12, life_state 1->2` (DEAD transition).

- **Value 12 = NPC_STATE_DEAD_CITADEL (CONFIRMED).** Co-occurs exactly with `m_lifeState = 2` (LIFE_DEAD) on every dead CNPC_Trooper. Confirmed by probe: 3,563 exact pairs across a full match. All pre-spawned CNPC_Trooper entities start with `npc_state = 12` at match load -- they are created in DEAD state and cycle from there.

**The earlier hypothesis that value 12 might indicate reading `m_MoveType` by mistake has been disproven.** With the fkey confirmed correct, value 12 is the DEAD state, not `MOVETYPE_LAST`/`MOVETYPE_INVALID`. The two-field correlation (exact count match with life_state=2) makes this unambiguous.

**Value 2 (`NPC_STATE_ALERT`) is confirmed valid.** This is a known enum entry and is expected on `CNPC_Trooper` entities that have detected enemies. It does not indicate cage phase on its own.

**`m_NPCState` transitions to DEAD before the DELETE event for bosses.** For `CNPC_Boss_Tier2` and `CNPC_Boss_Tier3` (Walkers/Patrons), the state machine transitions to `NPC_STATE_DEAD` while the death animation plays. The entity is not deleted immediately. For lane creeps (`CNPC_Trooper`) the DELETE event is the reliable signal. For bosses, either `NPC_STATE_DEAD` or the `BossKilled` user message (ID 347) is the correct death signal.

**Underlying type is `uint32_t` but value -1 exists.** `NPC_STATE_INVALID` = -1 is a valid enum entry. Reading the field as `u32` gives `0xFFFFFFFF` (4294967295). Always use a signed read (`i32`) or explicitly handle the sentinel: if value is `u32::MAX`, treat as invalid.

**Not present on hero pawns.** `CCitadelPlayerPawn` does not inherit from `CAI_BaseNPC`. Attempting to read `m_NPCState` from a player pawn entity returns `None`. Use `m_lifeState` for hero alive/dead state.

### Correct Usage Pattern

For lane creeps, `m_NPCState` is informational -- it shows combat engagement but does not replace the DELETE event for death detection. For bosses, the DEAD transition is useful:

```rust
const NPC_STATE_KEY: u64 = fkey_from_path(&["m_NPCState"]);

const NPC_STATE_INIT: i32 = 0;
const NPC_STATE_IDLE: i32 = 1;
const NPC_STATE_ALERT: i32 = 2;
const NPC_STATE_COMBAT: i32 = 3;
const NPC_STATE_DEAD: i32 = 5;
const NPC_STATE_INERT: i32 = 6;
const NPC_STATE_INVALID: i32 = -1;
// Deadlock-specific extensions (confirmed by probe against 55423930_379917638.dem):
const NPC_STATE_DYING_CITADEL: i32 = 10; // co-occurs exactly with life_state=1 (LIFE_DYING)
const NPC_STATE_DEAD_CITADEL: i32 = 12;  // co-occurs exactly with life_state=2 (LIFE_DEAD)

// Read as i32 to handle the -1 sentinel correctly
let npc_state: i32 = entity.get_value::<u32>(&NPC_STATE_KEY)
    .map(|v| v as i32)
    .unwrap_or(NPC_STATE_INVALID);

match npc_state {
    NPC_STATE_COMBAT        => { /* creep is fighting */ }
    NPC_STATE_DEAD          => { /* boss death animation playing, kill confirmed (standard SDK) */ }
    NPC_STATE_INERT         => { /* NPC frozen/disabled */ }
    NPC_STATE_DYING_CITADEL => { /* CNPC_Trooper: death animation playing (Deadlock-specific) */ }
    NPC_STATE_DEAD_CITADEL  => { /* CNPC_Trooper: dead / pre-spawn at base (Deadlock-specific) */ }
    _                       => {}
}
```

---

## m_MoveType: uint8 (MoveType_t enum)

**Source:** `[deadlock-CBaseEntity]` (field declaration confirmed `MNetworkEnable`); `[deadlock-MoveType_t]` (enum definition, underlying type `uint8_t`)
**Applies to:** `CBaseEntity` and all subclasses. Present on all entities with physics simulation -- `CNPC_Trooper`, `CCitadelPlayerPawn`, objectives, etc.

The movement mode currently assigned to the entity by the physics/movement system. For NPCs, this reflects the current locomotion mode set by the AI motor.

### Values / Semantics

| Value | Constant | Meaning for NPCs |
|-------|----------|-----------------|
| `0` | `MOVETYPE_NONE` | No movement -- entity is stationary and has no locomotion orders. NOT observed on CNPC_Trooper in practice (see below). |
| `2` | `MOVETYPE_WALK` | Walking movement mode -- used by player pawns, rare for NPCs. |
| `9` | `MOVETYPE_STEP` | Stepped NPC ground movement -- confirmed as the constant value for CNPC_Trooper across all lifecycle phases. |
| `3` | `MOVETYPE_FLY` | Flying movement -- not expected on standard troopers. |
| `5` | `MOVETYPE_VPHYSICS` | Physics object. Theoretical during death ragdoll, but not observed in demos for CNPC_Trooper. |
| `10` | `MOVETYPE_SYNC` | Movement synchronized to another entity. Observed on `CCitadelPlayerPawn` when riding a zipline. |

Full enum from `[deadlock-MoveType_t]`: NONE=0, OBSOLETE=1, WALK=2, FLY=3, FLYGRAVITY=4, VPHYSICS=5, PUSH=6, NOCLIP=7, OBSERVER=8, STEP=9, SYNC=10, CUSTOM=11, LAST=12, INVALID=12.

**Value 12 in `m_MoveType` vs. `m_NPCState`.** `MOVETYPE_LAST = 12` and `MOVETYPE_INVALID = 12` exist in the `MoveType_t` enum as sentinel values. Separately, `m_NPCState` value 12 is a confirmed Deadlock-specific extension (NPC_STATE_DEAD_CITADEL) -- it is NOT caused by reading `m_MoveType` by mistake. If your probe returns 12 for `m_NPCState`, that is the correct DEAD state for CNPC_Trooper, not a field mix-up. Verify which field you intend to read by checking the fkey constant.

### CNPC_Trooper: m_MoveType is ALWAYS 9 (Confirmed, Ruled Out as Discriminator)

**This field was tested and ruled out as a cage/zipline phase discriminator for `CNPC_Trooper`.**

Empirical validation via haste-inspector across many entities and many ticks in two Deadlock demos showed `m_MoveType` was always 9 (`MOVETYPE_STEP`) -- in every observed lifecycle phase, including before cage launch, during zipline travel, and during in-lane walking. The field never transitioned to 0 (`MOVETYPE_NONE`) or any other value.

The prior hypothesis -- that the field would transition from `MOVETYPE_NONE` to `MOVETYPE_STEP` when the cage drops -- is disproven.

**For cage-phase discrimination, see the `CNPC_Trooper Zipline/Cage Phase vs. In-Lane Phase Discrimination` section below.**

### Gotchas

**`m_nActualMoveType` is NOT networkable.** In `CBaseEntity.h`, `m_nActualMoveType` is declared immediately after `m_MoveType` with no `MNetworkEnable` annotation of its own. The annotation belongs solely to `m_MoveType`. `m_nActualMoveType` cannot be read from replay data.

**Flying trooper variants exist in VData.** `CAI_NPC_TrooperVData` has `m_flFlySpeed` and `m_flFlyHeight` fields, indicating a flying trooper variant is defined. Such a creep would use `MOVETYPE_FLY (3)` rather than STEP when marching. The `MOVETYPE_STEP` always-9 observation applies to standard ground troopers.

### Correct Usage Pattern

For detecting hero zipline state (`CCitadelPlayerPawn` only -- not applicable to CNPC_Trooper):

```rust
const MOVE_TYPE_KEY: u64 = fkey_from_path(&["m_MoveType"]);
const MOVETYPE_WALK: u8 = 2;
const MOVETYPE_SYNC: u8 = 10; // on zipline -- unconfirmed against GC message, but strongly inferred

// in on_entity for CCitadelPlayerPawn:
let move_type: u8 = entity.get_value(&MOVE_TYPE_KEY).unwrap_or(0);
let on_zipline = move_type == MOVETYPE_SYNC;
```

For `CNPC_Trooper`: do not use this field for lifecycle phase discrimination. It is always 9.

---

## m_flCreateTime: GameTime_t (float32)

**Source:** `[deadlock-CBaseEntity]` (confirmed `MNetworkEnable`)
**Applies to:** All `CBaseEntity` subclasses -- including `CNPC_Trooper`, `CCitadelPlayerPawn`, objectives.

The server game time at which the entity was created. This is an absolute server timestamp, NOT match-relative time. To convert to match-relative seconds, subtract `m_flGameStartTime` from `CCitadelGameRulesProxy`.

### Why This Field Matters for Zipline Discrimination

For `CNPC_Trooper` entities that are recycled between wave cycles (same entity index reused), `m_flCreateTime` is set once at entity creation and does NOT update when the entity is recycled. It reflects when the entity pool slot was first allocated, not when the current wave was dispatched. This means `m_flCreateTime` is NOT a reliable "when did this wave launch" signal for recycled entities. It is only meaningful for distinguishing entities that were freshly allocated (typically the very first wave) from those that have been through multiple recycling cycles.

**Confidence level: Confirmed networkable.** Field presence confirmed via `CBaseEntity.h` schema dump. Actual values across wave cycles have NOT been validated against a live demo -- the recyclability caveat is inferred from how Source 2 entity pools work.

### Gotchas

**Not a wave-launch timestamp for recycled entities.** Do not use `m_flCreateTime` to determine when a wave was sent to lane. The engine reuses entity slots; only the first allocation of that slot is captured in this field.

**Type is `GameTime_t` not `float32`.** In haste, `GameTime_t` serializes as a regular `f32`. Read it as `f32`.

### Correct Usage Pattern

```rust
const CREATE_TIME_KEY: u64 = fkey_from_path(&["m_flCreateTime"]);

// Only useful for: "was this entity ever recycled at all?"
// Not useful for: "when was this creep sent to lane?"
let create_time: f32 = entity.get_value(&CREATE_TIME_KEY).unwrap_or(0.0);
```

---

## m_spawnflags: uint32

**Source:** `[deadlock-CBaseEntity]` (confirmed `MNetworkEnable`)
**Applies to:** All `CBaseEntity` subclasses.

A bitmask of flags set at entity spawn time. These flags control entity behavior at the engine level (e.g., whether the entity starts disabled, whether it can be dormant, whether it is networked to all clients).

### CNPC_Trooper: m_spawnflags is ALWAYS 1028 (Confirmed, Ruled Out as Discriminator)

**Probe result: every single CNPC_Trooper entity across the entire match (55423930_379917638.dem) has `m_spawnflags = 0x00000404 (1028)`. No variation observed across 2197 CREATE events.** This field cannot discriminate cage entities from fighting creeps, or any other lifecycle phase distinction.

### Gotchas

**Set at spawn time, does not change.** `m_spawnflags` is static for all observed CNPC_Trooper entities.

**Flag values are not in any public proto or schema dump.** The numeric flag values are defined in engine source that is not available in the GameTracking schema dumps. The constant 1028 (0x404) is empirically confirmed but its bit-level meaning (which specific engine behaviors it enables) is not documented.

---

## CNPC_Trooper Zipline/Cage Phase vs. In-Lane Phase Discrimination

**Source:** Probe run against 55423930_379917638.dem (2026-03-17)
**Purpose:** Documents the confirmed mechanism for distinguishing cage-carrier entities from actual fighting lane creeps.

### Confirmed: Two Distinct Entity Pools Per Wave

The earlier "two-transition problem" framing (Option A vs. Option B) is resolved. **Option B is correct.** Each game wave uses two completely separate CNPC_Trooper entity pools -- they are not the same entity going through two transitions.

**Cage entities (health = 1):**
- Created in DEAD state (`npc_state=12, life_state=2`) at base. Z elevation ~1410-1422 (elevated base area).
- Transition DEAD->ALIVE at wave launch. Z immediately rises as the cage travels the zipline arc.
- Are the visual cage/zipline carrier sprites. NOT killable fighting units.
- Transition back to DEAD ~13-15 seconds after launch when the cage hits the drop-point ground.
- `m_iHealth = 1` throughout their entire lifecycle. This is constant and never changes.

**Actual lane creeps (health = 350+):**
- Created in DEAD state at same base positions.
- Are teleported to the lane drop point while still DEAD (Z ~248-376 = lane floor level).
- Transition DEAD->ALIVE ~13-15 seconds after the cage launch, at lane floor Z. These are the walking, fighting creeps.
- `m_iHealth` starts at ~350+ and decreases when taking damage.

**Wave timing observed in probe (match 55423930_379917638.dem):**
- Cage entities DEAD->ALIVE: tick ~1026 (server time ~17.1s, before match start at 17.688s)
- Actual creeps DEAD->ALIVE: tick ~1828 (server time ~30.5s)
- Gap: ~13.4 seconds between cage launch and creep deployment

**The 13-15 second gap exceeds the parser's 5-second wave grouping window.** Without filtering, the parser registers two wave entries per real wave (one for cage entities, one for actual creeps).

### The Discriminator: m_iHealth == 1

**`m_iHealth == 1` is the confirmed discriminator for cage entities.** Use this to exclude cage entities from wave registration.

```
CNPC_Trooper with m_iHealth == 1  ==>  cage/zipline carrier entity (skip)
CNPC_Trooper with m_iHealth > 1   ==>  actual fighting lane creep (track)
```

The `CAGE_ENTITY_HEALTH` constant (value 1) is defined in `parser/src/entities/constants.rs`.

### What Each Field Shows Per Entity Pool

| Field | Cage Entity | Actual Lane Creep | Confidence |
|-------|-------------|-------------------|-----------|
| `m_iHealth` | **1 (constant, never changes)** | 350+ at deployment; decreases in combat | **CONFIRMED by probe -- use as discriminator** |
| `m_lifeState` | 2 (DEAD) at base; 0 (ALIVE) during travel; 2 (DEAD) after landing | 2 (DEAD) at base; 0 (ALIVE) when walking; 1 (DYING) when killed; 2 (DEAD) after death | Confirmed |
| `m_NPCState` | 12 at base; transitions during travel | 12 at base; 1/2/3 when active; 10 (DYING); 12 (DEAD) | Confirmed |
| `m_MoveType` | 9 (STEP) -- always | 9 (STEP) -- always | Confirmed -- ruled out as discriminator |
| `m_spawnflags` | 1028 (0x404) -- always | 1028 (0x404) -- always | Confirmed -- ruled out as discriminator |
| `m_fFlags` | **NOT PRESENT on CNPC_Trooper in demos.** Returns `None` for all 2197 entities. | Same -- not present | **CONFIRMED NOT NETWORKABLE** |
| `m_iLane` | 1-4 (assigned at start) | 1-4 (same) | Confirmed |
| Position Z (at DEAD->ALIVE) | ~1410-1422 (elevated base; rises immediately) | ~248-376 (lane floor; already teleported while DEAD) | Confirmed -- corroborates health filter |

### Ruled-Out Fields

- **`m_MoveType`**: Always 9 (MOVETYPE_STEP). Confirmed across two demos and probe output. Ruled out.
- **`m_fFlags`**: **Confirmed NOT present on CNPC_Trooper in demos.** Probe returned `None` for every single entity across 2197 CREATE events and all UPDATE events. The FL_ONGROUND approach is eliminated. The field is simply not networked for this entity class.
- **`m_spawnflags`**: Always 1028 for all CNPC_Trooper entities. No variation. Ruled out.
- **`m_NPCState`**: Values 1/2/3 appear in both entity pools when ALIVE. Cannot discriminate alone.
- **`m_lifeState`**: Both pools use the same DEAD/ALIVE/DYING cycle. Cannot discriminate alone.
- **`m_iLane`**: Same value in both pools. Ruled out.
- **`m_flCreateTime`**: Set at entity slot allocation; not updated on recycling. Confirms the two-pool structure (different create times per pool) but is not a runtime discriminator.
- **Z position**: Cage entities launch from Z=1410-1422; actual creeps appear at Z=248-376. Z alone is not a clean threshold (ranges could overlap across lanes and at other map Z levels), but it corroborates the health-based filter.

### Recommended Approach

Filter cage entities at wave registration time using `m_iHealth == 1`:

```rust
// In handle_creep_create / wave assignment:
let health: i32 = entity.get_value(&HEALTH_KEY).unwrap_or(0);
if health == CAGE_ENTITY_HEALTH {
    return Ok(()); // cage/zipline carrier entity -- not a fighting creep, skip
}
```

The `HEALTH_KEY` and `CAGE_ENTITY_HEALTH` constants are defined in `parser/src/entities/constants.rs`.

---

## CNPC_Trooper Networkable Field Inventory

**Source:** `[deadlock-CNPC_Trooper]`, `[deadlock-CAI_CitadelNPC]`, `[deadlock-CAI_BaseNPC]`, `[deadlock-CBaseEntity]`
**Purpose:** Exhaustive record of which fields on `CNPC_Trooper` are networkable (readable from demos) vs. server-only. Captures findings from the wave-cycle lifecycle research session (2026-03-17).

### Networkable fields (confirmed MNetworkEnable in inheritance chain)

| Field | Declared on | Type | Notes |
|-------|-------------|------|-------|
| `m_iLane` | `CNPC_Trooper` | `int32` | Lane index 1-4; 0 = unassigned / pre-match |
| `m_hTargetedEnemy` | `CNPC_Trooper` | `CHandle<CBaseEntity>` | Entity handle of current AI target |
| `m_flHealingChargeParticlePct` | `CNPC_Trooper` | `float32` (0-1, 8-bit packed) | Medic healing charge particle percentage |
| `m_NPCState` | `CAI_BaseNPC` | `NPC_STATE` (uint32_t) | AI state machine; see m_NPCState entry |
| `m_bMinion` | `CAI_CitadelNPC` | `bool` | True if this NPC is a minion-type |
| `m_bBeamActive` | `CAI_CitadelNPC` | `bool` | Eye beam attack active |
| `m_vEyeBeamTarget` | `CAI_CitadelNPC` | `VectorWS` | Target position for eye beam |
| `m_vecWeakPoints` | `CAI_CitadelNPC` | embedded vector | Weak point hit zone data |
| `m_MoveType` | `CBaseEntity` | `MoveType_t` (uint8) | Movement mode; always 9 (MOVETYPE_STEP) for CNPC_Trooper -- see m_MoveType entry |
| `m_lifeState` | `CBaseEntity` | `uint8` | Life state; see m_lifeState entry |
| `m_iHealth` | `CBaseEntity` | `int32` | Current health |
| `m_iMaxHealth` | `CBaseEntity` | `int32` | Max health |
| `m_iTeamNum` | `CBaseEntity` | `uint8` | Team affiliation (2=Amber, 3=Sapphire) |
| `m_fFlags` | `CBaseEntity` | `uint32` | **Confirmed NOT readable from demos for CNPC_Trooper.** Probe returned `None` for all 2197 entities across a full match. Despite having `MNetworkEnable` in CBaseEntity.h, the field is not present in the network stream for this entity class. Ruled out as a discriminator. |
| `m_vecVelocity` | `CBaseEntity` | `CNetworkVelocityVector` | Current velocity -- **MNetworkUserGroup = "LocalPlayerExclusive"; likely not broadcast for NPCs in demos** |
| `m_spawnflags` | `CBaseEntity` | `uint32` | Spawn-time flags. **Always 1028 (0x404) for all CNPC_Trooper entities -- no variation across 2197 CREATE events. Cannot discriminate any lifecycle phase.** |
| Body component position fields | `CBodyComponent` (nested) | cell+vec pairs | World position; see position entry |

### Server-only fields (NOT readable from demos)

| Field | Declared on | Reason |
|-------|-------------|--------|
| `m_hSpawnWaveController` | `CNPC_Trooper` | No `MNetworkEnable`; server-side wave coordination handle only |
| `m_hTrooperSpawnPoint` | `CNPC_Trooper` | No `MNetworkEnable`; server-side spawn point reference |
| `m_iLaneSlot` | `CNPC_Trooper` | No `MNetworkEnable` |
| `m_hNearDeathModifier` | `CNPC_Trooper` | No `MNetworkEnable` |
| `m_nActualMoveType` | `CBaseEntity` | No `MNetworkEnable` (only `m_MoveType` above it is annotated) |
| `m_hGroundEntity` | `CBaseEntity` | Has `MNetworkEnable` but also `MNetworkUserGroup = "Player"` -- only sent to owning player client; likely unavailable for CNPC_Trooper in demos |
| `m_nGroundBodyIndex` | `CBaseEntity` | Same `MNetworkUserGroup = "Player"` restriction as `m_hGroundEntity` |
| `m_NPCState` shadow fields (`m_nPreModifierNPCState`, `m_IdealNPCState`) | `CAI_BaseNPC` | No `MNetworkEnable` |
| All CAI navigator / pathfinder / scheduler / sensor fields | `CAI_BaseNPC` | No `MNetworkEnable` |
| `m_vecSpawnOrigin` | `CAI_CitadelNPC` | No `MNetworkEnable` |

### Key negative findings from lifecycle research

- **No wave ID field exists on the entity.** `m_hSpawnWaveController` is server-only. `CInfoTrooperBossSpawn` is not a replicated entity class (no schema dump, not in the server schema directory). Wave membership must be computed in the parser from spawn timing -- the current `assign_wave` approach is architecturally correct.
- **No spawn group field.** `m_hSpawnGroup` / `m_iSpawnedFromGroupID` are not present in the `CBaseEntity` schema dump. Source 2 spawn groups are not networked in demo data.
- **No dormant / enabled / active flag.** There is no `m_bEnabled`, `m_bActive`, or `m_bDormant` anywhere in the `CNPC_Trooper` inheritance chain. The engine does not network a "waiting for next wave" boolean.
- **`NPC_STATE_INERT` is a combat frozen state, not a spawn-waiting state.** A recycled creep at base is expected to be `NPC_STATE_IDLE (1)`, not INERT. INERT is used by the engine for stunned or near-death-animation-suppressed NPCs mid-lane. Using INERT as a pre-spawn guard causes false negatives (active creeps being incorrectly treated as pre-spawn).
- **`m_MoveType` is confirmed always 9 (MOVETYPE_STEP) for CNPC_Trooper and cannot distinguish cage-phase from lane-phase.** The MOVETYPE_NONE hypothesis is disproven.
- **`m_fFlags` is confirmed NOT present in the demo network stream for CNPC_Trooper.** The FL_ONGROUND approach is eliminated. The probe returned `None` for all 2197 entities -- the field has `MNetworkEnable` in the schema but is not actually networked for this entity class in practice.
- **`m_spawnflags` is always 1028 (0x404) with no variation.** Ruled out as a discriminator.
- **The confirmed discriminator for cage entities is `m_iHealth == 1`.** Cage/zipline carrier entities have health=1 throughout their lifecycle. Actual fighting creeps have health >= ~350. Use this to filter cage entities at wave registration time.
- **CNPC_Trooper entities are never DELETE'd during match play.** Zero DELETE events were observed across a full match. Entities cycle via life_state transitions indefinitely. The `handle_creep_delete` path is not called during normal match operation.
- **npc_state=10 (DYING) and npc_state=12 (DEAD) are confirmed Deadlock-specific extensions.** Both co-occur exactly with their corresponding life_state values (10 with life_state=1, 12 with life_state=2). Constants `NPC_STATE_DYING_CITADEL` and `NPC_STATE_DEAD_CITADEL` are defined in `constants.rs`.
