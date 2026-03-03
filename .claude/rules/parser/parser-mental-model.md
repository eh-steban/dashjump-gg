# Parser Mental Model: replay File Architecture

## Core Concept

**Deadlock replay files record from the pre-match lobby, not from match start.**

This single fact causes cascading implications for timeline handling, data indexing, and frame reconciliation throughout the entire system.

---

## Timeline Architecture

### The Problem: Two Time Systems

replay files contain data in a **replay time** coordinate system, but the match runs in a **match clock** coordinate system. They are not aligned.

```
replay STREAM                    match STATE
├── Time window 0 (replay_time=0)     Pre-match lobby starts
│   └── positions[0]          Ignored by match
|
├── Time window 5                Waiting in lobby
│   └── positions[5]       Ignored by match
│
├── Time window 8                Laning phase starts ← match_clock = 0
│   └── positions[8]       THIS is where match_clock=0
│
└── ...continued...
   └── positions[i]           match_clock = (replay_time - match_start_offset)
```

**Key insight:** There is an offset between array indices and match time.

---

## Reconciliation: match_start_time_s

The parser extracts a `match_start_time_s` from the replay file. It does this by firing a entity update event updating `CCitadelGameRulesProxy` entity. The `m_pGameRules.m_flGameStartTime` field is updated and says how many seconds into the recording the match starts. Then we take that value, round down, and set `match_start_time_s`. We round down so if game start time is 7.75, we set start time to 7, player positions will be good enough, and then time resume from there.

### What if match_start_time_s is wrong?

Timeline will be offset by N frames. Example:
- If actual marker is 8 but we use 7
- Entire timeline shifts forward by 1 second
- All timeline events are early

**Detection:** If team positions look wrong at 0:00 (e.g., heroes in wrong lanes), check marker extraction.

---

## Data Structure: positions[] Array

```rust
// positions[i] represents time window i of the replay STREAM, not match time
// positions[0] through positions[match_start_time_s - 1] are pre-match noise
// positions[match_start_time_s] onwards is actual match data

pub struct Position {
    entity_index: u32,
    x: f32,
    y: f32,
    z: f32,
}

// Accessing:
let time window_8 = positions[8];  // This is the START of the match
let time window_10 = positions[10];  // This is 10 seconds into the match
```

**Critical constraint:** Array indices represent replay time windows, NOT match seconds.

---

## Common Mistakes & Why They Fail

### ****STUB**** Mistake 1: Explanation

```python
# ❌ WRONG - **explanation**
for i in range(len(positions)):
    match_time_seconds = i / FRAMES_PER_SECOND
    record_position(match_time_seconds, positions[i])

# Result: Timeline is off by ~266 seconds for a typical match start
```

**Why it fails:** The replay file starts recording before the match begins. You're counting pre-match time windows as match time.

**How to fix:**
```python
# ✅ CORRECT - **explanation**
for i in range(len(positions)):
    if i < match_start_time_s:
        continue  # Skip pre-match
    match_time_seconds = (i - match_start_time_s) / FRAMES_PER_SECOND
    record_position(match_time_seconds, positions[i])
```

---

## Time window Details

### Time window 0 Semantics
```
positions[0] = First replay time window (pre-match lobby)
positions[0] ≠ match clock 0
positions[match_start_time_s] = match clock 0
```

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
    ├── Receives positions[] with match_start_time_s
    ├── Stores in S3 as-is (preserves full timeline)
    └── Creates indexed JSON:
        {
            "match_start_time_s": 8,
            "positions": [...],
            "damage_events": [...]
        }

[Frontend]
    ├── Loads indexed JSON
    ├── When user scrubs timeline to 2:30:
    │   └── replay_time_window 150
    │   └── Loads positions[150]
    └── Displays hero positions at that moment
```

---

## Debugging: How to Verify Alignment

### **STUB** Check 1: Example name
```python
# Should be near fountain/base at match start
start_frame = match_start_time_s
positions_at_start = positions[start_frame]

# Should be scattered across map at 5 minutes
mid_frame = match_start_time_s + int(300 * 30)
positions_at_mid = positions[mid_frame]
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

## References in Codebase

| Location | Use |
|---|---|
| `parser/src/replay_parser.rs` | match_start_time_s extraction logic |
| `backend/app/domain/models/match_analysis.py` | Position data structure |
| `backend/app/application/mappers/match_mapper.py` | Converts parser output to domain models |
| `frontend/src/domain/timeline.ts` | Time conversion utilities |
| `frontend/src/components/matchAnalysis/MatchTimeViewer.tsx` | Reads match_start_time_s for timeline scrubbing |

---

## Summary: The Mental Model

**replay files = raw chronological recording, starting before match**

- Time window 0 = pre-match lobby
- Time window N = match start (where N = match_start_time_s)
- positions[i] always corresponds to replay time window i
- match_clock always in seconds from match start

**Core rule:** Every time you index into positions[], verify you're using the correct offset from `match_start_time_s`.

**See also:** `private/learnings.md` — "replay Timeline Offset: Reconciliation Pattern" for the cross-project summary.
