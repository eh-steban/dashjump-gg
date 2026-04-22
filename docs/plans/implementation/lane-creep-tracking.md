# Lane Creep Tracking Feature Plan

## Overview
Track lane creep positions to compute lane pressure metrics for Deadlock match analysis.

## Tracking Info
- **Plan File:** `~/.claude/plans/ethereal-riding-twilight.md`
- **Todo File:** `~/.claude/todos/944be8a5-2014-42a7-ba14-2810d97477c3-agent-944be8a5-2014-42a7-ba14-2810d97477c3.json`
- **Branch:** `feature/lane-creep-tracking-parser-refactor`
- **Last Updated:** 2026-02-06

## Current Status
- Phase 0: Pending (lane color fix deferred)
- Phase 1: Completed
- Phase 2: Completed
- Phase 2.5: Completed
- Phase 3: Completed
- Phase 4: Completed
- Phase 5-6: Pending

## Phases

### Phase 0: Fix lane color tracking in parser
**Status:** Pending (deferred for later troubleshooting)
- Lane color fix wasn't working correctly
- Code removed but `ZIPLINE_LANE_COLOR_KEY` constant kept for future use

### Phase 1: Parser refactor - extract domain, entities, utils, tracking modules
**Status:** Completed
- Extracted monolithic replay_parser.rs into modules
- Created: domain/, entities/, tracking/, utils/
- Committed: `c8c055a`

### Phase 2: Implement CreepTracker in parser
**Status:** Completed
- Created `tracking/creep_tracker.rs`
- Tracks CNPC_Trooper entities by lane/team
- Computes centroid positions per second
- Outputs `creep_waves` in JSON

### Phase 2.5: Minimal backend/frontend passthrough
**Status:** Completed
- Backend: Added CreepWaveData domain model, passthrough in transform
- Frontend: Added types, debug console.log
- Data now visible in browser console

### Phase 3: Backend integration - creep models and pressure calculation
**Status:** Completed
- Created `LanePressureService` with pressure calculation
- Added player attribution by proximity (1500 unit threshold)
- Fixed: Only attribute to actual players, not NPCs (`is_npc` check)
- Committed: `8f6634e`

### Phase 4: Frontend visualization - wave indicators on minimap
**Status:** Completed
- Created `LanePressurePanel` component
- Displays active waves with team colors and player attribution
- Committed: `963a903`

### Phase 4.5: Bug fixes
**Status:** Completed
- Fixed 3-creep issue: Parser now tracks pre-existing entities via UPDATE events (game pre-spawns 192 troopers)
- Added spatial clustering to keep separate waves distinct
- Wave key format changed to `{lane}_{team}_{waveIdx}`
- Committed: `376e393`

### Phase 5: Add tests for parser, backend, and frontend
**Status:** Pending

### Phase 6: Add observability and final E2E verification
**Status:** Pending

## Key Files Modified

### Parser
- `parser/src/tracking/creep_tracker.rs` (NEW)
- `parser/src/tracking/mod.rs`
- `parser/src/replay_parser.rs`
- `parser/src/domain/creep.rs` (NEW)

### Backend
- `backend/app/domain/creep.py` (NEW)
- `backend/app/domain/match_analysis.py`
- `backend/app/services/transform_service.py`
- `backend/app/application/use_cases/analyze_match.py`

### Frontend
- `frontend/src/domain/creep.ts` (NEW)
- `frontend/src/domain/matchAnalysis.ts`
- `frontend/src/api/MatchAnalysis.ts`

## Notes
- Game pre-spawns 192 CNPC_Trooper entities at spawn points before match start
- Creeps are tracked via both CREATE and UPDATE events (pre-existing entities only receive UPDATE)
- Spatial clustering keeps separate waves distinct (threshold: 1000 world units)
- Player attribution proximity threshold: 1500 world units
- Wave key format: `{lane}_{team}_{waveIdx}` (e.g., "1_2_0" for lane 1, team 2/Amber, wave 0)

## Blockers
None currently

## To Resume on Another Instance
1. Read this plan file
2. Check git status on branch `feature/lane-creep-tracking-parser-refactor`
3. Load todo file: `944be8a5-2014-42a7-ba14-2810d97477c3-agent-944be8a5-2014-42a7-ba14-2810d97477c3.json`
4. Continue from current phase
