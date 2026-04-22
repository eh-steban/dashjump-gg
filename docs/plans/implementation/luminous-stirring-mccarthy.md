# Game Phase Module + Lane Pressure Normalization Plan

## Context

Lane pressure values exceed 100% because the current formula is unbounded --
`raw * (creep_count * 0.25 + players_in_lane * 1.5)` has no ceiling. 2 players + 4
creeps at close range produces ~250% from the first minute. We need:
1. A `GamePhase` domain module that maps any match-second to a phase (laning / mid /
   late) and carries phase-specific tuning constants. Values should be trivially
   adjustable by the team, with the architecture ready for coach-level config if we
   decide that's valuable later.
2. A reformed formula (Options A+D) with two normalized channels (creep, player) that
   caps at 100% and exposes each channel separately for future UI and coaching work.
3. A dev script to re-transform cached matches from stored raw parser output, so schema
   changes don't require a full re-parse.

**Goals:**
- Lane pressure is bounded 0--100% at all times
- "Full lane" for each phase reads as 100% (e.g. 2 players + 4 creeps laning)
- Creep and player contributions are recorded separately per snapshot
- Phase constants are in one obvious place and easy to tune
- Re-transform script enables testing schema changes without re-parsing

**Branch:** `feature/player-lane-pressure` (existing worktree at `dashjump-gg-player-lane-pressure`)
**Review workflow:** implement → test → subagent updates plan → pause for user review → commit → next phase

---

## Scope

| Service  | Involved | Agent            |
|----------|----------|------------------|
| Parser   | no       | --               |
| Backend  | yes      | `backend-python` |
| Frontend | no       | --               |

---

## Acceptance Criteria

- [ ] Lane pressure never exceeds 1.0 in any snapshot
- [ ] 2 players + 4 creeps at laning phase near enemy guardian → pressure ≈ raw_pressure (≈ 1.0)
- [ ] `LanePressureSnapshot` includes `phase`, `creep_pressure`, `player_pressure`
- [ ] `build_phase_classifier` returns correct phase across all boundaries
- [ ] Re-transform script loads raw gzip from DB, re-transforms, saves `match_data` -- no parser call
- [ ] All existing + new pytest tests pass
- [ ] All in-scope phase checkpoints complete and signed off by user

---

## Reference Data

### Phase Boundaries
Percentage of `match_duration = total_match_time_s - match_start_time_s`, resolved at
transform time (not hardcoded seconds -- varies by match length).

| Phase  | Start | End  |
|--------|-------|------|
| Laning | 0%    | 33%  |
| Mid    | 33%   | 66%  |
| Late   | 66%   | 100% |

### Phase Config Defaults
All constants grouped at the top of `game_phase.py`, clearly labeled for easy tuning.
Creep channel weight scales **up** late game (more waves active, creep signal matters
more); player weight scales **down** correspondingly.

| Phase  | expected_creeps | expected_players | creep_weight | player_weight |
|--------|-----------------|------------------|--------------|---------------|
| Laning | 4               | 2                | 0.40         | 0.60          |
| Mid    | 6               | 3                | 0.50         | 0.50          |
| Late   | 8               | 5                | 0.60         | 0.40          |

`creep_weight + player_weight` must always equal 1.0 (enforced in `__post_init__`).
All values marked `# TODO: calibrate with coaches` in code.

### Formula (Options A + D combined)

```
creep_term      = clamp(creep_count / phase.expected_creeps, 0.0, 1.0)
player_term     = clamp(players_in_lane / phase.expected_players, 0.0, 1.0)
creep_pressure  = raw_pressure * phase.creep_weight * creep_term
player_pressure = raw_pressure * phase.player_weight * player_term
pressure        = creep_pressure + player_pressure
```

Max `pressure` = `raw_pressure * 1.0` since weights sum to 1.0.

### Existing Storage Architecture (re-transform context)

`parsedmatch` table has two data columns:
- `raw_payload_gzip` (BYTEA) -- gzipped JSON of `ParsedMatchResponse` (immutable)
- `match_data` (JSONB) -- transformed `TransformedMatchData` (what the API serves)

Re-transform: decompress `raw_payload_gzip` → `ParsedMatchResponse` → `MatchDataService.transform()` → overwrite `match_data`. No parser call needed.

---

## Critical Files

| Layer | File | Change |
|-------|------|--------|
| Backend repo | `backend/app/repo/parsed_matches_repo.py` | Modify |
| Backend dev script | `backend/scripts/retransform_match.py` | Create |
| Backend domain | `backend/app/domain/game_phase.py` | Create |
| Backend domain | `backend/app/domain/lane_pressure.py` | Modify |
| Backend service | `backend/app/services/lane_pressure_service.py` | Modify |
| Backend service | `backend/app/services/match_data_service.py` | Modify |
| Backend tests | `backend/tests/test_game_phase.py` | Create |
| Backend tests | `backend/tests/test_lane_pressure_service.py` | Modify |

