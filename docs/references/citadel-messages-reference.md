# Citadel Messages Reference
**Sources:** deadlock-api/valveprotos-rs (branch: master)
- `protos/deadlock/citadel_usermessages.proto` -- message definitions and `CitadelUserMessageIds` enum (IDs 300-366)
- `protos/deadlock/citadel_gcmessages_common.proto` -- shared enum definitions embedded in message entries. Full field-level catalog of that proto (including the `CMsgMatchMetaDataContents` post-match blob schema accessed via ID 316) lives in `citadel-gcmessages-common-reference.md`.

**Last Fetched:** 2026-03-17
**Last Verified:** 2026-04-09 (via `probe_all_messages` against 3 replays: `55423930_379917638`, `55841493_649180947`, `68175583_527726523`)
**Purpose:** Catalog of available Citadel protobuf messages for replay parsing. Used by parser agents implementing new message listeners.

---

## Outstanding TODOs

- **Backfill confidence labels** (`confirmed` / `inferred` / `hypothesis`) per field on all message entries below. The confidence-labeling convention in `.claude/rules/shared/research.md` was added after this doc was created. New sibling doc `citadel-gcmessages-common-reference.md` already follows the convention and can serve as the reference format.
- ~~**Run a decode probe** on `CCitadelUserMsg_PostMatchDetails` (ID 316)~~ -- **DONE 2026-04-10.** Probe run, schema validated, findings incorporated into ID 316 entry and sibling doc. See `private/engineering/tools/probe_post_match_details.rs` for the canonical probe.
- **Validate `CMsgMatchPlayerPathsData` presence** -- absent in all 3 test replays (2026-04-10). Re-run `probe_post_match_details` against newer replays (post 2026-04) to confirm whether path data is present or permanently stripped from the patched variant.

---

## Verification Status

Each catalog entry below carries a `**Status:**` line marking whether the message was actually observed in `on_packet` across the 3 verification replays (aggregated counts). Messages marked `NOT SEEN` exist in the proto/enum but did not arrive in any replay tested -- they may be client-only, removed, mode-gated, or pre-match-only. Treat them as unverified: do **not** build features on a `NOT SEEN` message without first confirming it fires in a newer replay.

**Summary (26 of 62 IDs verified, 36 NOT SEEN):**

| Status | Count | IDs |
|---|---|---|
| VERIFIED | 26 | 300, 303, 308, 309, 312, 314, 316, 317, 319, 332, 338, 340, 341, 346, 347, 348, 349, 350, 351, 352, 353, 356, 360, 361, 365, 500 |
| NOT SEEN | 36 | 304, 306, 310, 311, 313, 315, 318, 320, 321, 322, 323, 324, 325, 326, 327, 329, 330, 331, 333, 334, 336, 337, 339, 342, 343, 344, **345**, 354, 355, 357, 358, 359, 362, 363, 364, 366 |

**Critical gaps** -- messages with strong product alignment that did NOT arrive:
- `CurrencyChanged` (345) -- existing handlers in `parser/src/bin/probe_currency_changed.rs` and `parser/src/replay_parser.rs` subscribe to this ID but it never fires. **Treat the existing code as dead until this is re-verified on fresh replays.**
- `GoldHistory` (313), `RecentDamageSummary` (310), `DeathReplayData` (333), `PlayerLifetimeStatInfo` (334), `GetDamageStatsResponse` (339) -- the entire "summary / post-fight stats" category is absent. `PostMatchDetails` (316) is the only summary that fires (1 event per replay).
- `ObjectiveMask` (324), `ReturnIdol` (320) -- absent; game-phase detection should rely on `BossKilled` (347) + `MidBossSpawned` (349) instead.
- `BannedHeroes` (366), `HudGameAnnouncement` (363) -- absent.

**Re-run verification with:**
```
docker compose exec dashjump-parser cargo run --bin probe_all_messages -- \
  /parser/src/replays/<replay1>.dem /parser/src/replays/<replay2>.dem ...
```
(Probe lives at `private/engineering/tools/probe_all_messages.rs` -- copy into `parser/src/bin/` to run, then delete, per the existing probe convention.)

---

## Overview

Messages come from `protos/deadlock/citadel_usermessages.proto`. The enum `CitadelUserMessageIds` assigns numeric IDs to each message. These IDs are used in the `on_packet` callback to identify which message arrived.

There is one entity message category (`CitadelEntityMessageIds`) with a single entry.

The `on_packet` callback in haste receives `(packet_type: u32, data: &[u8])`. You match `packet_type` against the enum variant cast to `u32` to identify a message, then decode with prost.

---

## Enum: CitadelUserMessageIds

Complete current listing with numeric IDs:

