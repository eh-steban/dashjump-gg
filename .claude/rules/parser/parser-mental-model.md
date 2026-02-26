# Parser Mental Model: Demo File Architecture

## Core Concept

**Deadlock demo files record from the pre-game lobby, not from match start.**

This single fact causes cascading implications for timeline handling, data indexing, and frame reconciliation throughout the entire system.

---

## Timeline Architecture

### The Problem: Two Time Systems

Demo files contain data in a **demo time** coordinate system, but the game runs in a **game clock** coordinate system. They are not aligned.

```
DEMO STREAM                    GAME STATE
├── Frame 0 (demo_time=0)     Pre-game lobby starts
│   └── positions[0]          Ignored by game
│
├── Frame 1000                Waiting in lobby
│   └── positions[1000]       Ignored by game
│
├── Frame 5000                Hero select begins
│   └── positions[5000]       Ignored by game
│
├── Frame 8000                Laning phase starts ← game_clock = 0
│   └── positions[8000]       THIS is where game_clock=0
│
└── ...continued...
   └── positions[i]           game_clock = (demo_time - game_start_offset)
```

**Key insight:** There is an offset between array indices and game time.

### Reconciliation: game_start_marker

The parser extracts a `game_start_marker` from the demo file. This marker tells you:

```python
# game_start_marker is a frame number (demo time) where game clock = 0
game_start_marker = 8000  # Position in positions[] array

# To convert ANY position array index to actual game time:
def demo_index_to_game_clock(position_index: int, game_start_marker: int) -> float:
    return (position_index - game_start_marker) / FRAMES_PER_SECOND

# ✅ CORRECT
game_time = demo_index_to_game_clock(8000, 8000)  # = 0 seconds
game_time = demo_index_to_game_clock(8300, 8000)  # = 10 seconds (30 fps)

# ❌ WRONG
game_time = 8000 / 30  # Treating array index as time—off by ~266 seconds!
```

---

## Data Structure: positions[] Array

```rust
// positions[i] represents frame i of the DEMO STREAM, not game time
// positions[0] through positions[game_start_marker - 1] are pre-game noise
// positions[game_start_marker] onwards is actual match data

pub struct Position {
    entity_index: u32,
    x: f32,
    y: f32,
    z: f32,
}

// Accessing:
let frame_8000 = positions[8000];  // This is the START of the game
let frame_8300 = positions[8300];  // This is 10 seconds into the game
```

**Critical constraint:** Array indices represent demo frames, NOT game seconds.

---

## Common Mistakes & Why They Fail

### Mistake 1: Using array index as time

```python
# ❌ WRONG - treating index as time
for i in range(len(positions)):
    game_time_seconds = i / FRAMES_PER_SECOND
    record_position(game_time_seconds, positions[i])

# Result: Timeline is off by ~266 seconds for a typical game start
```

**Why it fails:** The demo file starts recording before the match begins. You're counting pre-game frames as game time.

**How to fix:**
```python
# ✅ CORRECT - reconcile against game_start_marker
for i in range(len(positions)):
    if i < game_start_marker:
        continue  # Skip pre-game
    game_time_seconds = (i - game_start_marker) / FRAMES_PER_SECOND
    record_position(game_time_seconds, positions[i])
```

### Mistake 2: Assuming game_clock from event timestamp

```python
# ❌ WRONG - trusting event timestamps as position indices
damage_event = {
    "game_clock": 1200,  # seconds into game
    "position_index": 1200  # array index
}
attacker_pos = positions[1200]  # This is pre-game frame, NOT game clock 1200!
```

**Why it fails:** Event timestamps are in game clock seconds, but positions array is indexed by demo frames.

**How to fix:**
```python
# ✅ CORRECT - convert game_clock to position_index
game_clock = 1200  # seconds
demo_frame_index = int(game_clock * FRAMES_PER_SECOND) + game_start_marker
attacker_pos = positions[demo_frame_index]
```

### Mistake 3: Synchronizing events with position snapshots

```python
# ❌ WRONG - assuming snapshot and event share same time reference
for event in damage_events:
    snapshot = positions[event.game_clock]  # MISMATCH!
    # event.game_clock is in seconds, positions[] is in demo frames
```

**How to fix:**
```python
# ✅ CORRECT - convert before indexing
for event in damage_events:
    demo_frame = int(event.game_clock * FRAMES_PER_SECOND) + game_start_marker
    snapshot = positions[demo_frame]
```

---

## Frame Timing Details

### Frame Rate
- Deadlock demos record at ~30 FPS
- Not guaranteed constant (may vary slightly)
- Parser should detect and log if inconsistent

### Frame 0 Semantics
```
positions[0] = First demo frame (pre-game lobby)
positions[0] ≠ Game clock 0
positions[game_start_marker] = Game clock 0
```

### Time Conversion Formula

```
game_clock_seconds = (position_index - game_start_marker) / FRAMES_PER_SECOND

position_index = int(game_clock_seconds * FRAMES_PER_SECOND) + game_start_marker
```

