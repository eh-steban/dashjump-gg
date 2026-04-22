# Backend API Contract

**Endpoint:** `GET /match/analysis/{match_id}`
**Owner:** `backend-python` agent -- update this file when changing response fields
**Consumer:** `frontend-react` agent (`frontend/src/domain/` interfaces)

This file is the source of truth for the JSON shape the backend returns. Any field added,
removed, or renamed here requires a matching update in `frontend/src/domain/` before
the work is considered complete.

---

## Top-Level Response (`MatchAnalysis`)

| Field | Type | Required | Notes |
|-------|------|----------|-------|
| `match_metadata` | MatchMetadata | yes | From Deadlock API |
| `parsed_match_data` | TransformedMatchData | yes | Parsed + transformed replay data |

---

## TransformedMatchData

| Field | Type | Required | Notes |
|-------|------|----------|-------|
| `match_duration_s` | int | yes | Match duration in seconds (passthrough from parser, immutable) |
| `match_start_time_s` | int | yes | Seconds into the replay recording where the match starts |
| `players_data` | PlayerData[] | yes | One entry per hero |
| `per_player_data` | Record<string, PlayerMatchData> | yes | Key = `custom_id` (string) |
| `bosses` | BossData | yes | Objective data |
| `lane_creep_data` | LaneCreepData | yes | Per-creep tracking |
| `lane_pressure` | LanePressureData | yes | Defaults to `{"pressure": {}}` |
| `mid_boss` | MidBossData | yes | Mid-boss tracking data (mid-boss always spawns at 10 min) |

---

## PlayerData

| Field | Type | Required | Notes |
|-------|------|----------|-------|
| `entity_id` | string | yes | Entity index as string |
| `custom_id` | string | yes | Stable per-player identifier; key in `per_player_data` |
| `name` | string | yes | Steam display name |
| `team` | int | yes | 2 = Amber, 3 = Sapphire |
| `lane` | int | yes | Starting lane (1-4) |
| `steam_id_32` | int | no | 32-bit Steam ID |
| `hero_id` | int | no | Hero ID |
| `lobby_player_slot` | int | no | Slot in lobby (0-indexed) |

---

## PlayerMatchData

| Field | Type | Required | Notes |
|-------|------|----------|-------|
| `positions` | PlayerPosition[] | yes | Positions for one tick (the tick this player appears in) |
| `damage` | DamageWindow[] | yes | Per-tick damage records for this player (as attacker) |

**DamageWindow:** `Record<string, DamageRecord[]>` -- victim entity index (string) to records.

**Note:** Keys are i32 entity indices serialized as strings (JSON object key constraint).

### DamageRecord

All fields optional.

| Field | Type | Notes |
|-------|------|-------|
| `damage` | int? | Raw damage dealt |
| `pre_damage` | float? | Damage before mitigation (fractional -- Deadlock applies damage internally as f32) |
| `type` | int? | Damage type enum |
| `citadel_type` | int? | Citadel-specific damage type |
| `entindex_inflictor` | int? | Inflictor entity index |
| `entindex_ability` | int? | Ability entity index |
| `damage_absorbed` | float? | Amount absorbed by shield (fractional, same as pre_damage) |
| `victim_health_max` | int? | Victim max HP at time of hit |
| `victim_health_new` | int? | Victim HP after hit |
| `flags` | int? | Damage flags bitmask |
| `ability_id` | int? | Ability that dealt damage |
| `attacker_class` | int? | Attacker entity class |
| `victim_class` | int? | Victim entity class |
| `victim_shield_max` | int? | Victim max shield at time of hit |
| `victim_shield_new` | int? | Victim shield after hit |
| `hits` | int? | Number of hits in this record |
| `health_lost` | int? | Net health lost |

---

## PlayerPosition

| Field | Type | Required | Notes |
|-------|------|----------|-------|
| `custom_id` | string | yes | Matches `PlayerData.custom_id` |
| `x` | float | yes | World coordinate |
| `y` | float | yes | World coordinate |
| `z` | float | yes | World coordinate (height) |
| `is_npc` | bool | yes | True for NPC entities |

**World bounds:** `[-10752, 10752]` on both x and y axes (confirmed from game rules entity).

---

## BossData

Identical shape to parser output -- backend passes through without transformation.

See `parser-output.md` -- BossData, BossSnapshot sections.

---

## LaneCreepData

Identical shape to parser output -- backend passes through without transformation.

See `parser-output.md` -- LaneCreepData, CreepSnapshot, WaveMeta sections.

---

## LanePressureData

| Field | Type | Required | Notes |
|-------|------|----------|-------|
| `pressure` | Record<string, (LanePressureSnapshot \| null)[]> | yes | Key = wave_id (see format below) |

**wave_id format:** `"{lane}_{team}_{spawn_sec}"` -- same key space as `LaneCreepData.wave_meta`

### LanePressureSnapshot

| Field | Type | Required | Notes |
|-------|------|----------|-------|
| `pressure` | float | yes | 0.0 = own base, higher = closer to enemy objective |
| `team` | int | yes | 2 = Amber, 3 = Sapphire |
| `wave_id` | string | yes | Matches key in `LanePressureData.pressure` |
| `creep_count` | int | yes | Alive creep count at this second |
| `attributed_players` | int[] | yes | `custom_id`s of nearby players (union across alive creeps) |

---

## MatchMetadata

Sourced from Deadlock API (not from replay). Shape may evolve as API changes.

| Field | Type | Required | Notes |
|-------|------|----------|-------|
| `match_info` | MatchInfoFields | yes | Match summary from Deadlock API |

### MatchInfoFields (key fields)

| Field | Type | Notes |
|-------|------|-------|
| `match_id` | int | |
| `duration_s` | int | |
| `winning_team` | int | |
| `match_outcome` | int | |
| `players` | PlayerInfo[] | Minimal team/hero info per player |
| `objectives` | DestroyedObjective[] | Objective destruction events with timestamps |
| `damage_matrix` | object | Cross-player damage breakdown from API |
| `start_time` | int | Unix timestamp of match start |
| `game_mode` | int | |
| `match_mode` | int | |
| `average_badge_team0` | int | |
| `average_badge_team1` | int | |

---

## MidBossData

Passed through from parser output unmodified.

Identical shape to parser output -- see `parser-output.md` MidBossData section for full field specs.

**Backend-specific notes:**
- `mid_boss` is required on `TransformedMatchData` -- mid-boss always spawns at 10 minutes
- All fields are passthrough from parser
- `post_match` is populated by the parser from collected kill events (haste replay data)

**Overlap with Deadlock API response:** The Deadlock API (HTTP, see `MatchInfoFields.mid_boss: list[dict]` in `app/domain/deadlock_api.py`) exposes equivalent semantic data (team_killed, team_claimed, destroyed_time_s) on its `match_info` payload. The frontend's objective damage panel currently consumes that HTTP source via `match_metadata`. The parser populates `MidBossPostMatch` independently by decoding Valve's `CMsgMatchMetaDataContents` protobuf from replay bytes via the haste library -- the parser never calls the Deadlock API. Both sources should agree; the parser version is the primary source going forward and the `match_metadata.mid_boss` path will be deprecated once the frontend migrates.

---

## Caching Headers

Responses include:
- `ETag: <sha256_hash>` -- use with `If-None-Match` for 304 Not Modified
- `Cache-Control: public, max-age=300`
- `Content-Encoding: gzip` -- backend applies GZip compression >= 500 bytes