| ID  | Enum Variant                              |
|-----|-------------------------------------------|
| 300 | k_EUserMsg_Damage                         |
| 303 | k_EUserMsg_MapPing                        |
| 304 | k_EUserMsg_TeamRewards                    |
| 306 | k_EUserMsg_AbilityFailed                  |
| 308 | k_EUserMsg_TriggerDamageFlash             |
| 309 | k_EUserMsg_AbilitiesChanged               |
| 310 | k_EUserMsg_RecentDamageSummary            |
| 311 | k_EUserMsg_SpectatorTeamChanged           |
| 312 | k_EUserMsg_ChatWheel                      |
| 313 | k_EUserMsg_GoldHistory                    |
| 314 | k_EUserMsg_ChatMsg                        |
| 315 | k_EUserMsg_QuickResponse                  |
| 316 | k_EUserMsg_PostMatchDetails               |
| 317 | k_EUserMsg_ChatEvent                      |
| 318 | k_EUserMsg_AbilityInterrupted             |
| 319 | k_EUserMsg_HeroKilled                     |
| 320 | k_EUserMsg_ReturnIdol                     |
| 321 | k_EUserMsg_SetClientCameraAngles          |
| 322 | k_EUserMsg_MapLine                        |
| 323 | k_EUserMsg_BulletHit                      |
| 324 | k_EUserMsg_ObjectiveMask                  |
| 325 | k_EUserMsg_ModifierApplied                |
| 326 | k_EUserMsg_CameraController               |
| 327 | k_EUserMsg_AuraModifierApplied            |
| 329 | k_EUserMsg_ObstructedShotFired            |
| 330 | k_EUserMsg_AbilityLateFailure             |
| 331 | k_EUserMsg_AbilityPing                    |
| 332 | k_EUserMsg_PostProcessingAnim             |
| 333 | k_EUserMsg_DeathReplayData                |
| 334 | k_EUserMsg_PlayerLifetimeStatInfo         |
| 336 | k_EUserMsg_ForceShopClosed                |
| 337 | k_EUserMsg_StaminaConsumed                |
| 338 | k_EUserMsg_AbilityNotify                  |
| 339 | k_EUserMsg_GetDamageStatsResponse         |
| 340 | k_EUserMsg_ParticipantStartSoundEvent     |
| 341 | k_EUserMsg_ParticipantStopSoundEvent      |
| 342 | k_EUserMsg_ParticipantStopSoundEventHash  |
| 343 | k_EUserMsg_ParticipantSetSoundEventParams |
| 344 | k_EUserMsg_ParticipantSetLibraryStackFields|
| 345 | k_EUserMsg_CurrencyChanged                |
| 346 | k_EUserMsg_GameOver                       |
| 347 | k_EUserMsg_BossKilled                     |
| 348 | k_EUserMsg_BossDamaged                    |
| 349 | k_EUserMsg_MidBossSpawned                 |
| 350 | k_EUserMsg_RejuvStatus                    |
| 351 | k_EUserMsg_KillStreak                     |
| 352 | k_EUserMsg_TeamMsg                        |
| 353 | k_EUserMsg_PlayerRespawned                |
| 354 | k_EUserMsg_CallCheaterVote                |
| 355 | k_EUserMsg_MeleeHit                       |
| 356 | k_EUserMsg_FlexSlotUnlocked               |
| 357 | k_EUserMsg_SeasonalKill                   |
| 358 | k_EUserMsg_MusicQueue                     |
| 359 | k_EUserMsg_AG2ParamTrigger                |
| 360 | k_EUserMsg_ItemPurchaseNotification       |
| 361 | k_EUserMsg_EntityPortalled                |
| 362 | k_EUserMsg_StreetBrawlScoring             |
| 363 | k_EUserMsg_HudGameAnnouncement            |
| 364 | k_EUserMsg_ItemDraftReaction              |
| 365 | k_EUserMsg_ImportantAbilityUsed           |
| 366 | k_EUserMsg_BannedHeroes                   |

## Enum: CitadelEntityMessageIds

| ID  | Enum Variant                          |
|-----|---------------------------------------|
| 500 | k_EEntityMsg_BreakablePropSpawnDebris |

---

## Message Catalog

### CCitadelUserMessage_Damage (ID: 300)
**Status:** VERIFIED (229,331 events across 3 replays; ~76k/match)
**Category:** UserMessage
**Fields:**
- `damage: int32` -- actual damage dealt
- `pre_damage: float` -- damage before mitigation (replaces deprecated `pre_damage_deprecated`)
- `type: int32` -- damage type enum
- `citadel_type: int32` -- citadel-specific damage type
- `origin: CMsgVector` -- world position of damage event
- `entindex_victim: int32` -- victim entity index
- `entindex_inflictor: int32` -- inflictor entity index
- `entindex_attacker: int32` -- attacker entity index
- `entindex_ability: int32` -- ability entity index
- `damage_absorbed: float` -- absorbed damage (replaces deprecated `damage_absorbed_deprecated`)
- `victim_health_max: int32`
- `victim_health_new: int32`
- `flags: uint64`
- `ability_id: uint32`
- `attacker_class: uint32`
- `victim_class: uint32`
- `victim_shield_max: int32`
- `victim_shield_new: int32`
- `hits: int32`
- `health_lost: int32`
- `hitgroup_id: int32`
- `entindex_attacking_object: int32`
- `damage_direction: CMsgVector`
- `is_secondary_stat: bool`
- `effectiveness: float`
- `crit_damage: float`
- `server_tick: int32`
**Product Alignment:** Objective damage breakdown, fight classification (detect engagements by damage bursts), hero matchup damage attribution.
**Data Richness:** High -- per-hit resolution with attacker/victim/ability IDs, positions, and health deltas.

