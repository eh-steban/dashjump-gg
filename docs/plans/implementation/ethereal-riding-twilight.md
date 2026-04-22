# Lane Creep Position Tracking Feature Plan

## Overview

Add lane creep wave tracking to measure **map pressure** throughout matches. The data flows from parser → backend (with pressure calculation) → frontend (wave indicators on minimap).

**Prerequisites:**
1. Comprehensive refactor of parser's monolithic `replay_parser.rs` (875 lines) into modular tracker architecture
2. Fix lane color tracking (capture at lane lock time, not player discovery time)

---

## User Requirements Summary

| Requirement | Decision |
|-------------|----------|
| Data granularity | Per-lane wave aggregates (centroid + count per tick) |
| Creep types | Track count only (no type breakdown - can't identify creep type yet) |
| Frontend visualization | Wave indicators (icon per wave showing position/size) |
| Pressure attribution | Player pushed lane (killed enemy creeps) OR player with wave |
| Refactor scope | Comprehensive - full tracker pattern |

---

## Data Structure

### Parser Output
```json
{
  "creep_waves": {
    "1": [  // lane_id -> per-tick array
      {"x": 1234, "y": 5678, "count": 4, "team": 2},
      null,  // No wave this tick
      ...
    ],
    "4": [...],
    "6": [...]
  }
}
```

### Backend Transform (Pressure Calculation)
```json
{
  "lane_pressure": {
    "1": [
      {"pressure": 0.7, "team": 2, "attributed_players": [0, 1], "wave_count": 4},
      ...
    ]
  }
}
```

**Pressure formula:**
- 0.0 = wave at own base
- 1.0 = wave at enemy base
- Derived from wave Y-coordinate relative to map bounds

**Attribution rules:**
1. Player is within proximity of wave (1500 world units)
2. Future: Player recently killed enemy creeps in that lane

---

## Phase 0: Lane Color Fix (Prerequisite)

### Problem

`zipline_lane_color` is captured at player discovery time ([replay_parser.rs:516](parser/src/replay_parser.rs#L516)) but the lane number is captured later when lane swap locks ([replay_parser.rs:424-428](parser/src/replay_parser.rs#L424-L428)). The lane color may not be finalized at discovery time.

### Solution

Update `check_and_update_lane_lock()` to also capture `zipline_lane_color`:

```rust
// In check_and_update_lane_lock (replay_parser.rs:409-444)
for player in &mut self.players {
    if player.lobby_player_slot == lobby_slot {
        // Existing: Update lane
        player.lane = entity
            .get_value(&ASSIGNED_LANE_KEY)
            .filter(|&v| v != 0)
            .or_else(|| entity.get_value(&ORIGINAL_LANE_ASSIGNMENT_KEY))
            .unwrap_or(999999);

        // NEW: Also update lane color at lock time
        player.zipline_lane_color = entity
            .get_value(&ZIPLINE_LANE_COLOR_KEY)
            .unwrap_or(player.zipline_lane_color);  // Keep existing if not found

        break;
    }
}
```

### Files Modified
- `parser/src/replay_parser.rs` - Update `check_and_update_lane_lock()`

### Checkpoint 0
- [ ] Parse a replay, verify `zipline_lane_color` values are correct
- [ ] Compare before/after to ensure change is correct
- [ ] Commit changes

---

## Phase 1: Parser Refactor

### Target Structure
```
parser/src/
├── domain/
│   ├── mod.rs
│   ├── player.rs          # Player, PlayerPosition (move from replay_parser.rs:54-73)
│   ├── boss.rs            # BossSnapshot (move from replay_parser.rs:75-90)
│   ├── damage.rs          # DamageRecord (move from replay_parser.rs:241-264)
│   └── creep.rs           # NEW: CreepWaveSnapshot
├── entities/
│   ├── mod.rs
│   ├── constants.rs       # All entity hashes (move from replay_parser.rs:297-324)
│   └── custom_id.rs       # get_custom_id() (move from replay_parser.rs:485-546)
├── tracking/
│   ├── mod.rs
│   ├── boss_tracker.rs    # BossTracker (move from replay_parser.rs:93-230)
│   ├── position_tracker.rs # Position collection logic
│   ├── damage_tracker.rs  # Damage handling logic
│   └── creep_tracker.rs   # NEW: CreepTracker
├── utils/
│   ├── mod.rs
│   ├── entity_position.rs # get_entity_position() (move from replay_parser.rs:371-395)
│   └── steam_id.rs        # steamid64_to_accountid() (move from replay_parser.rs:326-336)
├── handlers/              # (unchanged)
├── demo/                  # (unchanged)
├── replay_parser.rs       # Slim coordinator (~200 lines)
├── main.rs
└── config.rs
```

### Refactor Steps (Incremental)

| Step | Action | Risk | Test Point |
|------|--------|------|------------|
| 1.1 | Create `domain/` module, move structs | Low | `cargo build` |
| 1.2 | Create `entities/` module, move constants | Low | `cargo build` |
| 1.3 | Create `utils/` module, move helpers | Low | `cargo build` |
| 1.4 | Create `tracking/`, move `BossTracker` | Medium | Parse replay, verify output |
| 1.5 | Remove `debug_print_stuff()` (116 dead lines) | Low | `cargo build` |
| 1.6 | Slim down `replay_parser.rs` | Medium | Full integration test |

### Checkpoint 1
- [ ] Parser compiles successfully
- [ ] Parse same replay as before, compare JSON output (should be identical)
- [ ] Review code organization with user
- [ ] Commit refactor changes

---

## Phase 2: CreepTracker Implementation (Parser)

### New Files

**`parser/src/domain/creep.rs`**
```rust
#[derive(Debug, Clone, Serialize)]
pub struct CreepWaveSnapshot {
    pub x: f32,
    pub y: f32,
    pub count: u32,
    pub team: u32,
}

pub type CreepWaveTimeline = HashMap<String, Vec<Option<CreepWaveSnapshot>>>;

#[derive(Debug, Clone, Serialize)]
pub struct CreepWaveData {
    pub waves: CreepWaveTimeline,
}
```

**`parser/src/tracking/creep_tracker.rs`**
```rust
pub struct CreepTracker {
    team_key: u64,
    lane_key: u64,
    trooper_hash: u64,
    wave_timeline: HashMap<i32, Vec<Option<CreepWaveSnapshot>>>,
    current_window: HashMap<(i32, u32), Vec<(f32, f32)>>,  // (lane, team) -> positions
}

impl CreepTracker {
    pub fn new() -> Self;
    pub fn is_creep_entity(&self, hash: u64) -> bool;
    pub fn record_creep(&mut self, entity: &Entity);
    pub fn finalize_window(&mut self, window_s: u32);
    pub fn get_output(&self) -> CreepWaveData;
}
```

### Integration Points

1. **`MyVisitor` struct**: Add `creep_tracker: CreepTracker` field
2. **`on_tick_end()`**: Call `creep_tracker.record_creep()` for creep entities
3. **Window rotation**: Call `creep_tracker.finalize_window(this_window)`
4. **`get_match_data_json()`**: Include `"creep_waves": self.creep_tracker.get_output()`

### Key Discovery: Lane Field

Creeps use `m_iLane` field (same as bosses). The `BossTracker` already has:
```rust
lane_key: fkey_from_path(&["m_iLane"]),
```

This provides authoritative lane assignment from game data.

### Testing the Parser

To test the parser in isolation:
1. Hit backend endpoint to get a demo file name: `GET /match/analysis/{match_id}` or check existing replays
2. Download replay to parser's replay directory
3. Call parser directly: `POST http://localhost:9000/parse` with demo URL
4. Verify `creep_waves` appears in response

### Checkpoint 2
- [ ] CreepTracker compiles
- [ ] Parse a replay, verify `creep_waves` in JSON output
- [ ] Verify wave counts look reasonable (4-6 creeps per wave typical)
- [ ] Verify lanes 1, 4, 6 (or whichever are valid) have data
- [ ] Commit CreepTracker implementation

---

## Phase 3: Backend Integration

### New Files

**`backend/app/domain/models/creep.py`**
```python
class CreepWaveSnapshot(SQLModel):
    x: float
    y: float
    count: int
    team: int

CreepWaveTimeline = dict[str, list[Optional[CreepWaveSnapshot]]]

class CreepWaveData(SQLModel):
    waves: CreepWaveTimeline
```

**`backend/app/domain/models/lane_pressure.py`**
```python
class LanePressureSnapshot(SQLModel):
    lane_id: int
    team: int
    pressure: float  # 0-1
    attributed_players: list[int]
    wave_x: float
    wave_y: float
    wave_count: int

LanePressureTimeline = dict[str, list[Optional[LanePressureSnapshot]]]

class LanePressureData(SQLModel):
    pressure: LanePressureTimeline
```

**`backend/app/domain/services/lane_pressure_service.py`**
```python
class LanePressureCalculator:
    @staticmethod
    def calculate_pressure(wave: CreepWaveSnapshot, lane_id: int) -> float:
        """
        Pressure = how far into enemy territory.
        Amber (team=2) pushes toward low Y (Sapphire base).
        Sapphire (team=3) pushes toward high Y (Amber base).
        """
        normalized_y = (wave.y - WORLD_MIN) / WORLD_SPAN
        if wave.team == 2:
            return 1.0 - normalized_y
        return normalized_y

    @staticmethod
    def attribute_players(
        wave: CreepWaveSnapshot,
        player_positions: list[PlayerPosition],
        proximity_threshold: float = 1500.0
    ) -> list[int]:
        # Return player slots within threshold of wave centroid
```

### Modified Files

| File | Changes |
|------|---------|
| `app/domain/models/match_analysis.py` | Add `creep_waves`, `lane_pressure` fields to `TransformedMatchData` |
| `app/domain/services/transform_match_data.py` | Call `LanePressureCalculator.process_creep_waves()` |

### Checkpoint 3
- [ ] Backend tests pass (`pytest`)
- [ ] Pressure calculation unit tests added and passing
- [ ] Fetch match analysis via API, verify `creep_waves` and `lane_pressure` in response
- [ ] Commit backend changes

---

## Phase 4: Frontend Visualization

### New Files

**`frontend/src/domain/creep.ts`**
```typescript
export interface CreepWaveSnapshot {
  x: number;
  y: number;
  count: number;
  team: number;
}

export interface ScaledCreepWave extends CreepWaveSnapshot {
  left: number;
  top: number;
  laneId: string;
}

export interface CreepWaveData {
  waves: Record<string, (CreepWaveSnapshot | null)[]>;
}
```

**`frontend/src/components/matchAnalysis/CreepWaveIndicator.tsx`**
- Renders single wave indicator
- Size scales with creep count
- Color by team (amber/sapphire)
- Tooltip shows details

**`frontend/src/components/matchAnalysis/CreepWaveLayer.tsx`**
- Manages all wave indicators for current tick
- Transforms world coords → minimap pixels
- Filters null snapshots

### Modified Files

| File | Changes |
|------|---------|
| `src/domain/matchAnalysis.ts` | Add `creep_waves`, `lane_pressure` to `ParsedMatchData` |
| `src/components/matchAnalysis/Minimap.tsx` | Add `<CreepWaveLayer>` after `<PlayerPositions>` |
| `src/pages/MatchAnalysis.tsx` | Pass `creep_waves` prop to Minimap |

### Checkpoint 4
- [ ] Frontend compiles (`npm run build`)
- [ ] Wave indicators visible on minimap
- [ ] Waves move as timeline scrubs
- [ ] Toggle visibility works (if implemented)
- [ ] Commit frontend changes

---

## Phase 5: Testing

### Parser Tests
```rust
// tests/tracking/creep_tracker_test.rs
#[test] fn test_empty_window_produces_no_waves();
#[test] fn test_wave_centroid_calculation();
#[test] fn test_multiple_lanes_tracked_separately();
```

### Backend Tests
```python
# tests/domain/services/test_lane_pressure_service.py
def test_amber_wave_at_sapphire_base_has_max_pressure();
def test_sapphire_wave_at_own_base_has_min_pressure();
def test_player_attribution_by_proximity();
def test_empty_waves_return_empty_pressure();
```

### Frontend Tests
```tsx
// tests/components/matchAnalysis/CreepWaveIndicator.test.tsx
it('displays creep count');
it('scales size with creep count');
it('uses correct team color');
```

### Checkpoint 5
- [ ] All parser tests pass (`cargo test`)
- [ ] All backend tests pass (`pytest`)
- [ ] All frontend tests pass (`npm test`)
- [ ] Commit test additions

---

## Phase 6: Observability

### Parser Logging
```rust
tracing::debug!("[creep_tracker] Skipping creep with invalid lane: {}", lane);
tracing::debug!("[creep_tracker] Finalized window {} with {} waves", window_s, count);
```

### Backend Logging
```python
logger.debug("Calculated pressure for lane %s team %s: %.2f", lane_id, team, pressure)
logger.info("Match %s: Processed %d creep wave snapshots", match_id, total_waves)
```

### Error Handling

| Scenario | Parser | Backend | Frontend |
|----------|--------|---------|----------|
| No creeps in lane | Empty timeline | Pass through | Show nothing |
| Invalid lane ID | Skip with debug log | Filter out | Ignore |
| Missing creep_waves | N/A | Skip pressure calc | Show nothing |

### Final Checkpoint
- [ ] End-to-end test: Parse replay → backend transform → frontend display
- [ ] Performance acceptable (no noticeable slowdown)
- [ ] Data size increase reasonable
- [ ] All tests pass
- [ ] Create PR for review

---

## Human-in-the-Loop Checkpoints Summary

| Phase | Checkpoint | Review Focus |
|-------|------------|--------------|
| 0 | Lane color fix | Verify `zipline_lane_color` correct in output |
| 1 | Parser refactor | Code organization, output unchanged |
| 2 | CreepTracker | Wave data structure, reasonable counts |
| 3 | Backend | Pressure calculation, API response shape |
| 4 | Frontend | Visual display on minimap |
| 5 | Testing | Test coverage, edge cases |
| Final | E2E | Full pipeline working |

Each checkpoint is a natural commit point. Feel free to pause after any checkpoint to review.

---

## Data Size Estimate

| Metric | Value |
|--------|-------|
| Waves per tick | 6 max (3 lanes × 2 teams) |
| Ticks per match | ~2000 (35 min) |
| Bytes per wave | ~60 JSON (no types) |
| **Total per match** | **~720 KB** |

This is acceptable given existing match data is 15-18 MB.

---

## Files Summary

### Parser (Create)
- `src/domain/mod.rs`
- `src/domain/player.rs`
- `src/domain/boss.rs`
- `src/domain/damage.rs`
- `src/domain/creep.rs`
- `src/entities/mod.rs`
- `src/entities/constants.rs`
- `src/entities/custom_id.rs`
- `src/tracking/mod.rs`
- `src/tracking/boss_tracker.rs`
- `src/tracking/position_tracker.rs`
- `src/tracking/damage_tracker.rs`
- `src/tracking/creep_tracker.rs`
- `src/utils/mod.rs`
- `src/utils/entity_position.rs`
- `src/utils/steam_id.rs`

### Parser (Modify)
- `src/replay_parser.rs` (reduce from 875 → ~200 lines, fix lane color)
- `src/main.rs` (update module imports)

### Backend (Create)
- `app/domain/models/creep.py`
- `app/domain/models/lane_pressure.py`
- `app/domain/services/lane_pressure_service.py`

### Backend (Modify)
- `app/domain/models/match_analysis.py`
- `app/domain/services/transform_match_data.py`

### Frontend (Create)
- `src/domain/creep.ts`
- `src/domain/lanePressure.ts`
- `src/components/matchAnalysis/CreepWaveIndicator.tsx`
- `src/components/matchAnalysis/CreepWaveLayer.tsx`

### Frontend (Modify)
- `src/domain/matchAnalysis.ts`
- `src/components/matchAnalysis/Minimap.tsx`
- `src/pages/MatchAnalysis.tsx`

---

## Risk Assessment

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Data size explosion | Low | Per-lane aggregates (not individual creeps) |
| Frontend performance | Low | Toggle visibility, skip null entries |
| Parser refactor breaks output | Medium | Incremental steps with test points |
| Creep entity identification | Low | `CNPC_TROOPER_ENTITY` already defined |
| Lane color still wrong | Low | Test with multiple replays |