### Rounding/Quantization
When converting game_clock (float) to position_index:
```rust
// Use nearest frame, not truncate
let demo_frame = ((game_clock_seconds * FRAMES_PER_SECOND) + game_start_marker as f32).round() as usize;
```

---

## game_start_marker Extraction

### How Parser Determines It

The parser looks for the frame where:
1. Entities stabilize (stop despawning)
2. Player positions are assigned (not 0,0,0)
3. Team formation is stable (players stay in lanes)

Typically happens 240-300 frames after demo start (~8-10 seconds of lobby/hero select at 30 fps).

### What if game_start_marker is wrong?

Timeline will be offset by N frames. Example:
- If actual marker is 8000 but we use 7950
- Entire timeline shifts forward by 50 frames (~1.67 seconds)
- All timeline events are early

**Detection:** If team positions look wrong at 0:00 (e.g., heroes in wrong lanes), check marker extraction.

---

## Data Flow: From Demo to Backend

```
Demo File
    ↓
[Parser]
    ├── Extract positions[] array (all frames)
    ├── Extract game_start_marker
    ├── Extract damage events (with event.game_clock in seconds)
    └── Compress and send to backend

[Backend]
    ├── Receives positions[] with game_start_marker
    ├── Stores in S3 as-is (preserves full timeline)
    └── Creates indexed JSON:
        {
            "game_start_marker": 8000,
            "positions": [...],
            "damage_events": [...]
        }

[Frontend]
    ├── Loads indexed JSON
    ├── When user scrubs timeline to 2:30:
    │   └── Converts 150 seconds → demo_frame 13500
    │   └── Loads positions[13500]
    └── Displays hero positions at that moment
```

---

## Debugging: How to Verify Alignment

### Check 1: Visual Timeline Inspection
```python
# Should be near fountain/base at game start
start_frame = game_start_marker
positions_at_start = positions[start_frame]

# Should be scattered across map at 5 minutes
mid_frame = game_start_marker + int(300 * 30)
positions_at_mid = positions[mid_frame]
```

If positions are stationary or in wrong location, timeline is misaligned.

### Check 2: Event Synchronization
```python
# Pick a major event (first kill)
kill_event = damage_events[0]
game_clock = kill_event.time  # in seconds

# Convert to position index
demo_frame = int(game_clock * 30) + game_start_marker
position_at_kill = positions[demo_frame]

# Attacker position should be near victim
# If attacker is on opposite side of map, frame is wrong
```

### Check 3: Marker Extraction Logs
Parser should log:
```
INFO: Extracted game_start_marker=8000 (match begins at frame 8000)
INFO: Pre-game duration=8000 frames (~266 seconds)
```

If pre-game duration looks wrong (>15 min or <1 min), investigate.

---

## Architectural Implications

### 1. Storage Strategy
- Store `game_start_marker` alongside positions array
- Do NOT pre-filter to remove pre-game frames
- Do NOT shift indices (lose information)

### 2. Query Strategy
When UI asks for "position at game_clock=150":
- Convert 150 → demo frame index
- Look up positions[demo_frame]
- Do NOT try to slice array by time

### 3. Caching Strategy
Cache keys should include `game_start_marker`:
```
cache_key = f"match_{match_id}:positions:marker_{game_start_marker}"
```

---

## Edge Cases

### Edge Case 1: Very Long Pre-game
Some matches have 15+ minutes of lobby activity before laning phase.
- Parser correctly handles this (just extracts longer pre-game prefix)
- Backend/frontend should handle large position arrays
- Storage should not assume max_frames ≈ game_length

### Edge Case 2: Sudden Game Restart
If game restarts mid-match:
- Demo continues recording
- There will be TWO game_start_markers (or multiple)
- Current parser assumes single game_start_marker
- Document if multi-game support is needed

### Edge Case 3: Incomplete Demo
If demo file cuts off early:
- positions array is shorter than expected
- game_start_marker is still valid
- Timeline just stops
- Frontend should gracefully handle short arrays

---

## References in Codebase

| Location | Use |
|---|---|
| `parser/src/replay_parser.rs` | game_start_marker extraction logic |
| `backend/app/domain/models/match_analysis.py` | Position data structure |
| `backend/app/application/mappers/match_mapper.py` | Converts parser output to domain models |
| `frontend/src/domain/timeline.ts` | Time conversion utilities |
| `frontend/src/components/matchAnalysis/MatchTimeViewer.tsx` | Reads game_start_marker for timeline scrubbing |

---

## Summary: The Mental Model

**Demo files = raw chronological recording, starting before match**

- Frame 0 = pre-game lobby
- Frame N = game start (where N = game_start_marker)
- positions[i] always corresponds to demo frame i
- game_clock always in seconds from match start
- Conversion between them is mandatory, not optional

**Core rule:** Every time you index into positions[], verify you're using demo frames, not game time.

**See also:** `private/learnings.md` — "Demo Timeline Offset: Reconciliation Pattern" for the cross-project summary.
