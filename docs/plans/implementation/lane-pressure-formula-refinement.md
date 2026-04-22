# Lane Creep Tracking Refactor

## Context

The current creep wave system aggregates all creeps in a wave into a single centroid position. This has two problems:

1. **Bugs**: Creeps are not being removed from waves when they die, and a false "second wave" appears when the first wave lands from the zipline. The spatial clustering algorithm is the likely culprit -- proximity thresholds are unreliable during the zipline-to-lane transition when creeps briefly spread out.
2. **Limitations**: Centroid aggregation loses per-creep information needed for accurate player attribution, meaningful pressure calculations, and fine-grained minimap display.

**Goals:**
- Track each lane creep individually (own timeline in parser output)
- Fix both bugs (double-wave and dead-creep-not-removed)
- Compute lane pressure based on distance to the nearest alive enemy lane objective using the creep's assigned lane (not spatial proximity), scaled by `alive_creep_count × 0.25`
- Attach nearby player IDs to each per-second creep snapshot (replaces wave-level attribution)
- Show a "pin" (last death position) as a minimap black-dot marker after a wave clears; pin persists until the next wave's last creep dies in that lane
- Remove centroid fields from backend lane pressure output (individual creep positions already in LaneCreepData)

Branch: `feature/lane-creep-tracking-parser-refactor`

**Review workflow**: After testing passes for each phase, stop for user review before committing. Flow: implement → test → pause for review → commit → next phase.

---

## Scope

| Service  | Involved | Agent              |
|----------|----------|--------------------|
| Parser   | yes      | `rust-parser`      |
| Backend  | yes      | `backend-python`   |
| Frontend | yes      | `frontend-react`   |

---

## Acceptance Criteria

Feature is done when ALL of the following are true from a user/product perspective:

- [x] Each lane creep has its own per-second timeline in parser output (no centroid aggregation)
- [x] Double-wave bug is gone -- no false second wave appears during zipline-to-lane transition
- [x] Dead creeps are removed from active tracking -- dead creep positions no longer appear after death
- [x] Lane pressure is computed per wave using `alive_creep_count × 0.25 × normalized_distance` (lane-assigned objective, not proximity-based) -- lane_length is dynamic (own frontline ↔ enemy frontline per second), not static guardian-to-guardian
- [x] Each per-second creep snapshot includes `nearby_players` for player attribution
- [ ] Minimap shows a black pin dot at the last death position of a wave after it clears, persisting until the next wave clears in that lane
- [x] Backend lane pressure output contains no centroid fields
- [ ] All in-scope phase checkpoints complete and signed off by user

---

## Objective Order Per Lane (all lanes same)

| Priority | Parser Constant | Game Name |
|---|---|---|
| 1 (first target) | `CNPC_TROOPERBOSS_ENTITY` | Guardian |
| 2 | `CNPC_BOSS_TIER2_ENTITY` | Walker |
| 3 | `CNPC_BARRACKBOSS_ENTITY` | Base Guardian |
| 4 | `CCITADEL_DESTROYABLE_BUILDING_ENTITY` | Shrine |
| 5 (last target) | `CNPC_BOSS_TIER3_ENTITY` | Patron |

Add comments labeling each constant in `parser/src/entities/constants.rs`. Verify `CCITADEL_DESTROYABLE_BUILDING_ENTITY` hash is present in constants.rs; add it if missing.

---

## Critical Files

| Layer | File |
|---|---|
| Parser constants | `parser/src/entities/constants.rs` |
| Parser domain | `parser/src/domain/creep.rs` |
| Parser tracker | `parser/src/tracking/creep_tracker.rs` |
| Parser integration | `parser/src/replay_parser.rs` |
| Backend domain creep | `backend/app/domain/creep.py` |
| Backend domain boss | `backend/app/domain/boss.py` |
| Backend lane pressure | `backend/app/services/lane_pressure_service.py` |
| Backend match data service | `backend/app/services/match_data_service.py` |
| Backend analyze_match | `backend/app/application/use_cases/analyze_match.py` |
| Frontend domain creep | `frontend/src/domain/creep.ts` |
| Frontend domain pressure | `frontend/src/domain/lanePressure.ts` |
| Frontend creep layer | `frontend/src/components/matchAnalysis/CreepWaveLayer.tsx` |
| Frontend creep indicator | `frontend/src/components/matchAnalysis/CreepWaveIndicator.tsx` |
| Frontend match analysis page | `frontend/src/pages/MatchAnalysis.tsx` |
| Frontend API client | `frontend/src/api/MatchAnalysis.ts` |

---

## Phase A -- Parser (rust-parser agent)

### A0. Investigate bugs first

