---
paths:
  - "parser/src/*.rs"
  - "parser/src/**/*.rs"
---
# Parser Mental Model: replay File Architecture

## Module Structure

```
parser/
├── src/
│   ├── main.rs               # Axum server setup, module registration
│   ├── config.rs             # Configuration constants
│   ├── replay_parser.rs      # Core parsing coordinator (~400 lines)
│   │
│   ├── domain/               # Data Structures (pure, serializable)
│   │   ├── player.rs         # Player, PlayerPosition
│   │   ├── boss.rs           # BossSnapshot
│   │   ├── damage.rs         # DamageRecord
│   │   └── creep.rs          # CreepSnapshot, LaneCreepData, WaveMeta
│   │
│   ├── entities/             # Entity Identification
│   │   └── constants.rs      # Entity hashes, field keys (fkey_from_path)
│   │
│   ├── tracking/             # Stateful Trackers
│   │   ├── boss_tracker.rs   # BossTracker (spawn/despawn lifecycle)
│   │   └── creep_tracker/    # CreepTracker (per-creep lane tracking)
│   │       ├── mod.rs        #   implementation
│   │       └── tests.rs      #   unit tests
│   │
│   ├── utils/                # Pure Helper Functions
│   │   ├── entity_position.rs # get_entity_position()
│   │   └── steam_id.rs       # steamid64_to_accountid()
│   │
│   ├── handlers/             # HTTP Route Handlers
│   │   ├── check_demo.rs
│   │   └── parse_demo.rs
│   │
│   └── demo/                 # Demo File Operations
│       ├── downloader.rs
│       └── decompressor.rs
│
├── Cargo.toml
├── Dockerfile
└── docker-compose.yaml
```

---

## Core Concept

**Deadlock replay files record from the pre-match lobby, not from match start.**

This single fact causes cascading implications for timeline handling, data indexing, and frame reconciliation throughout the entire system.

**The parser filters pre-match frames and outputs match-relative arrays.** All output arrays (positions, boss health timeline, creep wave timeline) start at index 0 = match second 0. Consumers do not need to apply a `match_start_time_s` offset to index into positions[].

---

## Timeline Architecture

### The Problem: Two Time Systems

replay files contain data in a **replay time** coordinate system, but the match runs in a **match clock** coordinate system. They are not aligned.

```
replay STREAM                    PARSER BEHAVIOR             OUTPUT
├── Time window 0 (replay_time=0)  [discarded - pre-match]   —
│   Pre-match lobby starts
|
├── Time window 5                  [discarded - pre-match]   —
│   Waiting in lobby
│
├── Time window 8                  [emitted as positions[0]] positions[0]
│   match starts ← match_clock = 0
│
├── Time window 9                  [emitted as positions[1]] positions[1]
│   match_clock = 1
│
└── ...continued...
    Time window N                  [emitted as positions[N-8]] positions[N-8]
    match_clock = N - match_start_time_s
```

**Key insight:** The parser discards pre-match frames. Output array indices map directly to match seconds (positions[0] = match second 0).

---

## Reconciliation: match_start_time_s

The parser extracts a `match_start_time_s` from the replay file. It does this by firing an entity update event updating the `CCitadelGameRulesProxy` entity. The `m_pGameRules.m_flGameStartTime` field is updated and says how many seconds into the recording the match starts. The value is rounded to the nearest integer (`f32::round()`). For example, if game start time is 7.75, `match_start_time_s` is set to 8.

This value is used internally to gate frame emission (pre-match frames are discarded) and is exported as metadata in the output JSON. It is no longer needed by consumers to offset into positions[].

### What if match_start_time_s is wrong?

Timeline will be offset by N frames. Example:
- If actual marker is 8 but we use 7
- Entire timeline shifts forward by 1 second
- All timeline events are early

**Detection:** If team positions look wrong at 0:00 (e.g., heroes in wrong lanes), check marker extraction.

---

