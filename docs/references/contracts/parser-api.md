# Parser Output Contract

**Endpoint:** `POST http://localhost:9000/parse`
**Owner:** `rust-parser` agent -- update this file when changing serialized output fields
**Consumer:** `backend-python` agent (`backend/app/domain/match_analysis.py` -- `ParsedMatchResponse`)

This file is the source of truth for the JSON shape the parser returns. Any field added,
removed, or renamed here requires a matching update in `ParsedMatchResponse` (backend) before
the work is considered complete.

---

## Top-Level Response

| Field | Type | Required | Notes |
|-------|------|----------|-------|
| `match_duration_s` | int | yes | Match duration in seconds (final value of internal match-second counter) |
| `match_start_time_s` | int | yes | Seconds into the replay recording where the match starts |
| `damage` | DamageTick[] | yes | Per-tick damage records (see below) |
| `players_data` | PlayerData[] | yes | One entry per hero in the match |
| `positions` | PositionWindow[] | yes | Per-tick position snapshots |
| `bosses` | BossData | yes | Objective snapshots + health timeline |
| `lane_creep_data` | LaneCreepData | yes | Per-creep lane tracking |
| `sinners` | SinnerSnapshot[] | yes | One entry per spawn event including respawns |
| `mid_boss` | MidBossData | yes | Mid-boss spawn/kill/health/rejuv tracking (see below) |

---

## DamageTick

One element per tick. Each tick is a map of attacker entity index (as string) to victim map.

```
DamageTick = Record<string, Record<string, DamageRecord[]>>
             ^attacker_idx  ^victim_idx
```

**Note:** Parser emits i32 entity indices as JSON object keys. JSON spec requires string keys, so
these arrive in Python as `dict[str, dict[str, list[...]]]`.

### DamageRecord

All fields optional -- parser omits zero/null values.

| Field | Type | Notes |
|-------|------|-------|
| `damage` | int? | Raw damage dealt |
| `pre_damage` | float? | Damage before mitigation (Deadlock applies fractional damage internally) |
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

## PlayerData

| Field | Type | Required | Notes |
|-------|------|----------|-------|
| `entity_id` | string | yes | Entity index as string |
| `custom_id` | string | yes | Stable per-player identifier |
| `name` | string | yes | Steam display name |
| `team` | int | yes | 2 = Amber, 3 = Sapphire |
| `lane` | int | yes | Starting lane (1, 2, 3, or 4) |
| `steam_id_32` | int | no | 32-bit Steam ID |
| `hero_id` | int | no | Hero ID |
| `lobby_player_slot` | int | no | Slot in lobby (0-indexed) |

---

## PositionWindow

Array of `PlayerPosition` objects (one per player alive that tick):

| Field | Type | Required | Notes |
|-------|------|----------|-------|
| `custom_id` | string | yes | Matches `PlayerData.custom_id` |
| `x` | float | yes | World coordinate |
| `y` | float | yes | World coordinate |
| `z` | float | yes | World coordinate (height) |
| `is_npc` | bool | yes | True for NPC entities |

---

## BossData

| Field | Type | Required | Notes |
|-------|------|----------|-------|
| `snapshots` | BossSnapshot[] | yes | One per objective entity |
| `health_timeline` | BossHealthWindow[] | yes | Per-second health map |