Before any refactor, add temporary debug logging:
- On CREATE: log `entity_index`, `lane`, `team`, `y`, `current_sec`
- On DELETE: log same fields
- Parse the first 2 minutes of a real replay and inspect logs to confirm:
  - Whether creeps are the same entity on the zipline vs. in lane (does CREATE fire twice?)
  - Whether DELETE fires correctly on death

Document findings before proceeding. The wave grouping refactor should fix the double-wave bug regardless, but death tracking may need a targeted fix if DELETE is not firing correctly.

### A1. Label constants (`parser/src/entities/constants.rs`)

Add inline comments to all boss entity hash constants documenting their game role and lane objective priority order as shown in the table above. Add `CCITADEL_DESTROYABLE_BUILDING_ENTITY` if not present.

### A2. New domain types (`parser/src/domain/creep.rs`)

Replace `CreepWaveSnapshot` / `CreepWaveTimeline` / `CreepWaveData` with:

```rust
/// Per-second snapshot for one individual creep while it is alive.
#[derive(Debug, Clone, Serialize)]
pub struct CreepSnapshot {
    pub x: f32,
    pub y: f32,
    pub lane: i32,
    pub team: u32,
    pub wave_id: String,             // "lane_team_spawnsec" e.g. "1_2_45"
    pub nearby_players: Vec<i32>,    // player custom_ids within 1500 world units
}

/// Match-relative sparse timeline for one creep (None = dead or not yet spawned).
pub type CreepTimeline = Vec<Option<CreepSnapshot>>;

/// Metadata computed across the wave's full lifetime.
/// Wave membership is derived by filtering creep snapshots where wave_id matches --
/// no need to store entity indices here (avoids duplication and bloat).
#[derive(Debug, Clone, Serialize)]
pub struct WaveMeta {
    pub lane: i32,
    pub team: u32,
    pub spawn_sec: u32,
    pub last_death_sec: Option<u32>,    // match-relative second of last creep death
    pub last_death_x: Option<f32>,
    pub last_death_y: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct LaneCreepData {
    /// key: entity_index as string (JSON compat)
    pub creeps: HashMap<String, CreepTimeline>,
    /// key: wave_id "lane_team_spawnsec"
    pub wave_meta: HashMap<String, WaveMeta>,
}
```

### A3. Rewrite `CreepTracker` (`parser/src/tracking/creep_tracker.rs`)

**Internal state:**

```rust
struct ActiveCreep {
    entity_index: i32,
    lane: i32,
    team: u32,
    x: f32,
    y: f32,
    wave_id: String,
}

pub struct CreepTracker {
    lane_key: u64,
    active_creeps: HashMap<i32, ActiveCreep>,
    creep_timelines: HashMap<i32, CreepTimeline>,      // entity_index -> match-relative timeline
    wave_meta: HashMap<String, WaveMeta>,              // wave_id -> metadata
    wave_last_spawn: HashMap<(i32, u32), (u32, String)>,  // (lane, team) -> (spawn_sec, wave_id)
}
```

**Wave assignment (replaces spatial clustering entirely):**
- On CREATE: check `wave_last_spawn[(lane, team)]`
  - If `current_sec - last_spawn_sec <= 5`: assign to same `wave_id`
  - Otherwise: create new wave; `wave_id = format!("{}_{}_{}", lane, team, current_sec)`; update `wave_last_spawn`
- This eliminates the `cluster_creeps` function and the `WAVE_CLUSTER_THRESHOLD` constant

**Death tracking:**
- On DELETE: look up the creep's last known position from `active_creeps`
- Remove from `active_creeps`
- Extend its timeline with `None` for remaining seconds going forward (handled in snapshot builder)
- Check if this was the last alive creep in its wave:
  - `alive_in_wave = active_creeps.values().filter(|c| c.wave_id == dying_wave_id).count()`
  - If 0: update `wave_meta[wave_id].last_death_sec`, `last_death_x`, `last_death_y`

**Per-second snapshot:**
- New signature: `pub fn build_creep_snapshot(&mut self, window_sec: u32, player_positions: &[(i32, f32, f32)])`
  - `player_positions`: slice of `(custom_id, x, y)` for all current player positions
- For each active creep: compute nearby players (euclidean ≤ 1500 units), push `Some(CreepSnapshot)` to its timeline
- For each inactive creep in `creep_timelines`: push `None` (match-relative alignment)

**Integration in `replay_parser.rs`:**
- `on_tick_end` (real tick): call `creep_tracker.build_creep_snapshot(match_window_sec, player_positions_slice)`
  - Derive `player_positions_slice` from `positions` tracker (verify what it exposes and adapt)
- `get_match_data_json`: rename key to `"lane_creep_data"`, output `LaneCreepData`
- Verify DELETE routing in `on_entity()` -- confirm creep deletes are gated by entity type check (or by checking if entity_index is in active_creeps)