---

### CCitadelUserMsg_AbilitiesChanged (ID: 309)
**Status:** VERIFIED (1,115 events across 3 replays)
**Category:** UserMessage
**Fields:**
- `entindex_purchaser: int32` -- entity index of the purchasing hero pawn (not a player slot; resolve via entity lookup)
- `entindex_ability: int32` -- entity index of the ability
- `ability_id: uint32`
- `change: Change` -- `EPurchased=0`, `EUpgraded=1`, `ESold=2`, `ESwappedActivatedAbility=3`, `EFailure=4`
**Product Alignment:** Hero matchup by player (ability build tracking -- what abilities a player buys, upgrades, or sells and when). Supplements `ItemPurchaseNotification` (360) which covers items; this covers ability upgrades specifically.
**Data Richness:** Medium -- buy/sell/upgrade type with player slot and ability ID.

---

### CCitadelUserMsg_RecentDamageSummary (ID: 310)
**Status:** NOT SEEN in 3 verification replays (2026-04-09) -- spec-only; do not rely on this message without re-verifying.
**Category:** UserMessage
**Fields:**
- `player_slot: int32`
- `damage_records: repeated DamageRecord`
  - `damage: int32`, `hits: int32`, `damage_type: uint32`, `hero_id: uint32`, `ability_id: uint32`, `attacker_class: uint32`, `damage_absorbed: float`, `is_killing_blow: bool`, `victim_hero_id: uint32`, `is_secondary_stat: bool`, `pre_damage: float`, `crit_damage: float`
- `modifier_records: repeated ModifierRecord`
  - `ability_id: uint32`, `modifier_type_id: uint32`, `entindex_caster: int32`, `start_time: float`, `end_time: float`, `debuff: bool`
- `start_time: float`, `end_time: float`, `total_damage: int32`, `lost_gold: int32`
**Product Alignment:** Objective damage breakdown (post-fight summary per player), fight classification. Complements the per-hit Damage message.
**Data Richness:** High -- aggregated fight window with modifier timeline and ability breakdown.

---

### CCitadelUserMsg_GoldHistory (ID: 313)
**Status:** NOT SEEN in 3 verification replays (2026-04-09) -- spec-only. `CurrencyChanged` (345) was also NOT SEEN, so there is currently **no verified gold/currency message** in replays. This is a critical gap for any lane-pressure or gold-curve feature.
**Category:** UserMessage
**Fields:**
- `entindex_player: int32`
- `minute_records: repeated MinuteRecord`
  - `match_minute: int32`
  - `gold_records: repeated GoldRecord` (each: `currency_source: int32`, `gold: int32`, `events: int32`)
    - `currency_source` decodes via `EGoldSource` enum (see `CurrencyChanged` entry)
    - `events` = number of transactions that minute for that source
**Product Alignment:** Lane pressure tracking (gold delta between teams indicates lane dominance), game phase detection (gold curves shift at phase transitions).
**Data Richness:** Medium -- per-minute bucketed, not per-event resolution. Use `CurrencyChanged` (ID 345) when per-event granularity is needed.

---

### CCitadelUserMsg_PostMatchDetails (ID: 316)
**Status:** VERIFIED (1 event per replay -- fires exactly once at match end; this is the *only* post-match summary message that arrives, since 310/313/333/334/339 are all NOT SEEN).
**Decode probe:** Verified 2026-04-10 via `probe_post_match_details` against 3 replays. Full schema confirmed decodable. See `citadel-gcmessages-common-reference.md` for per-field findings.
**Category:** UserMessage
**Fields:**
- `match_details: bytes` -- raw bytes that decode directly as `CMsgMatchMetaDataContentsPatched` (no intermediate `CMsgMatchMetaData` envelope -- confirmed by probe 2026-04-10)
**Decode path (corrected):** `CCitadelUserMsg_PostMatchDetails.match_details` -> decode directly as `CMsgMatchMetaDataContentsPatched`. The `CMsgMatchMetaData` intermediate envelope mentioned in earlier notes does NOT apply to the wire format.
**Rust type name:** `CMsgMatchMetaDataContentsPatched` (confirmed in prost-generated code; `patch.proto` source uses trailing `l` but prost normalizes it).
**Key probe findings:**
- 12 players per match, all fields populated (kills/deaths/assists/net_worth/hero_id/level/last_hits/denies/ability_points/assigned_lane/party/team confirmed present)
- `assigned_lane` uses `CMsgLaneColor` encoding (1=Yellow, 4=Blue, 6=Purple -- NOT sequential)
- `PlayerStats`: 10-13 snapshots per player; nominal 300s interval, first snapshot at 180s, last aligns to match end
- `CMsgMatchPlayerPathsData` absent in all 3 replays (match_paths=None) -- may be version-gated or stripped from patched variant
- `CMsgMatchPlayerDamageMatrix`: 180s sample cadence, 13 dealers per match, source_name strings confirmed as human-readable ability names
- `mid_boss` steal confirmed: `team_killed != team_claimed` observed in one replay
- `legacy_objectives_mask` absent (None) in all replays -- use `objectives_mask_team0/1` (uint64) instead
- `party` field: present as 0/1/2 when players queue in parties; None in solo-queue replays
**Inner schema:** See `citadel-gcmessages-common-reference.md` Section 2 for the full `CMsgMatchMetaDataContents` catalog with all open questions resolved.
**Product Alignment:** All analytics features -- end-of-match summary blob. The inner schema is extremely rich: `PlayerStats` time-series with CS% denominator (`possible_creeps`), per-death `time_to_kill_s` and positions, per-objective damage split by type, and pairwise damage matrix. Path data (`CMsgMatchPlayerPathsData`) is absent from test replays and unvalidated.
**Data Richness:** Very high once decoded. The blob unlocks lane pressure (CS%), fight classification (pairwise damage matrix), and objective damage breakdown (spirit/bullet split) -- all previously blocked by `NOT SEEN` live messages.