## Data Structure: positions[] Array

```rust
// positions[i] represents match second i (0-indexed from match start)
// positions[0] = match clock 0 (match starts)
// positions.len() = total match duration in seconds
// Pre-match frames are discarded by the parser before populating this array

pub struct Position {
    entity_index: u32,
    x: f32,
    y: f32,
    z: f32,
}

// Accessing:
let match_start = &positions[0];   // Match second 0
let two_minutes = &positions[120]; // Match second 120 (2:00 into match)
```

**Critical constraint:** Array indices are match-relative. positions[0] = match second 0. No offset required.

---

## Common Mistakes & Why They Fail

### Mistake 1: Applying match_start_time_s as an offset on the consumer side

```python
# ❌ WRONG - applying the old offset pattern; positions[] is already match-relative
for i in range(len(positions)):
    if i < match_start_time_s:
        continue  # Skipping entries that don't exist — positions[0] IS match second 0
    match_time_seconds = i - match_start_time_s
    record_position(match_time_seconds, positions[i])

# Result: Skips the first match_start_time_s seconds of real match data
```

**Why it fails:** The parser already discards pre-match frames. positions[0] IS match second 0. Applying the offset again skips real match data.

**How to fix:**
```python
# ✅ CORRECT - index directly; positions are already match-relative
for i, position_frame in enumerate(positions):
    match_time_seconds = i  # i == match second directly
    record_position(match_time_seconds, position_frame)
```

---

## Time window Details

### Time window 0 Semantics
```
positions[0] = match clock 0 (match starts)
positions[i] = match clock i (i seconds into the match)
positions.len() = total match duration in seconds
```
The parser discards all frames before `match_start_time_s` before appending to positions[].

---

## match_duration_s

The parser outputs `match_duration_s` -- the final value of the internal `current_match_second` counter after parsing completes. This is a match-relative value (0-indexed from match start). It should agree with `positions.len()` and `damage.len()`. If it doesn't, that's a signal to investigate the parser's tick-to-second boundary logic.

**This field is immutable after the parser sets it.** The backend passes it through to the frontend without manipulation.

---

## Data Flow: From replay to Backend

```
replay File
    ↓
[Parser]
    ├── Extract positions[] array (all time windows)
    ├── Extract match_start_time_s
    ├── Extract damage events
    └── Compress and send to backend

[Backend]
    ├── Receives positions[] (match-relative; positions[0] = match second 0)
    ├── Stores in S3 as-is
    └── Creates indexed JSON:
        {
            "match_start_time_s": 8,  // metadata only -- not an index offset
            "positions": [...],
            "damage_events": [...]
        }

[Frontend]
    ├── Loads indexed JSON
    ├── When user scrubs timeline to 2:30:
    │   └── match_second = 150
    │   └── Loads positions[150] directly (no offset calculation needed)
    └── Displays hero positions at that moment
```

---

## Debugging: How to Verify Alignment

### Check 1: Verify timeline alignment
```python
# positions[0] should show heroes near fountain/base at match start
positions_at_start = positions[0]

# positions[300] should show heroes scattered across map at 5 minutes
positions_at_5min = positions[300]
```

If positions are stationary or in wrong location, timeline is misaligned.

---

## Architectural Implications

### 1. Caching Strategy
Cache keys should include `match_start_time_s`:
```
cache_key = f"match_{match_id}:positions:marker_{match_start_time_s}"
```

---

## Edge Cases

### Edge Case 1: Incomplete replay
If replay file cuts off early:
- positions array is shorter than expected
- match_start_time_s is still valid
- Timeline just stops
- Frontend should gracefully handle short arrays

---

## Entity Field Lookup Tools

**Core rule: Never search proto files for game-engine entity field enums.**

Deadlock protobufs (`valveprotos-rs`) only cover network messages — game events and netmessages. Game-engine field types and enum values (e.g., `m_NPCState` on `CNPC_Trooper`, `m_lifeState`) are embedded in the demo's SendTables, not in any `.proto` file.