---

## Phase B -- Backend (`backend-python` agent)

### B1. Add re-transform methods to `parsed_matches_repo.py`

Add two methods to `ParsedMatchesRepo`:

**`get_raw_parsed_match(match_id, schema_version, session) -> ParsedMatchResponse | None`**
- Calls existing `get_raw_gzip()` to retrieve bytes
- `gzip.decompress(raw) → json.loads() → ParsedMatchResponse(**dict)`
- Returns `None` if no row found

**`update_match_data(match_id, schema_version, match_data, etag, session) -> None`**
- `UPDATE parsedmatch SET match_data=:data, etag=:etag WHERE match_id=:id AND schema_version=:v`
- Does not touch `raw_payload_gzip` or `created_at`

### B2. Create `backend/scripts/retransform_match.py`

Standalone script invoked via `docker compose exec`:

```
Usage: python scripts/retransform_match.py <match_id> [--schema-version 1]
```

Steps:
1. Load `ParsedMatchResponse` from `raw_payload_gzip` via `get_raw_parsed_match()`
2. Run `MatchDataService.transform(parsed_match)` with current logic
3. Compute new etag from transformed data
4. Call `update_match_data()` to overwrite `match_data` in place

Log `INFO` at start ("Retransforming match {id}"), completion ("Done -- match {id} retransformed"), and `ERROR` if raw gzip missing. No other output.

This step is a **separate commit** from the game phase logic.

**Run command (for CLAUDE.md):**
```bash
docker compose --project-directory /home/lifted/Code/dashjump-gg-<name> exec dashjump-backend python scripts/retransform_match.py <match_id>
```

### B3. Create `backend/app/domain/game_phase.py`

Pure domain module, no framework dependencies.

**At the top of the file -- the tuning section (clearly labeled):**
```python
# ---------------------------------------------------------------------------
# Phase tuning -- adjust these values to calibrate lane pressure.
# creep_weight + player_weight must equal 1.0 per phase.
# TODO: calibrate with coaches
# ---------------------------------------------------------------------------
_LANING_EXPECTED_CREEPS   = 4
_LANING_EXPECTED_PLAYERS  = 2
_LANING_CREEP_WEIGHT      = 0.40
_LANING_PLAYER_WEIGHT     = 0.60

_MID_EXPECTED_CREEPS      = 6
_MID_EXPECTED_PLAYERS     = 3
_MID_CREEP_WEIGHT         = 0.50
_MID_PLAYER_WEIGHT        = 0.50

_LATE_EXPECTED_CREEPS     = 8
_LATE_EXPECTED_PLAYERS    = 5
_LATE_CREEP_WEIGHT        = 0.60
_LATE_PLAYER_WEIGHT       = 0.40

_LANING_END_FRACTION = 1 / 3  # 0-33% of match = laning
_MID_END_FRACTION    = 2 / 3  # 33-66% = mid, 66-100% = late
```

**`GamePhase(str, Enum)`** -- `LANING`, `MID`, `LATE`

**`PhaseConfig` (frozen dataclass):**
- Fields: `phase`, `expected_creeps`, `expected_players`, `creep_channel_weight`, `player_channel_weight`
- `__post_init__`: assert `abs(creep_channel_weight + player_channel_weight - 1.0) < 1e-9`

**`_PHASE_CONFIGS: dict[GamePhase, PhaseConfig]`** -- built from the tuning constants above

**`build_phase_classifier(match_duration: int) -> Callable[[int], PhaseConfig]`:**
- Computes `laning_end = int(match_duration * _LANING_END_FRACTION)`
- Computes `mid_end = int(match_duration * _MID_END_FRACTION)`
- Returns closure `classify(second: int) -> PhaseConfig`

### B4. Update `backend/app/domain/lane_pressure.py`

Add to `LanePressureSnapshot`:
- `phase: str` -- GamePhase value ("laning" / "mid" / "late")
- `creep_pressure: float`
- `player_pressure: float`

Keep `pressure: float` as the sum. No fields removed.

### B5. Update `backend/app/services/lane_pressure_service.py`

**Import:** `from app.domain.game_phase import PhaseConfig` and `from typing import Callable`.

**Signature:** add `phase_classifier: Callable[[int], PhaseConfig]` as final parameter.
Remove `_PLAYER_LANE_WEIGHT` constant.