---

### CCitadelUserMsg_HeroKilled (ID: 319)
**Status:** VERIFIED (247 events across 3 replays; ~82 kills/match)
**Category:** UserMessage
**Fields:**
- `entindex_victim: int32`
- `entindex_inflictor: int32`
- `entindex_attacker: int32`
- `entindex_assisters: repeated int32`
- `entindex_scorer: int32`
- `respawn_reason: int32`
- `victim_team_number: int32`
**Product Alignment:** Fight classification (1v1 vs skirmish vs teamfight by counting participants + assisters), kill streak tracking, hero matchup by player.
**Data Richness:** Medium -- who killed whom with assisters, but no position or timing data directly.

---

### CCitadelUserMsg_ReturnIdol (ID: 320)
**Status:** NOT SEEN in 3 verification replays (2026-04-09). Spec-only.
**Category:** UserMessage
**Fields:**
- `location_index: int32`
- `return_location: CMsgVector`
- `location_enabled: bool`
**Product Alignment:** Game phase detection (idol return events mark mid-game transitions), walker first contact context.
**Data Richness:** Low -- binary event with position.

---

### CCitadelUserMessage_BulletHit (ID: 323)
**Status:** NOT SEEN in 3 verification replays (2026-04-09). Spec-only. `GE_BulletImpact` (461) from `citadel-messages-supplemental.md` may be the runtime replacement; also NOT SEEN here but documented as the richer alternative.
**Category:** UserMessage
**Fields:**
- `shotid: int32`
- `pellet: int32`
- `hit_entindex: int32`
- `weapon_entindex: int32`
- `is_predicted: bool`
**Product Alignment:** No direct alignment with current coach analytics features. Could feed accuracy stats in future.
**Data Richness:** Low -- minimal hit confirmation, no damage values.

---

### CCitadelUserMessage_ObjectiveMask (ID: 324)
**Status:** NOT SEEN in 3 verification replays (2026-04-09). Spec-only. Game-phase detection should rely on `BossKilled` (347) and `MidBossSpawned` (349) instead, both of which are VERIFIED.
**Category:** UserMessage
**Fields:**
- `objective_mask_team0: uint64` -- bitmask of objective states for team 0
- `objective_mask_team1: uint64` -- bitmask of objective states for team 1
**Product Alignment:** Game phase detection (guardians/walkers destroyed changes phase), walker first contact (detect when walker state changes in bitmask).
**Data Richness:** Medium -- compact bitmask requires mapping against `ECitadelObjective` enum from `citadel_gcmessages_common.proto` to decode individual objectives.

**ECitadelObjective mapping (from citadel_gcmessages_common.proto):**
Tier1 = Guardians (lanes 1-4 per team), Tier2 = Walkers (lanes 1-4 per team), Titan + TitanShieldGenerators, BarrackBoss per lane, Neutral_Mid.

---

### CCitadelUserMsg_BossKilled (ID: 347)
**Status:** VERIFIED (68 events across 3 replays; ~23 boss kills/match -- matches roughly 8 guardians + 8 walkers + mid boss + titan objectives)
**Category:** UserMessage
**Fields:**
- `objective_team: int32` -- which team owns/killed the boss
- `objective_mask_change: int32` -- what changed in the objective mask
- `entity_killed: uint32` -- ehandle of killed entity
- `entity_killed_class: int32`
- `entity_killer: uint32` -- ehandle of killer
- `gametime: float` -- match time of kill
- `bosses_remaining: int32`
- `entity_position: CMsgVector`
**Product Alignment:** Game phase detection (Sinners/mid boss kills mark phase transitions), walker first contact (walkers killed here). This is the primary event for detecting laning-to-midgame transition.
**Data Richness:** High -- precise game time, position, team context, bosses remaining counter.