### A4. Tests

**Timeline alignment note:** `timeline[0]` = match second 0 = the moment the gameStart flag fires (tick 1). All timelines begin at this point -- no pre-match padding. Creep tests should treat index 0 as the game's first second.

Add unit tests in a `#[cfg(test)]` module in `creep_tracker.rs`:
- Wave grouping: 4 creeps at sec 45, then 4 more at sec 75 → two distinct wave_ids
- Wave grouping same wave: 4 creeps at secs 45, 46, 47, 48 → same wave_id
- Death pin: 4 creeps in wave, remove 3 → `last_death_sec` is None; remove 4th → pin set
- Nearby player: player at 1400 units → included; player at 1600 → excluded

### A5. Record learnings

Append any important findings to `private/learnings.md` ## Drafts section. Examples: bug investigation outcomes, entity lifecycle discoveries, why spawn-time grouping was chosen over spatial clustering, any Deadlock entity behavior that future agents should know.

### A6. Checkpoint

Stop here. Run `cargo test` and parse a real replay sample. Share:
- Test results
- Sample JSON output (first 60 seconds, a few creep timelines and wave_meta)

Await user review and commit approval before proceeding to Phase B.

---

## Phase B -- Backend (backend-python agent)

### B1. Update domain models

**`backend/app/domain/creep.py`:** Replace existing classes with Python equivalents:
```python
class CreepSnapshot(SQLModel):
    x: float
    y: float
    lane: int
    team: int
    wave_id: str
    nearby_players: list[int]

class WaveMeta(SQLModel):
    lane: int
    team: int
    spawn_sec: int
    last_death_sec: Optional[int] = None
    last_death_x: Optional[float] = None
    last_death_y: Optional[float] = None
    # Wave membership derived by filtering creep snapshots where wave_id matches

class LaneCreepData(SQLModel):
    creeps: dict[str, list[Optional[CreepSnapshot]]]  # str(entity_index) -> timeline
    wave_meta: dict[str, WaveMeta]                    # wave_id -> metadata
```

**`backend/app/domain/lane_pressure.py`:** Remove centroid fields, update schema:
```python
class LanePressureSnapshot(SQLModel):
    pressure: float                  # 0.0 when wave dead; >0 when alive
    team: int
    wave_id: str
    creep_count: int                 # alive creep count at this second
    attributed_players: list[int]    # union of nearby_players across alive creeps at this second

class LanePressureData(SQLModel):
    pressure: dict[str, list[Optional[LanePressureSnapshot]]]  # wave_id -> timeline
```

### B2. Rewrite lane pressure service (`backend/app/services/lane_pressure_service.py`)

**Objective map construction:**
- Accept `boss_data: BossData` as input
- For each boss snapshot, record: `(lane, team, boss_type_priority, position_x, position_y, health_timeline)`
- Priority order: `GUARDIAN=1, WALKER=2, BASE_GUARDIAN=3, SHRINE=4, PATRON=5`
- Build lookup: `objective_map[(lane, team)] = sorted list of bosses by priority`

**Lane-based objective lookup (not proximity-based):**
- For creeps with `lane = L`, `team = T`:
  - Enemy objectives are in `objective_map[(L, enemy_team)]`
  - At second `s`, find the lowest-priority boss where `health_timeline[s] > 0` (still alive)
  - That boss's position is the "current pressure target"
  - `lane_length` = dynamic per second: `euclidean(own_frontline, enemy_frontline)` where each frontline is the lowest-priority alive objective for that team (replaces the original static guardian-to-guardian distance)

**Pressure calculation per wave per second:**
- For each second `s` in the match timeline:
  - Collect alive creeps in this wave: iterate all `creep_timelines` where `snapshot.wave_id == wave_id` and `creeps[idx][s]` is not None
  - If no alive creeps: append `None` for this second (no pressure)
  - Else:
    - `centroid_x = mean(c.x for c in alive)` (internal only, not in output)
    - `centroid_y = mean(c.y for c in alive)` (internal only, not in output)
    - Find current enemy objective at second `s` (from objective_map)
  - `dist = euclidean(centroid_xy, objective_xy)`
  - `raw_pressure = clamp(1.0 - dist / lane_length, 0.0, 1.0)`
  - `pressure = raw_pressure * (len(alive) * 0.25)`
  - `attributed_players = union of c.nearby_players for c in alive`
  - Append `LanePressureSnapshot(...)`

**Output key:** wave_id (same keys as LaneCreepData.wave_meta)

### B3. Update downstream

- `match_data_service.py`: Pass `boss_data` to `LanePressureCalculator.process_creep_waves(lane_creep_data, boss_data)`
- `analyze_match.py`: Parse `lane_creep_data` from parser JSON (was `creep_waves`)
- Update any Pydantic/SQLModel schema classes in `ParsedMatchResponse` or `TransformedMatchData` that reference old creep/pressure types