### The Correct Lookup Path

1. **`uniquetypes` tool** (`haste` repo at `tools/uniquetypes/src/main.rs`) — Run against a `.dem` file to extract all unique type name identifiers from the SendTables. This reveals what Rust type backs a given field name (e.g., `ELifeState`, `ENPCState`).

2. **`haste-inspector`** (repo: `blukai/haste-inspector`) — Interactive browser for entity fields in a live demo. Browse `CNPC_Trooper` directly to see field names, types, and current values.

3. **`dezlock-dump`** — Runtime schema extractor that injects into a running Deadlock process and outputs `_all-enums.hpp` with all scoped enums and their integer values. Requires the game running; not usable in devcontainer.

4. **`SteamDatabase/GameTracking-Deadlock`** — Community-maintained game file tracking on GitHub. Does NOT contain schema dumps; mainly tracks `.vpk` asset changes. Do not rely on this for enum values.

### Reference Pattern

The `lifestate.rs` example in haste (`LIFE_ALIVE=0, LIFE_DEAD=2`) shows the format. Observed `m_NPCState` values 2, 6, 12 on `CNPC_Trooper` suggest a Deadlock-specific enum — value 12 rules out the standard Source 2 `NPC_STATE` (0–7 range), meaning these must be extracted via SendTables tooling.

**Cross-project summary:** `private/learnings.md` — "Deadlock Entity Field Enums Are Not in Protobufs"

---

## Creep Tracking Architecture

### Wave Grouping: Spawn-Time Over Spatial Clustering

The `CreepTracker` assigns `wave_id` at CREATE time using spawn-time grouping, not spatial clustering. Two creeps in the same `(lane, team)` that spawn within 5 seconds share a `wave_id = "lane_team_spawnsec"`.

**Why spatial clustering was abandoned:**

1. **False second wave during zipline touchdown.** Deadlock creeps ride a zipline from spawn to lane. During the ~1–2 second touchdown window the 4 creeps in a wave spread out before converging into march formation. A 1000-unit cluster threshold treated that spread as two separate waves. The second cluster disappeared once creeps converged, and the real next wave then got index 0 — producing unstable, non-correlatable wave identities across time.

2. **Dead creep semantics.** Spatial re-clustering every second shifted wave keys after deaths reduced a cluster, making it impossible to correlate "wave X at second 50" with "wave X at second 60".

Spawn-time grouping fixes both: wave identity is assigned once and never changes.

### Entity Lifecycle in Deadlock Demos (Lane Creeps)

- CREATE fires close to actual spawn (within 1–2 ticks). Pre-match creeps have `m_iLane == 0` and are gated out before the tracker sees them (gated by `match_started` in `replay_parser.rs::on_entity`).
- **DELETE does NOT fire reliably on creep death.** Some entities persist in the demo with their last-alive state (`life_state=ALIVE`, `npc_state=COMBAT`) for up to several minutes after dying in game. DELETE eventually fires -- often triggered by a game event like a guardian being destroyed, not the creep's own death. Do not rely on DELETE as the primary death signal.
- **Entity indices ARE reused within a match.** Deadlock reuses entity slots for new waves (DEAD→ALIVE in-place recycling). The `CreepTracker` detects this via a `life_state == LIFE_DEAD → life_state == LIFE_ALIVE` transition in `handle_creep_update` and re-assigns a new `wave_id`.
- There is **no double-CREATE** across the zipline/in-lane transition -- the entity is created once and updated via UPDATE events throughout its lifetime.

### CNPC_Trooper State Machine

Confirmed observable states (from probe + live testing):