---

### CCitadelUserMsg_BossDamaged (ID: 348)
**Status:** VERIFIED (790 events across 3 replays). NOTE: ID 348 is NOT present in the current `deadlock-api/valveprotos-rs` rev (`9625c07`) enum -- decoding requires bumping that dep or decoding manually.
**Category:** UserMessage
**Fields:**
- `objective_team: int32`
- `objective_id: int32`
- `entity_damaged: uint32`
**Product Alignment:** Walker first contact (detect first hit on a walker before it dies), objective damage breakdown.
**Data Richness:** Low -- confirms damage occurred, no amount or attacker identity.

---

### CCitadelUserMsg_MidBossSpawned (ID: 349)
**Status:** VERIFIED (8 events across 3 replays -- ~2-3 per match, consistent with mid-boss respawn cadence). Not in the current valveprotos-rs enum (same caveat as 348).
**Category:** UserMessage
**Fields:** (empty message -- no fields)
**Product Alignment:** Game phase detection -- the Sinner spawn event. This is the trigger for laning -> mid-game phase transition. Pair with `BossKilled` to know when mid boss dies.
**Data Richness:** Low -- binary event only, no fields. Timing comes from `ctx.tick()` in haste.

---

### CCitadelUserMsg_RejuvStatus (ID: 350)
**Status:** VERIFIED (48 events across 3 replays; ~16/match). Not in current valveprotos-rs enum.
**Category:** UserMessage
**Fields:**
- `killing_team: int32`
- `player_pawn: uint32`
- `user_team: int32`
- `event_type: int32`
**Product Alignment:** Game phase detection (rejuvenation status correlates with mid/late boss kill cycles).
**Data Richness:** Low -- event type enum values not documented in proto.

---

### CCitadelUserMsg_KillStreak (ID: 351)
**Status:** VERIFIED (348 events across 3 replays). Not in current valveprotos-rs enum.
**Category:** UserMessage
**Fields:**
- `player_pawn: uint32`
- `num_kills: int32`
- `is_first_blood: bool`
- `streak_ended: bool`
- `duration: float` (default 5s)
**Product Alignment:** Fight classification (kill streaks indicate sustained aggression), hero matchup pressure tracking.
**Data Richness:** Medium -- streak count and first blood flag with duration context.

---

### CCitadelUserMessage_MeleeHit (ID: 355)
**Status:** NOT SEEN in 3 verification replays (2026-04-09). Spec-only. Also not in current valveprotos-rs enum.
**Category:** UserMessage
**Fields:**
- `hit_entindex: int32`
- `heavy: bool`
**Product Alignment:** No direct alignment with current features. Could feed combat style classification.
**Data Richness:** Low -- minimal.

---

### CCitadelUserMsg_FlexSlotUnlocked (ID: 356)
**Status:** VERIFIED (18 events across 3 replays; exactly 6/match -- matches flex slot unlock progression). Not in current valveprotos-rs enum.
**Category:** UserMessage
**Fields:**
- `team_number: int32`
- `flexslot_unlocked: int32`
**Product Alignment:** Game phase detection (flex slot unlocks correlate with game progression milestones).
**Data Richness:** Low -- binary event.

---

### CCitadelUserMsg_GetDamageStatsResponse (ID: 339)
**Status:** NOT SEEN in 3 verification replays (2026-04-09). Spec-only. Name suggests a request/response mechanic -- may only fire when a client explicitly requests stats, not during passive recording.
**Category:** UserMessage
**Fields:**
- `player_slot: uint32`
- `ability_name: string`
- `damage: StatType` -- packed arrays: `target_player_slot[]`, `value[]`
- `healing: StatType` -- packed arrays: `target_player_slot[]`, `value[]`
**Product Alignment:** Objective damage breakdown (ability-level damage breakdown per target player), hero matchup by player.
**Data Richness:** High -- per-ability, per-target damage and healing stats keyed by player slot.

---

### CCitadelUserMsg_DeathReplayData (ID: 333)
**Status:** NOT SEEN in 3 verification replays (2026-04-09). Spec-only. Consistent with the rest of the summary-message category being absent -- see PostMatchDetails (316) note.
**Category:** UserMessage
**Fields:**
- `killer_scorer: int32` -- entity index of the scoring killer
- `killer_inflictor: int32` -- entity index of the killing ability/weapon
- `damage_summary: CCitadelUserMsg_RecentDamageSummary` -- full embedded damage summary (see ID 310)
**Product Alignment:** Fight classification (all-in-one death event: killer identity + full damage breakdown in one message). Fires at death and bundles the `RecentDamageSummary` inline, which is a convenient alternative to correlating `HeroKilled` (319) + `RecentDamageSummary` (310) separately.
**Data Richness:** High -- killer attribution plus the full damage window embedded.

---

