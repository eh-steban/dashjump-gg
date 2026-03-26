---
paths:
  - "backend/**/*.py"
  - "backend/**/**/*.py"
  - "backend/**/**/**/*.py"
  - "backend/**/**/**/**/*.py"
---
# Backend Mental Model

## S3 Storage Strategy

**Status:** Evaluated (not yet implemented)

PostgreSQL JSONB storage fails at scale for match data. Each parsed match produces 15-18 MB of JSON. At this size, JSONB hits hard limits: slow queries, large row sizes, and expensive full-object updates.

**Chosen architecture:**
- Store raw + transformed match JSON in S3 (one object per match)
- Keep lightweight metadata in PostgreSQL (`match_id`, `s3_key`, `duration_seconds`, `status`)
- API layer fetches S3 object on cache miss; PostgreSQL answers metadata queries

**Why not differential encoding?**
Differential encoding (storing deltas between frames) reduces storage size but adds read-time reconstruction complexity with no query performance benefit. S3 achieves storage efficiency without the reconstruction overhead.

**Key constraint:** JSONB reserved for small metadata only. Any new large-data feature defaults to S3 storage pattern.

**See:** `private/learnings.md` — "S3 Storage Solves JSONB Bottleneck" for the cross-project summary.

---

## Lane Pressure Service Architecture

**File:** `backend/app/services/lane_pressure_service.py`

### Objective lookup: lane-based, not proximity-based

Creep centroid proximity to objectives breaks down near lane junctions, base entrances, and during team fights. Deadlock creeps belong to a specific lane from spawn (`m_iLane`); they target only objectives in that lane. Use `(lane, team)` as the lookup key -- it is both accurate and O(1) per second.

### Lane length is dynamic, not static

Lane length is computed per second as `euclidean(own_frontline, enemy_frontline)`, where each frontline is the lowest-priority alive objective for that team at that second. As objectives die, both endpoints move deeper into their respective bases and the denominator contracts, keeping pressure values meaningful throughout the match. The old static guardian-to-guardian approach (21504.0 fallback) has been removed.

### Objective liveness: two-signal approach

Use `death_time_s` as the primary gate and `health_timeline` as belt-and-suspenders. `death_time_s` is set at the entity deletion tick by the parser and is more reliable than health alone. `health_timeline` carry-forward confirms liveness second-by-second. Either signal can be absent or delayed by demo recording artifacts; checking both makes liveness tolerant of edge cases.

**Objective chain per lane:** Guardian → Walker → Base Guardian → Shrine → Patron. Boss health_timeline is indexed by match second (0 = match start). A boss's entity_index absent from a window is a data gap -- treat as alive. Health == 0 means destroyed.

### Phase weights: late game is player-driven

Phase weights intentionally scale in opposite directions from what might be expected:

| Phase | creep_weight | player_weight | Rationale |
|-------|-------------|---------------|-----------|
| Laning | 0.65 | 0.35 | Creep wave position is the primary pressure signal |
| Mid | 0.50 | 0.50 | Balanced |
| Late | 0.25 | 0.75 | Player clustering at objectives dominates |

Creep waves are sparse in late game; players are the dominant force. Weights tuned for match 68182475 -- needs coach calibration for final values.

### Ghost creeps are a parser problem, not a backend problem

Do not add backend staleness filters to work around ghost creep positions. A position-staleness filter incorrectly excludes melee fighters stopped at objectives (real data) along with ghost creeps (bad data). The parser whitelist (`life_state == ALIVE && health > 0 && npc_state in whitelist`) is the correct suppression mechanism. Any ghost that leaks through is a parser bug.

---

**See `.claude/knowledge-management.md` for when and how to populate this file.**