| `m_NPCState` | `m_lifeState` | Meaning | Render? |
|---|---|---|---|
| IDLE (1) | ALIVE (0) | Active in lane, no target | Yes |
| ALERT (2) | ALIVE (0) | Spotted enemy, approaching | Yes |
| COMBAT (3) | ALIVE (0) | Actively fighting | Yes |
| INERT (6) | ALIVE (0) | Stunned mid-lane (or pre-spawn) | Yes -- stunned ≠ dead |
| INERT (6) | DYING (1) or DEAD (2) | Recycling back to base | No |
| DYING_CITADEL (10) | DYING (1) | Death animation playing | No |
| DEAD_CITADEL (12) | DEAD (2) | Dead, waiting at base for next wave | No |
| INIT (0) | any | Pre-spawn, uninitialized | No |
| INVALID (-1) | any | Sentinel / unknown | No |

**Ghost creep mechanism and fix (March 2026):** When a creep dies mid-lane it transitions to `NPC_STATE_INERT + LIFE_ALIVE` -- the initial post-death recycling state. `m_iHealth` reaches 0 in the same tick as the fatal blow, before the AI state machine catches up. Adding `health > 0` to the whitelist catches this: a stunned-but-alive creep has `health > 0`; a dead creep stuck in `INERT + ALIVE` has `health == 0`. Ghost creeps consistently disappear at wave spawn boundaries (the DEAD→ALIVE recycling tick). Entities that never receive any death state update (rare; observed near objective kills) may briefly appear frozen until DELETE fires -- this is a Valve demo edge case affecting a small number of ticks.

### Whitelist Approach for Snapshot Suppression

**Rule: use a whitelist, not a blacklist, for entity visibility.**

A blacklist (suppress known-dead states) fails silently whenever an undocumented or intermediate state is encountered. A whitelist (emit only when in known-alive states) fails safely -- unrecognized states become invisible rather than visible.

```rust
// Whitelist: emit only when confirmed alive in lane
let is_active = life_state == LIFE_ALIVE
    && health > 0
    && matches!(npc_state, NPC_STATE_IDLE | NPC_STATE_ALERT | NPC_STATE_COMBAT | NPC_STATE_INERT);
```

`NPC_STATE_INERT` is included because it covers stunned creeps (alive, not fighting) AND cage entities traveling the zipline (both legitimate render cases). The `life_state == LIFE_ALIVE` guard excludes INERT entities that are dead-and-recycling. The `health > 0` guard catches the gap where a freshly-dead creep has not yet exited ALIVE+INERT but has already had health zeroed by the damage system -- see ghost creep note above.

---

## References in Codebase

| Location | Use |
|---|---|
| `parser/src/replay_parser.rs` | match_start_time_s extraction logic |
| `backend/app/domain/models/match_analysis.py` | Position data structure |
| `backend/app/application/mappers/match_mapper.py` | Converts parser output to domain models |
| `frontend/src/domain/timeline.ts` | Time conversion utilities |
| `frontend/src/components/matchAnalysis/MatchTimeViewer.tsx` | Reads match_start_time_s for timeline scrubbing |
| `private/specs/citadel-messages-reference.md` | Citadel user message catalog -- fields, IDs, product alignment |
| `private/specs/citadel-messages-supplemental.md` | Low-alignment message namespaces (ECitadelGameEvents) |
| `private/specs/entity-fields-reference.md` | Entity field semantics, gotchas, deprecated fields |
| `private/specs/entity-fields-supplemental.md` | Background-context entity fields (m_nPlatformType, m_MoveType) |
| `private/specs/deadlock-api-haste-reference.md` | haste Visitor trait, subscription patterns, parse lifecycle, haste-inspector |

---

## Summary: The Mental Model

**Parser output = match-relative arrays, starting at match second 0**

- positions[i] = match second i (positions[0] = match starts)
- boss.health_timeline[i] = match second i (aligned with positions)
- lane_creep_data.creeps[entity_idx][i] = match second i (aligned with positions)
- match_start_time_s is metadata -- not needed as a positions[] offset
- match_duration_s = final value of internal match-second counter (should equal positions.len())

**Core rule:** positions[0] is match second 0. Index directly; no offset required.