### CCitadelUserMsg_PlayerLifetimeStatInfo (ID: 334)
**Status:** NOT SEEN in 3 verification replays (2026-04-09). Spec-only. Despite `end_of_match: bool` field suggesting an end-of-match fire, the message does not arrive.
**Category:** UserMessage
**Fields:**
- `stats: repeated Stat`
  - `stat_name: string`, `match_total: uint32`, `lifetime_value: uint32`, `priority: uint32`, `prev_lifetime_max: uint32`, `stat_type: uint32`, `stat_type_id: uint32`
- `match_id: uint64`
- `end_of_match: bool`
- `is_official_match: bool`
**Product Alignment:** Hero matchup by player (per-player match stats), post-match analytics.
**Data Richness:** High -- named stats with match totals and lifetime context.

---

### CCitadelUserMsg_StaminaConsumed (ID: 337)
**Status:** NOT SEEN in 3 verification replays (2026-04-09). Spec-only. NOTE: The generated Rust enum names this variant `KEUserMsgStaminaDrained` -- the spec name `StaminaConsumed` was renamed in a more recent proto rev but is still accurate for the ID slot.
**Category:** UserMessage
**Fields:**
- `entindex_target: int32`
- `stamina_before: float`
- `stamina_after: float`
- `drained: bool`
- `stamina_max: float`
- `gametime: float`
**Product Alignment:** No direct alignment with current coach features. Could feed combat pressure / chase analysis.
**Data Richness:** Medium -- precise stamina delta with game time.

---

### CCitadelUserMessage_CurrencyChanged (ID: 345)
**Status:** NOT SEEN in 3 verification replays (2026-04-09). **CRITICAL GAP:** The existing handlers at `parser/src/bin/probe_currency_changed.rs` and `parser/src/replay_parser.rs` subscribe to this ID but it never fires. Treat that code as dead until re-verified on a fresh replay. No verified gold/currency message currently exists in replays -- `GoldHistory` (313) is also NOT SEEN.
**Category:** UserMessage
**Fields:**
- `entindex_hero_pawn: int32` -- entity index of the hero pawn (NOT a userid or player slot; resolve via `m_hOwnerEntity` → controller → `m_unLobbyPlayerSlot` chain)
- `currency_type: int32`
- `currency_source: int32` -- see `EGoldSource` enum below
- `delta: int32` -- amount changed (positive = earned, negative = spent)
- `notification: bool`
- `entindex_victim: int32` -- relevant when source is kills/assists
- `victim_pos: CMsgVector`
- `playsound: int32`
- `ability_id: uint32`

**Note:** `new_value` (running total) does NOT exist on this message. Soul balance must be accumulated from `delta` values. `userid` is also absent -- use `entindex_hero_pawn` for player resolution.

**Product Alignment:** Lane pressure tracking (gold flow on kills/assists/objectives), hero matchup by player (gold earned from specific opponents), deny tracking.
**Data Richness:** High -- per-transaction with source and delta. More granular than GoldHistory.

**Implementation notes (building soon):**
- Subscribe in `on_packet` matching `CitadelUserMessageIds::KEUserMsgCurrencyChanged as u32`
- Use async Visitor pattern (not sync); see `replay_parser.rs:299-513` for the current API surface
- Filter by `currency_source` using `EGoldSource` values to isolate creep gold, deny gold, etc.
- Accumulate `delta` to derive running totals -- no `new_value` field available
- `entindex_victim` is set for kill/assist sources; use with entity lookup to get victim hero ID
- Events fire for both teams -- key by `entindex_hero_pawn` to attribute to the correct player

### EGoldSource Enum (currency_source field decoder)

Used in `CCitadelUserMessage_CurrencyChanged.currency_source`, `CCitadelUserMsg_GoldHistory.gold_records[].currency_source`, and `CMsgMatchMetaDataContents.GoldSource.source` (post-match blob).

**Canonical definition:** See `citadel-gcmessages-common-reference.md` Section 1, `CMsgMatchMetaDataContents.EGoldSource`. The enum is defined in `citadel_gcmessages_common.proto` and shared across both live replay messages and the post-match blob.

**Key values for lane pressure features:**
- `LaneCreeps = 2` -- direct measure of last-hit efficiency per lane
- `Denies = 7` -- enables deny tracking without entity-level detection; fires when a player denies a creep
- `Players = 1` + `Assists = 6` -- kill gold flow, secondary lane pressure signal

---

### CCitadelUserMsg_ParticipantStartSoundEvent (ID: 340)
**Status:** VERIFIED (176,466 events across 3 replays; by far the highest-volume Citadel message). High-frequency audio-system chatter -- cheap to tally but likely not load-bearing for gameplay features.
**Category:** UserMessage
**Fields:**
- `event: CMsgSosStartSoundEvent`
- `player_slots: repeated int32`
**Product Alignment:** No direct alignment.
**Data Richness:** Low -- audio system event.

---

### CCitadelEntityMsg_BreakablePropSpawnDebris (Entity ID: 500)
**Status:** VERIFIED (10,293 events across 3 replays). The only Citadel **entity** message in the spec, and it fires heavily -- breakable props are common in Deadlock maps.
**Category:** EntityMessage
**Fields:**
- `entity_msg: CEntityMsg`
- `damage_pos: CMsgVector`
- `damage: float`
- `damage_force: CMsgVector`
**Product Alignment:** No direct alignment.
**Data Richness:** Low -- physics debris event.