### B4. Tests

- [x] Unit: wave with 4 alive creeps near enemy guardian → pressure between 0.25 and 1.0
- [x] Unit: creeps alive at second 0, dead at second 1 → `None` snapshot at second 1 (empty waves cannot occur)
- [x] Unit: wave with 2 of 4 alive → `pressure_value = raw × 0.5` (half multiplier)
- [x] Unit: guardian destroyed → pressure calculated against walker position instead
- [x] Integration: full `process_creep_waves()` call with mock data

### B5. Record learnings

Append findings to `private/learnings.md` ## Drafts. Examples: how boss positions are extracted, objective ordering logic, lane-assignment-based lookup vs. proximity, pressure formula reasoning.

### B6. Checkpoint

Stop here. Run `pytest` and spot-check a real parsed match through the API. Share:
- Test results
- Sample `/match/analysis/{id}` response showing `lane_pressure` data

Await user review and commit approval before proceeding to Phase C.

---

## Phase C -- Frontend (frontend-react agent)

### C1. Update domain types

**`frontend/src/domain/creep.ts`:** Replace existing interfaces:
```typescript
export interface CreepSnapshot {
  x: number;
  y: number;
  lane: number;
  team: number;
  wave_id: string;
  nearby_players: number[];
}

export interface WaveMeta {
  lane: number;
  team: number;
  spawn_sec: number;
  last_death_sec: number | null;
  last_death_x: number | null;
  last_death_y: number | null;
  // Wave membership derived by filtering creep timelines where wave_id matches
}

export interface LaneCreepData {
  creeps: Record<string, (CreepSnapshot | null)[]>;  // string entity_index -> timeline
  wave_meta: Record<string, WaveMeta>;               // wave_id -> metadata
}
```

**`frontend/src/domain/lanePressure.ts`:** Remove centroid fields, match B1 schema.

### C2. Rewrite `CreepWaveLayer.tsx`

Replace centroid-dot rendering with two passes:

**Pass 1 -- Live creeps:**
- Iterate `laneCreepData.creeps` entries
- If `timeline[currentTick]` is not null: render a small filled circle at `worldToMinimapPixels(snapshot.x, snapshot.y)` in team color

**Pass 2 -- Pins:**
- Iterate `laneCreepData.wave_meta` entries
- For a pin to be shown at `currentSec`:
  - `last_death_sec != null && currentSec >= last_death_sec`
  - No newer wave (same lane+team) has `last_death_sec <= currentSec`
- Render a black filled circle at `worldToMinimapPixels(last_death_x, last_death_y)`

### C3. Update `CreepWaveIndicator.tsx`

Simplify to two modes:
- **Live** mode: small filled circle in team color (no count overlay)
- **Pin** mode: small black filled circle

### C4. Update `MatchAnalysis.tsx` and API client

- `MatchAnalysis.tsx`: pass `laneCreepData` (not `creepWaves`) to `CreepWaveLayer`
- `MatchAnalysis.ts` API client: update parsing to read `lane_creep_data` key from parser JSON response

### C5. Tests

- Snapshot test: `CreepWaveLayer` renders individual dots at correct pixel positions
- Unit test: pin visibility logic (not shown before `last_death_sec`, shown after, replaced by newer wave pin)

### C6. Record learnings

Append findings to `private/learnings.md` ## Drafts. Examples: pin rendering logic, how `last_death_sec` drives supersession, minimap coordinate handling for per-creep vs. wave-centroid approaches.

### C7. Checkpoint

Stop here. Run `npm test`. Manually verify the minimap:
- Individual dots visible for each live creep
- Black pin dot appears at last death position after wave clears
- Pin updates when next wave clears
- Lane pressure chart shows 0 between waves

Await user review and commit approval.

---

## Verification Summary

| Phase | Command | Key checks |
|---|---|---|
| A | `cargo test` | No double-wave, correct death tracking, wave grouping |
| A | Parse real replay | 4 creeps in `creeps` at index 0 (game start), each with own timeline, pins set after wave dies |
| B | `pytest` | Pressure 0 when wave dead, scales with alive count, lane-based objective lookup |
| B | API spot-check | `lane_pressure` keyed by wave_id, no centroid fields |
| C | `npm test` | Snapshot tests pass, pin logic correct |
| C | Manual minimap | Individual dots + black pins, pressure chart behavior |

---

## Execution Order

1. **Phase A** (rust-parser) → review → commit
2. **Phase B** (backend-python) → review → commit
3. **Phase C** (frontend-react) → review → commit

Phases B and C are sequential on Phase A's finalized output schema.