**BossHealthWindow:** `Record<string, int>` -- entity_index (as string) to current health. Carry-forward semantics: each window emits the most recent observed sample for every tracked boss, so a boss appears in every window from its first sample onward. Samples are captured from three sources: CREATE (initial HP), damage game events, and every entity UPDATE (which also catches Walker/Shrine sibling-scaling heals, Patron phase 1->2 reset, and out-of-combat regen that don't produce damage events). After death the parser inserts a `health=0` sample, which then carries forward for the rest of the match.

### BossSnapshot

| Field | Type | Required | Notes |
|-------|------|----------|-------|
| `entity_index` | int | yes | Per-game entity slot. **NOT a stable type identifier** -- slots are reused across games and within a game. Use `boss_name_hash` for type matching. |
| `custom_id` | int | yes | Convenience enum assigned by `parser/src/replay_parser.rs::get_custom_id` in source-definition order: 21=Guardian, 26=Base Guardian, 27=Shrine, 28=Walker, 29=Patron. The numeric value is **not** an attack-priority ordering and is **not** the canonical identifier -- use `boss_name_hash` for any load-bearing logic. Consumers that key on `custom_id` risk silently breaking when the parser adds or renames a boss type. |
| `boss_name_hash` | string | yes | **Canonical type identifier.** u64 fxhash of the SendTable serializer name (e.g. `"CNPC_TrooperBoss"`), transported as a decimal string because JavaScript `number` cannot losslessly hold integers above `2^53`. Computed in `parser/src/entities/constants.rs` at Rust compile time and emitted on every snapshot. Consumers should key on this value (it matches 1:1 with the `BOSS_*_ENTITY` constants in the parser). See [Boss Type Identification](#boss-type-identification) for the exact hash values. |
| `team` | int | yes | 2 = Amber, 3 = Sapphire |
| `lane` | int | yes | Lane number |
| `x` | float | yes | Spawn position |
| `y` | float | yes | Spawn position |
| `z` | float | yes | Spawn position |
| `spawn_time_s` | int | yes | Match-relative spawn second |
| `max_health` | int | yes | **Latest known max HP**, not create-time. Updated by the parser whenever the entity's `m_iMaxHealth` changes (Walker scaling, Patron phase transition, surviving Shrine buff). See [max_health Mechanics](#max_health-mechanics). Consumers computing `health / max_health` ratios should always read the current `BossSnapshot.max_health`, not assume the create-time value. |
| `life_state_on_create` | int | yes | Life state at entity creation |
| `death_time_s` | int | no | Match-relative second of death |
| `life_state_on_delete` | int | no | Life state at entity deletion |

#### Boss Type Identification

`boss_name_hash` is the only stable type identifier across games and across parser versions. Match against the entity class name hash:

| Objective | Entity class | `boss_name_hash` (u64 as decimal string) | `custom_id` (convenience) | Parser constant |
|-----------|--------------|------------------------|----------------------------|-----------------|
| Guardian | `CNPC_TrooperBoss` | `12946736302082733589` | 21 | `CNPC_TROOPERBOSS_ENTITY` |
| Base Guardian | `CNPC_BarrackBoss` | `793562361056549792` | 26 | `CNPC_BARRACKBOSS_ENTITY` |
| Shrine | `CCitadel_Destroyable_Building` | `8292725763874089450` | 27 | `CCITADEL_DESTROYABLE_BUILDING_ENTITY` |
| Walker | `CNPC_Boss_Tier2` | `1942975293714691302` | 28 | `CNPC_BOSS_TIER2_ENTITY` |
| Patron | `CNPC_Boss_Tier3` | `7814756300278693755` | 29 | `CNPC_BOSS_TIER3_ENTITY` |

All parser constants live in `parser/src/entities/constants.rs`. The `boss_name_hash` values above are the u64 results of `fxhash::hash_bytes(b"<class_name>")`, computed at Rust compile time. Consumers in other languages should hardcode these values and resync from a fresh parse if `constants.rs` ever adds or renames a boss type.

`boss_name_hash` is stable across games for the same objective type. **`entity_index` is not** -- never key on it for type detection.

#### max_health Mechanics

The parser tracks `m_iMaxHealth` changes via UPDATE events on boss entities, so `BossSnapshot.max_health` reflects the live value at the time of serialization. The mechanics that move it:

| Objective | Starting max | Scaling trigger | Resulting max | Wiki |
|-----------|--------------|-----------------|---------------|------|
| Guardian | 5,500 | -- (constant; only damage resistance scales over time) | 5,500 | [Guardian](https://deadlock.wiki/Guardian) |
| Base Guardian | 4,000 | -- (constant) | 4,000 | [Guardian](https://deadlock.wiki/Guardian) |
| Walker | 6,000 | Sibling Walker dies (per team) | +3,000 and full heal -> 9,000, then 12,000 | [Walker](https://deadlock.wiki/Walker) |
| Shrine | 5,000 | Sibling Shrine dies (per team) | Surviving Shrine -> 10,000 | [Shrine](https://deadlock.wiki/Shrine) |
| Patron | 12,000 | Phase 1 -> Phase 2 at 6,000 HP remaining | Phase 2 starts at 12,000 (no carry-over health) | [Patron](https://deadlock.wiki/Patron) |
| Patron | -- | Time-based scaling | Phase 1: +250/min after 20:00. Phase 2: +450/min starting 1:00 after phase entry. | [Patron](https://deadlock.wiki/Patron) |

Notes:
- Walker scaling is a flat additive heal -- the parser sees `m_iMaxHealth` and `m_iHealth` both jump on the UPDATE tick.
- Patron phase transition includes a brief invulnerability window while the form transforms; the parser does not currently emit a phase-change event, so consumers infer phase from a combination of `max_health` resets and elapsed time.
- Shrine pair scaling means a single team's surviving Shrine doubles in max HP after the first dies. The first Shrine in a pair stays at 5,000 throughout its life.
- The parser ignores `m_iMaxHealth <= 0` updates (transient teardown values).

---

## LaneCreepData

| Field | Type | Required | Notes |
|-------|------|----------|-------|
| `creeps` | Record<string, (CreepSnapshot \| null)[]> | yes | entity_index (string) to per-second timeline |
| `wave_meta` | Record<string, WaveMeta> | yes | wave_id to wave metadata |

**wave_id format:** `"{lane}_{team}_{spawn_sec}"` -- e.g., `"1_2_45"` (lane 1, team 2, spawned at sec 45)

### CreepSnapshot

| Field | Type | Required | Notes |
|-------|------|----------|-------|
| `x` | float | yes | World position |
| `y` | float | yes | World position |
| `lane` | int | yes | Creep's lane |
| `team` | int | yes | Creep's team |
| `wave_id` | string | yes | Wave this creep belongs to |
| `nearby_players` | int[] | yes | `custom_id`s of players within 1500 world units |
| `is_cage` | bool | yes | True = still on zipline (pre-lane-drop); default false |

### WaveMeta

| Field | Type | Required | Notes |
|-------|------|----------|-------|
| `lane` | int | yes | Lane number |
| `team` | int | yes | Team |
| `spawn_sec` | int | yes | Match-relative spawn second |
| `last_death_sec` | int | no | Match-relative second of last creep death in wave |
| `last_death_x` | float | no | World X of last creep death |
| `last_death_y` | float | no | World Y of last creep death |

---

## SinnerSnapshot

One entry per Sinner Sacrifice spawn event (initial spawn or respawn). Entity indices are recycled on respawn, so `entity_index` is not unique across snapshots in a single match.

| Field | Type | Required | Notes |
|-------|------|----------|-------|
| `entity_index` | int | yes | Entity index (recycled on respawn -- not unique across snapshots) |
| `x` | float | yes | World X at spawn |
| `y` | float | yes | World Y at spawn |
| `z` | float | yes | World Z at spawn |
| `spawn_time_s` | int | yes | Match-relative seconds at CREATE |
| `max_health` | int | yes | Max health at spawn (confirmed 500 for all sinners) |
| `death_time_s` | int | no | Match-relative seconds at killing blow tick; null if alive at match end |
| `time_alive_s` | int | no | Derived: `death_time_s - spawn_time_s`; null if alive at match end |
| `killer_player_slot` | int | no | Lobby slot (0-indexed) of the killing player; null if alive at match end or if Lil Helper owner resolution fails |
| `retaliation_damage` | Record\<string, int\> | yes | **DEPRECATED** -- Map of lobby player slot (as string) to total retaliation damage that sinner dealt to that player; empty object `{}` if no player hit this sinner. Retained for `PlayerCards.tsx` compatibility during the frontend refactor. Derivable via `damage_events.filter(e => e.kind === "retaliated").reduce(...)`. Will be removed in a follow-up parser commit after the frontend migrates to `damage_events`. |
| `damage_events` | SinnerDamageEvent[] | yes | Ordered list of all damage exchanges involving this sinner across its life (initial spawn through death). Populated for every player-sourced damage exchange; non-player attackers (troopers, etc.) are skipped silently. Scope is per-life: each snapshot starts with a fresh empty list. Preferred source of truth for retaliation and dealt damage going forward. May be empty if no players interacted with this sinner. |

**Backend impact:** `ParsedMatchResponse` and `TransformedMatchData` in `backend/app/domain/match_analysis.py` need `sinners: list[SinnerSnapshot]` added. Python type for `retaliation_damage` is `dict[str, int]` (Rust serializes `HashMap<u32, i32>` keys as strings). Python type for `damage_events` is `list[SinnerDamageEvent]`.

### SinnerDamageEvent

| Field | Type | Required | Notes |
|-------|------|----------|-------|
| `time_s` | int | yes | Match-relative second the event was observed |
| `player_slot` | int | yes | When `kind == "dealt"`: the attacking player's lobby slot (0-indexed). When `kind == "retaliated"`: the victim player's lobby slot (0-indexed). Note: this is a plain integer field, not a map key -- it serializes as a JSON number, not a string. This is distinct from `retaliation_damage` whose keys are strings because they come from a Rust `HashMap<u32, _>`. |
| `kind` | string | yes | One of `"dealt"` (player dealt damage to the sinner) or `"retaliated"` (sinner dealt damage back to the player). Serialized as snake_case from a Rust enum. |
| `damage` | int | yes | Raw damage amount from the underlying Valve damage event (i32 in Rust; may be negative in edge cases if Valve emits corrective events, though not observed in practice). |

**Pairing note:** A single melee punch currently produces one `"dealt"` event and one `"retaliated"` event at the same `time_s` for the same `player_slot`. This 1:1 coupling reflects Deadlock's current retaliation mechanics but is not guaranteed -- the event log shape is intentionally agnostic so Valve can change retaliation behavior without requiring a schema change.

---

## MidBossData

Mid-boss spawn, kill, health, and rejuv buff tracking. Present on every parse -- arrays are empty if the mid-boss never spawned or was never engaged.

| Field | Type | Required | Notes |
|-------|------|----------|-------|
| `boss_name_hash` | string | yes | `fxhash::hash_bytes(b"CNPC_MidBoss")` as u64 string. Canonical type identifier per Boss Type Identification table. |
| `spawn_events` | MidBossSpawnEvent[] | yes | One per spawn cycle; empty if mid-boss never spawned. Per-cycle `max_health` lives on each entry -- there is no match-global `max_health` because `m_iMaxHealth` scales with match time. |
| `kill_events` | MidBossKillEvent[] | yes | One per kill; empty if mid-boss was never killed |
| `rejuv_events` | RejuvStatusEvent[] | yes | One per individual rejuv grant/consume/expire; empty if no events |
| `fight_windows` | FightWindow[] | yes | One per engagement; captures health progression only during active fights |
| `post_match` | MidBossPostMatch[] | yes | One record per kill cycle; populated by parser from collected kill events |

### MidBossSpawnEvent

| Field | Type | Required | Notes |
|-------|------|----------|-------|
| `spawn_cycle` | int | yes | 1-indexed spawn cycle number; links spawn to its corresponding kill |
| `spawn_time_s` | float | yes | Match-relative time derived from `ctx.tick()` at `MidBossSpawned` (ID 349) |
| `max_health` | int | yes | `m_iMaxHealth` read on the `CNPC_MidBoss` CREATE paired with this spawn. Static for this cycle's entity lifetime. Scales per cycle: `13000 + 195 * (spawn_time_s / 60)`, probe-validated within 0.7% across three cycles (2026-04-16, `private/engineering/tools/probe_midboss_health.rs`). |

### MidBossKillEvent

Teams use `ECitadelLobbyTeam` (2=Amber, 3=Sapphire). Attribution window, derivation rules, and Valve-divergence rationale live in [`references.md`](references.md).

| Field | Type | Required | Notes |
|-------|------|----------|-------|
| `spawn_cycle` | int | yes | Which spawn cycle this kill ended; matches `MidBossSpawnEvent.spawn_cycle` |
| `team_killed` | int | yes | Team that landed the killing blow. Sourced from `RejuvStatus.killing_team` -- see [references.md#midbosskillevent-team_killed-source](references.md#midbosskillevent-team_killed-source). |
| `team_claimed` | int | yes | Team that won majority of rejuv grants (strict `>= 2` of 3). Diverges from Valve's blob -- see [references.md#midbosskillevent-team_claimed-derivation](references.md#midbosskillevent-team_claimed-derivation). |
| `rejuvs_by_team` | object<string, int> | yes | Raw grant counts keyed by team as string (`"2"`, `"3"`); both keys always present. See [references.md#midbosskillevent-rejuvs_by_team-shape](references.md#midbosskillevent-rejuvs_by_team-shape). |
| `matchtime_s` | float | yes | Match time of the `BossKilled` message. Note: attribution uses the fight window's last-damage time, not this value -- see [references.md#midbosskillevent-attribution-window](references.md#midbosskillevent-attribution-window). |
| `x` | float | yes | `entity_position.x` from `BossKilled` |
| `y` | float | yes | `entity_position.y` from `BossKilled` |
| `z` | float | yes | `entity_position.z` from `BossKilled` |
| `bosses_remaining` | int | yes | `bosses_remaining` from `BossKilled`. Passthrough of Valve protobuf value -- semantics unverified. # TODO: verify meaning from replay data |

### RejuvStatusEvent

| Field | Type | Required | Notes |
|-------|------|----------|-------|
| `player_pawn` | int | yes | EHandle of the player pawn that received the buff |
| `user_team` | int | yes | Team of the player who received the buff |
| `killing_team` | int | yes | Team that killed the mid-boss |
| `matchtime_s` | float | yes | Match time when this event fired |
| `event_type` | int | yes | 6=buff granted, 7=buff consumed (revive), 8=buff expired. Filter on `event_type == 6` for claim tracking. |

### FightWindow

A fight window captures one continuous engagement with the mid-boss. Windows open on first damage and close after 5 seconds of no damage or on boss death. Only records data during active interactions -- empty time between engagements is not stored.

| Field | Type | Required | Notes |
|-------|------|----------|-------|
| `spawn_cycle` | int | yes | Which spawn cycle this window belongs to |
| `window_start_s` | float | yes | Match time of first damage event in this engagement |
| `window_end_s` | float | yes | Match time of last damage event (or death) in this engagement |
| `health_at_start` | int | yes | `m_iHealth` at window open |
| `health_at_end` | int | yes | `m_iHealth` at window close; 0 if this window ends in a kill |
| `health_samples` | HealthSample[] | yes | Sparse samples within this window only |

### HealthSample

| Field | Type | Required | Notes |
|-------|------|----------|-------|
| `time_s` | float | yes | Match-relative time in seconds |
| `health` | int | yes | Health value at this sample; 0 at death |

### MidBossPostMatch

One record per kill cycle, derived by the parser from collected kill events. Pure summary of the parser's own derivation -- `team_claimed` is intentionally absent so callers who want the "who won this fight" verdict go to `MidBossKillEvent` (and Valve's raw number stays at `match_metadata.match_info.mid_boss[].team_claimed`).

| Field | Type | Required | Notes |
|-------|------|----------|-------|
| `team_killed` | int | yes | Team that landed the killing blow (from `MidBossKillEvent.team_killed`) |
| `rejuvs_by_team` | object<string, int> | yes | Raw grant counts for this kill (from `MidBossKillEvent.rejuvs_by_team`) |
| `destroyed_time_s` | int | yes | Kill time in match seconds (truncated from `MidBossKillEvent.matchtime_s`) |