---

### CCitadelUserMsg_TeamMsg (ID: 352)
**Status:** VERIFIED (37 events across 3 replays; ~12/match). Not in current valveprotos-rs enum. Usable for lane-color tagged team events.
**Category:** UserMessage
**Fields:**
- `event_type: int32`
- `team_number: int32`
- `lane_color: int32` -- corresponds to CMsgLaneColor enum (Yellow=1, Green=3, Blue=4, Purple=6)
- `player_controller: uint32`
**Product Alignment:** Lane priority tracking (lane-associated team events), solo time (player controller involved in team events).
**Data Richness:** Medium -- lane color field directly maps to the 4-lane structure.

---

### CCitadelUserMessage_GameOver (ID: 346)
**Status:** VERIFIED (3 events across 3 replays -- exactly once per match, as expected).
**Category:** UserMessage
**Fields:**
- `winning_team: int32`
- `just_a_test: bool`
**Product Alignment:** Game phase detection (marks end of match), frame for all phase durations.
**Data Richness:** Low -- binary result event.

---

### CCitadelUserMsg_PlayerRespawned (ID: 353)
**Status:** VERIFIED (252 events across 3 replays; ~84/match). Not in current valveprotos-rs enum. Pairs with `HeroKilled` (319) for death/respawn timeline reconstruction.
**Category:** UserMessage
**Fields:**
- `player_pawn: uint32`
- `facing_yaw: float`
**Product Alignment:** Solo time tracking (respawn timing feeds dead time, affects solo map presence calculations).
**Data Richness:** Low -- respawn event with facing direction.

---

### CCitadelUserMessage_ItemPurchaseNotification (ID: 360)
**Status:** VERIFIED (868 events across 3 replays; ~290/match). Not in current valveprotos-rs enum. This is the primary item-build tracking message.
**Category:** UserMessage
**Fields:**
- `userid: int32`
- `ability_id: uint32`
- `sell: bool`
- `quickbuy: bool`
**Product Alignment:** Hero matchup by player (item build tracking per player).
**Data Richness:** Medium -- buy/sell events per item.

---

### CCitadelUserMsg_HudGameAnnouncement (ID: 363)
**Status:** NOT SEEN in 3 verification replays (2026-04-09). Spec-only; not in current valveprotos-rs enum either. Phase-detection features should rely on `MidBossSpawned` (349) + `BossKilled` (347) instead.
**Category:** UserMessage
**Fields:**
- `title_locstring: string`
- `description_locstring: string`
- `classname: repeated string`
- `dialog_variable_name: repeated string`
- `dialog_variable_locstring: repeated string`
**Product Alignment:** Game phase detection (HUD announcements mark major game events like boss spawns, phase transitions). Localization strings may encode event type.
**Data Richness:** Medium -- human-readable event labels, requires parsing locstring keys.

---

## Product Alignment Summary -- Priority Targets

For the Q1/Q2 2026 coach analytics roadmap. Legend: `[v]` = verified in replays, `[x]` = NOT SEEN in 2026-04-09 probe (do not rely on without re-verifying).

| Feature | Primary Messages | Secondary Messages |
|---------|-----------------|-------------------|
| Game phase detection | [v] `MidBossSpawned` (349), [v] `BossKilled` (347), [v] `GameOver` (346) | [x] `ObjectiveMask` (324), [x] `HudGameAnnouncement` (363), [x] `ReturnIdol` (320) |
| Lane priority tracking | [x] `CurrencyChanged` (345), [v] `TeamMsg` (352) | [x] `GoldHistory` (313), [x] `RecentDamageSummary` (310) |
| Solo time tracking | [v] `PlayerRespawned` (353) | [v] `HeroKilled` (319) -- cross-ref dead time |
| Fight classification | [v] `HeroKilled` (319), [v] `Damage` (300) | [v] `KillStreak` (351), [v] `ImportantAbilityUsed` (365), [x] `RecentDamageSummary` (310), [x] `DeathReplayData` (333), [x] `AbilityInterrupted` (318) |
| Walker first contact | [v] `BossKilled` (347), [v] `BossDamaged` (348) | [x] `ObjectiveMask` (324) |
| Objective damage breakdown | [v] `Damage` (300) | [x] `GetDamageStatsResponse` (339), [x] `RecentDamageSummary` (310) |
| Hero matchup by player | [v] `HeroKilled` (319), [v] `ItemPurchaseNotification` (360), [v] `AbilitiesChanged` (309) | [x] `GetDamageStatsResponse` (339), [x] `PlayerLifetimeStatInfo` (334) |
| Draft / ban analysis | [x] `BannedHeroes` (366) -- **no verified in-replay source; use Deadlock API** | -- |

