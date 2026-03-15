---
paths:
  - "parser/src/*.rs"
  - "parser/src/**/*.rs"
---
# Parser Mental Model: replay File Architecture

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

## total_match_time_s: Current Behavior and Known Rough Edge

The `total_match_time_s` field in the output JSON stores **replay-absolute time** (the raw replay clock second of the last frame processed), not the match duration in seconds. This is a known rough edge that will be addressed in the upcoming parser repo switch.

**To get match duration:** Use `positions.len()` directly.

**Approximation:** `total_match_time_s - match_start_time_s` approximates match duration in seconds.

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
- DELETE fires reliably on creep death for lane creeps (the lane is always in the demo recorder's interest scope).
- The same `entity_index` is **not reused** within a match.
- There is **no double-CREATE** across the zipline/in-lane transition — the entity is created once and updated via UPDATE events throughout its lifetime.

---

## References in Codebase

| Location | Use |
|---|---|
| `parser/src/replay_parser.rs` | match_start_time_s extraction logic |
| `backend/app/domain/models/match_analysis.py` | Position data structure |
| `backend/app/application/mappers/match_mapper.py` | Converts parser output to domain models |
| `frontend/src/domain/timeline.ts` | Time conversion utilities |
| `frontend/src/components/matchAnalysis/MatchTimeViewer.tsx` | Reads match_start_time_s for timeline scrubbing |
| `private/specs/citadel-messages-reference.md` | Citadel protobuf message catalog -- fields, IDs, product alignment |
| `private/specs/deadlock-api-haste-reference.md` | haste Visitor trait, subscription patterns, parse lifecycle |

---

## Summary: The Mental Model

**Parser output = match-relative arrays, starting at match second 0**

- positions[i] = match second i (positions[0] = match starts)
- boss.health_timeline[i] = match second i (aligned with positions)
- lane_creep_data.creeps[entity_idx][i] = match second i (aligned with positions)
- match_start_time_s is metadata -- not needed as a positions[] offset
- Use positions.len() for match duration (not total_match_time_s -- see rough edge note above)

**Core rule:** positions[0] is match second 0. Index directly; no offset required.

**See also:** `private/learnings.md` — Drafts section for the timeline alignment change note.