**Formula replacement** inside the per-second loop after `raw_pressure`:
```python
phase_config = phase_classifier(second)
creep_term = _clamp(len(alive_creeps) / phase_config.expected_creeps, 0.0, 1.0)
player_term = _clamp(players_in_lane / phase_config.expected_players, 0.0, 1.0)
creep_pressure = raw_pressure * phase_config.creep_channel_weight * creep_term
player_pressure = raw_pressure * phase_config.player_channel_weight * player_term
pressure = creep_pressure + player_pressure
```

Update `LanePressureSnapshot(...)` construction to pass `phase`, `creep_pressure`, `player_pressure`.

Update module docstring to reflect new formula.

### B6. Update `backend/app/services/match_data_service.py`

```python
from app.domain.game_phase import build_phase_classifier

match_duration = parsed_match.total_match_time_s - parsed_match.match_start_time_s
phase_classifier = build_phase_classifier(match_duration)

lane_pressure = LanePressureCalculator.process_creep_waves(
    parsed_match.lane_creep_data,
    parsed_match.bosses,
    parsed_match.positions,
    parsed_match.players_data,
    phase_classifier,
)
```

### B7. Create `backend/tests/test_game_phase.py`

Use `match_duration = 3000` (round number, clean phase boundaries at 1000s and 2000s).

- `second=0` → `LANING`
- `second=999` → `LANING`
- `second=1000` → `MID` (first second of mid)
- `second=1999` → `MID`
- `second=2000` → `LATE`
- `second=2999` → `LATE`
- Verify `PhaseConfig` fields match `_PHASE_CONFIGS` for each phase
- Verify `PhaseConfig.__post_init__` raises on mismatched weights

### B8. Update `backend/tests/test_lane_pressure_service.py`

All existing `process_creep_waves(data, bosses, [], [])` calls need a `phase_classifier`
argument. Use a fixed laning classifier for all existing tests:

```python
from app.domain.game_phase import build_phase_classifier
_LANING_CLASSIFIER = build_phase_classifier(300)  # all seconds fall in laning
```

Recalculate expected values in `TestPartialAliveCreeps` and `TestPlayerLaneContribution`
using the new formula and laning-phase weights.

Add `TestPhaseBasedPressure`:
- Full laning (2 players + 4 creeps, near enemy guardian) → pressure ≈ raw_pressure
- Half-full laning (1 player + 2 creeps) → pressure ≈ raw_pressure * 0.5
- Same half-full scenario classified as mid → pressure < same scenario at laning
  (different expected counts → lower term values)

### B9. Record learnings

Append to `private/learnings.md` ## Drafts:
- Game phase module design: injectable classifier resolved at transform time from known `match_duration`; constants grouped for easy tuning; extensible for coach config without service changes
- Why percentage boundaries beat absolute seconds
- Re-transform workflow: `raw_payload_gzip` stores the immutable `ParsedMatchResponse`; `match_data` is the mutable transformed output; re-transform overwrites only `match_data`

---

### B Checkpoint

**Status:** `[ ] Not started`

> **Agent instructions:** Stop here. Before returning you MUST:
> 1. Run `docker compose --project-directory /home/lifted/Code/dashjump-gg-player-lane-pressure exec dashjump-backend pytest tests/` and record results
> 2. Run `python scripts/retransform_match.py 68182475` and confirm it completes without error
> 3. Hit `GET /match/analysis/68182475` -- verify `pressure` ≤ 1.0 and `phase` / `creep_pressure` / `player_pressure` fields appear in every snapshot
> 4. Check off every item below with date and actual result
> 5. Note any deferred items
> 6. Update **Status** above

#### Results *(agent fills in)*

- [ ] `pytest` -- [X passed, Y failed]
- [ ] Retransform script ran cleanly for match 68182475 -- [date]
- [ ] `pressure` ≤ 1.0 in all snapshots -- [date / verified against match 68182475]
- [ ] `phase`, `creep_pressure`, `player_pressure` present in API response -- [date]
- [ ] Laning snapshot with 4 creeps + 2 players near guardian ≈ raw_pressure -- [date]
- [ ] Learnings appended to `private/learnings.md`

#### Sample output *(agent fills in)*
```
[Paste 3 LanePressureSnapshot examples -- one per phase]
```

#### Deferred items
[None, or list with reason]

Await user review and commit approval.

---

## Verification Summary

| Phase | Command | Key checks | Status |
|-------|---------|------------|--------|
| B | `pytest tests/` | All pass; formula tests use updated expected values | |
| B | `python scripts/retransform_match.py 68182475` | Completes cleanly, no parser call | |
| B | `GET /match/analysis/68182475` | `pressure` ≤ 1.0; `phase`, `creep_pressure`, `player_pressure` in every snapshot | |

---

## Execution Order

1. **B1--B2** (re-transform script) → user review → commit (standalone utility)
2. **B3--B9** (game phase module + formula) → user review → commit