**Coverage by feature:**
- **Fully verified:** Game phase detection, Solo time tracking, Walker first contact
- **Partially verified:** Fight classification (has core primaries), Hero matchup (lost the stats response)
- **Blocked:** Lane priority (no gold signal), Objective damage breakdown (only per-hit Damage, no aggregates), Draft/ban analysis (no in-replay source)

---

### CCitadelUserMessage_ImportantAbilityUsed (ID: 365)
**Status:** VERIFIED (7,293 events across 3 replays; ~2,400/match). Not in current valveprotos-rs enum. High-signal for fight classification -- Valve's own "important" filter already narrows to key actives and ultimates.
**Category:** UserMessage
**Fields:**
- `player: uint32` -- player entity handle
- `caster: uint32` -- caster entity handle (may differ from player for summons)
- `ability_name: string` -- human-readable ability name (e.g. `"citadel_ability_vindicta_snipe"`)
**Product Alignment:** Fight classification (game-flagged key ability usage -- likely ultimates and major actives). The `ability_name` string eliminates the need for ability ID lookup tables. Use to detect whether ultimates were used in a given engagement window.
**Data Richness:** Medium -- ability name + player identity, but no target, position, or outcome.

---

### CCitadelUserMsg_AbilityInterrupted (ID: 318)
**Status:** NOT SEEN in 3 verification replays (2026-04-09). Spec-only. Backlog item -- do not build on this without re-verifying.
**Category:** UserMessage
**Fields:**
- `entindex_victim: int32` -- player whose ability was interrupted
- `entindex_interrupter: int32` -- entity that caused the interrupt
- `ability_id_interrupted: uint32` -- which ability was cut off
- `ability_id_interrupter: uint32` -- ability used to interrupt (if applicable)
- `hero_id_interrupter: uint32` -- hero that performed the interrupt
**Product Alignment:** Fight classification and hero matchup -- interrupts are a fight-quality metric. Surfaces counter/disrupt patterns: which hero interrupted which ability, how often, and with what. Backlog item; not in current experiment scope.
**Data Richness:** Medium -- victim, interrupted ability, and interrupter hero all present. No position or timing.
**Alternative source:** Aggregate interrupt counts may be inferrable from the post-match blob's `CMsgMatchPlayerDamageMatrix` (see `citadel-gcmessages-common-reference.md` Section 2) via ability source names, but per-event interrupt records are only in this live message.

---

### CCitadelUserMsg_EntityPortalled (ID: 361)
**Status:** VERIFIED but **rare** (2 events total across 3 replays -- only fired in one replay). Not in current valveprotos-rs enum. Too sparse to drive a feature on its own; treat as a supplemental signal.
**Category:** UserMessage
**Fields:**
- `entity_portalled: uint32` -- entity handle of entity that passed through a portal (default 16777215 = invalid handle)
- `portal_transform: CMsgTransform` -- world transform of the portal
**Product Alignment:** Future positioning and mobility analysis -- portal usage per player/hero could contribute to solo-time inference (portal to safety) or fight engagement patterns. `entity_portalled` requires cross-referencing with player entity handles to attribute to a specific player.
**Data Richness:** Low -- entity handle + portal position. No direction, cooldown, or outcome.
**Alternative source:** Broader positioning/mobility context lives in the post-match blob's `CMsgMatchPlayerPathsData` (see `citadel-gcmessages-common-reference.md` Section 2), including per-sample `move_type` values with `InAir` / `AirDash` / `Ziplining` states. Portal-specific events remain exclusive to this live message.

---

### CCitadelUserMsg_BannedHeroes (ID: 366)
**Status:** NOT SEEN in 3 verification replays (2026-04-09). Spec-only; not in current valveprotos-rs enum. **Critical gap for draft/ban analysis** -- there is currently no verified in-replay source for hero bans. This may fire only in ranked/official modes, or require fetching draft data from the Deadlock API instead.
**Category:** UserMessage
**Fields:**
- `banned_hero_ids: repeated uint32` -- list of banned hero IDs for this match
**Product Alignment:** Draft / ban analysis -- authoritative source for hero bans per match. Fires once near match start. Directly feeds hero matchup and draft composition features.
**Data Richness:** Low -- single repeated field, no timestamps or per-ban team attribution.
**Alternative source:** Confirmed hero *picks* per player are available in the post-match blob via `CMsgMatchMetaDataContents.Players.hero_id` (see `citadel-gcmessages-common-reference.md` Section 2). Bans themselves are still only in this live message (ID 366) or via the GC-only `CMsgHeroSelectionMatchInfo.banned_heroes` (Section 3), neither of which was observed in the 2026-04-09 probe.

---

## Notes on Entity-Based Data vs. Messages

Not all game state is delivered via user messages. Entity state changes (via `on_entity`) carry:
- Player positions (via `CCitadelPlayerPawn.CBodyComponent.m_skeletonInstance.m_vecOrigin`)
- Creep positions and class names
- Objective health states

Solo time tracking in particular will likely require entity position polling (via `on_entity`) rather than a dedicated message, since there is no `PlayerAlone` message -- solo status must be inferred from proximity to allies.
