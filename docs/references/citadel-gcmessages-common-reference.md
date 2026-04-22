# citadel_gcmessages_common.proto -- Reference

**Source:** `protos/deadlock/citadel_gcmessages_common.proto`, repo `deadlock-api/valveprotos-rs`, branch `master`, commit `458c5e17c402953ab61baaef4a099a073cf01644`
**Last Fetched:** 2026-04-01
**Last Verified:** 2026-04-10 (via `probe_post_match_details` against 3 replays: `55423930_379917638`, `55841493_649180947`, `68175583_527726523`)
**Purpose:** Field-level catalog of all messages and enums in this proto, with product alignment for dashjump.gg analytics. Used by parser and backend agents accessing post-match blob data.

**Imports (not recursively fetched):** `steammessages.proto`, `gcsdk_gcmessages.proto`, `base_gcmessages.proto`, `valveextensions.proto`

---

## Critical: Two Data Paths

**GC messages (majority of this file)** are exchanged between game clients and the Steam Game Coordinator. They are NOT present in `.dem` replay files and cannot be subscribed to via haste `on_packet` or `on_entity` callbacks.

**Post-match blob (accessible from replays):** `CMsgMatchMetaDataContents` is embedded in every replay as the inner blob of `CCitadelUserMsg_PostMatchDetails` (ID 316). Decode path:
1. Subscribe to ID 316 in `on_packet`
2. Decode as `CCitadelUserMsg_PostMatchDetails`
3. Decode `match_details: bytes` **directly** as `CMsgMatchMetaDataContentsPatched` -- there is no intermediate `CMsgMatchMetaData` envelope step (confirmed by probe, 2026-04-10)

**Note on CMsgMatchMetaData:** The `CMsgMatchMetaData` struct (version + match_details + match_id) exists in the generated code but is NOT the actual wire format. The raw bytes from `CCitadelUserMsg_PostMatchDetails.match_details` decode directly as `CMsgMatchMetaDataContentsPatched`. Do not add an intermediate decode step.

The blob contains end-of-match summaries, not live tick data. It fires once near match end.

**Enumerations** defined in this file are reference types used by both GC and replay messages -- they are always accessible.

---

## Section 1: Enumerations

### `CMsgLaneColor`

**Confidence:** confirmed

| Value | Name |
|-------|------|
| 0 | `k_ELaneColor_Invalid` |
| 1 | `k_ELaneColor_Yellow` |
| 3 | `k_ELaneColor_Green` |
| 4 | `k_ELaneColor_Blue` |
| 6 | `k_ELaneColor_Purple` |

**Note:** Values are non-contiguous (gaps at 2 and 5). Do not assume sequential. The four valid lane colors map to Deadlock's 4-lane structure.

**Product alignment:** Lane pressure -- the canonical lane discriminator. Used in `CCitadelUserMsg_TeamMsg` (ID 352) `lane_color` field. Map Yellow=1, Green=3, Blue=4, Purple=6.

---

### `ECitadelMatchMode`

**Confidence:** confirmed

| Value | Name |
|-------|------|
| 0 | `k_ECitadelMatchMode_Invalid` |
| 1 | `k_ECitadelMatchMode_Unranked` |
| 2 | `k_ECitadelMatchMode_PrivateLobby` |
| 3 | `k_ECitadelMatchMode_CoopBot` |
| 4 | `k_ECitadelMatchMode_Ranked` |
| 5 | `k_ECitadelMatchMode_ServerTest` |
| 6 | `k_ECitadelMatchMode_Tutorial` |
| 7 | `k_ECitadelMatchMode_HeroLabs` |
| 8 | `k_ECitadelMatchMode_Calibration` |

**Product alignment:** All analytics features -- filter input data to modes 1 (Unranked) and 4 (Ranked) only. Exclude 7 (HeroLabs), 3 (CoopBot), and 5/6 (test/tutorial). Carried in `CMsgMatchMetaDataContents.MatchInfo.match_mode`.

---

### `ECitadelLobbyTeam`

**Confidence:** confirmed

| Value | Name |
|-------|------|
| 0 | `k_ECitadelLobbyTeam_Team0` |
| 1 | `k_ECitadelLobbyTeam_Team1` |
| 16 | `k_ECitadelLobbyTeam_Spectator` |

**Note:** Spectator = 16, not 2. All comparison code must handle this non-contiguous range.

**Product alignment:** Universal team discriminator -- used throughout `CMsgMatchMetaDataContents` for player, objective, and mid-boss team attribution.

---

### `ECitadelObjective`

**Confidence:** confirmed

| Range | Names | Category |
|-------|-------|----------|
| 0 | `k_eCitadelObjective_Team0_Core` | Team 0 Patron |
| 1--4 | `k_eCitadelObjective_Team0_Tier1_Lane1..4` | Team 0 Guardians |
| 5--8 | `k_eCitadelObjective_Team0_Tier2_Lane1..4` | Team 0 Walkers |
| 9 | `k_eCitadelObjective_Team0_Titan` | Team 0 Titan |
| 10--11 | `k_eCitadelObjective_Team0_TitanShieldGenerator_1..2` | Team 0 Shield generators |
| 12--15 | `k_eCitadelObjective_Team0_BarrackBoss_Lane1..4` | Team 0 Barracks bosses |
| 16 | `k_eCitadelObjective_Team1_Core` | Team 1 Patron |
| 17--20 | `k_eCitadelObjective_Team1_Tier1_Lane1..4` | Team 1 Guardians |
| 21--24 | `k_eCitadelObjective_Team1_Tier2_Lane1..4` | Team 1 Walkers |
| 25 | `k_eCitadelObjective_Team1_Titan` | Team 1 Titan |
| 26--27 | `k_eCitadelObjective_Team1_TitanShieldGenerator_1..2` | Team 1 Shield generators |
| 28--31 | `k_eCitadelObjective_Team1_BarrackBoss_Lane1..4` | Team 1 Barracks bosses |
| 32 | `k_eCitadelObjective_Neutral_Mid` | Neutral mid boss (Sinner) |

**Product alignment:** Game phase detection and objective damage breakdown. Decodes `legacy_objective_id` in `CMsgMatchMetaDataContents.Objective` and the bitmask in `CCitadelUserMessage_ObjectiveMask` (ID 324). Tier1 = Guardians; Tier2 = Walkers. Walkers destroyed marks laning-to-midgame transition. Value 32 identifies the Sinner mid-boss kill.

---

### `ECitadelTeamObjective`

**Confidence:** confirmed

Same structure as `ECitadelObjective` but team-relative (no `Team0_`/`Team1_` prefix). Used alongside a separate `team: ECitadelLobbyTeam` field.

**Product alignment:** Objective damage breakdown -- prefer this for per-team objective charts because it is team-neutral by design. Used in `CMsgMatchMetaDataContents.Objective.team_objective_id`.

---

### `ECitadelGameMode`

**Confidence:** confirmed

| Value | Name |
|-------|------|
| 0 | `k_ECitadelGameMode_Invalid` |
| 1 | `k_ECitadelGameMode_Normal` |
| 2 | `k_ECitadelGameMode_1v1Test` |
| 3 | `k_ECitadelGameMode_Sandbox` |
| 4 | `k_ECitadelGameMode_StreetBrawl` |
| 5 | `k_ECitadelGameMode_ExploreNYC` |
| 6 | `k_ECitadelGameMode_Internal` |

**Product alignment:** All analytics -- only `Normal` (1) should feed competitive analytics. If `CMsgMatchMetaDataContents.MatchInfo.street_brawl_rounds` is populated, the match is a StreetBrawl (4) match -- exclude. Carried in `MatchInfo.game_mode`.

---

### `CMsgMatchMetaDataContents.EMatchOutcome`

**Confidence:** confirmed (nested enum)

| Value | Name |
|-------|------|
| 0 | `k_eOutcome_TeamWin` |
| 1 | `k_eOutcome_Error` |
| 2 | `k_eOutcome_MatchDraw` |

**Product alignment:** All analytics -- filter `Error` outcomes before computing any stats. `MatchDraw` is rare but must be treated as a non-win non-loss outcome in hero matchup calculations.

---

### `CMsgMatchMetaDataContents.EGoldSource`

**Confidence:** confirmed (nested enum). Canonical gold source enum used in both `CMsgMatchMetaDataContents.GoldSource` (post-match blob) and `CCitadelUserMessage_CurrencyChanged.currency_source` (live replay event).

| Value | Name |
|-------|------|
| 1 | `k_ePlayers` |
| 2 | `k_eLaneCreeps` |
| 3 | `k_eNeutrals` |
| 4 | `k_eBosses` |
| 5 | `k_eTreasure` |
| 6 | `k_eAssists` |
| 7 | `k_eDenies` |
| 8 | `k_eTeamBonus` |
| 9 | `k_eAbilityAssassinate` |
| 10 | `k_eItemTrophyCollector` |
| 11 | `k_eItemCultistSacrifice` |
| 12 | `k_eBreakable` |
| 13 | `k_eItemGooseEgg` |

**Product alignment:** Lane pressure (values 2, 7), kill gold (1, 6), objective gold (4), itemization (10-13).

---

### `CMsgMatchPlayerDamageMatrix.EStatType`

**Confidence:** confirmed (nested enum)

| Value | Name |
|-------|------|
| 0 | `k_eType_Damage` |
| 1 | `k_eType_Healing` |
| 2 | `k_eType_HealPrevented` |
| 3 | `k_eType_Mitigated` |
| 4 | `k_eType_LethalDamage` |
| 5 | `k_eType_Regen` |

**Product alignment:** Objective damage breakdown and hero matchup. `Mitigated` (3) and `HealPrevented` (2) enable survivability analysis. `LethalDamage` (4) distinguishes final killing damage from total damage dealt.

---

### `CMsgMatchPlayerPathsData.ECombatType`

**Confidence:** confirmed (nested enum)

| Value | Name |
|-------|------|
| 0 | `k_eCombatType_Out` |
| 1 | `k_eCombatType_Player` |
| 2 | `k_eCombatType_EnemyNPC` |
| 3 | `k_eCombatType_Neutral` |

**Product alignment:** Solo time tracking and fight classification. At each path sample, this field indicates whether the player was in combat with another player (1), a creep/NPC (2), a neutral camp (3), or out of combat (0). Used to compute time-in-combat vs. farming vs. roaming.

---

### `CMsgMatchPlayerPathsData.EMoveType`

**Confidence:** confirmed (nested enum)

| Value | Name |
|-------|------|
| 0 | `k_eMoveType_Normal` |
| 1 | `k_eMoveType_Ability` |
| 2 | `k_eMoveType_AbilityDebuff` |
| 3 | `k_eMoveType_GroundDash` |
| 4 | `k_eMoveType_Slide` |
| 5 | `k_eMoveType_RopeClimbing` |
| 6 | `k_eMoveType_Ziplining` |
| 7 | `k_eMoveType_InAir` |
| 8 | `k_eMoveType_AirDash` |

**Product alignment:** Mobility analysis -- ziplining (6) distinguishes rotations from lane presence. InAir (7) + AirDash (8) indicate vertical mobility usage.

---

### Other Enumerations (no analytics alignment)

| Enum | Values | Notes |
|------|--------|-------|
| `ECitadelAccountStatMedal` | 0=None, 1=Bronze, 2=Silver, 3=Gold | Career stat medal level -- GC API only |
| `ECitadelMMPreference` | 0=Invalid, 1=Casual, 2=Serious | MM preference -- lobby metadata only |
| `ECitadelBotDifficulty` | 0-5: None/Easy/Medium/Hard/Nightmare/Guided | Use to exclude CoopBot matches |
| `ECitadelRegionMode` / `ECitadelLeaderboardRegion` | Region codes | Not in replays |
| `ELobbyServerState` | Server state transitions | GC/lobby only |
| `EBannedFeature` / `EFeatureBanReason` | Account restriction types | GC API only |
| `CSOCitadelParty.*` nested enums | `EMemberRights`, `EPlayerType`, `EChatMode` | Party/lobby only |
| `EGCCitadelCommonMessages` | 7000=ReportAsserts, 7001=Response | Internal GC telemetry -- never in replays |

---

## Section 2: Post-Match Blob Messages (Accessible via Replay Decode)

Access path: subscribe to `CCitadelUserMsg_PostMatchDetails` (ID 316) in `on_packet` --> decode outer message --> decode `match_details: bytes` as `CMsgMatchMetaDataContents`.

---

### `CMsgMatchMetaData` (outer envelope)

**Data richness:** Low (envelope only)

| Field | Type | # | Confidence |
|-------|------|---|------------|
| `version` | `uint32` | 1 | confirmed -- schema version for the blob |
| `match_details` | `bytes` | 2 | confirmed -- inner blob; decode as `CMsgMatchMetaDataContents` |
| `match_id` | `uint64` | 3 | confirmed |

**Product alignment:** All post-match analytics -- this is the wrapper; all richness is in the inner `match_details` field.

---

### `CMsgMatchMetaDataContents.MatchInfo`

**Data richness:** Rich

| Field | Type | # | Confidence | Notes |
|-------|------|---|------------|-------|
| `duration_s` | `uint32` | 1 | confirmed | Total match duration in seconds |
| `match_outcome` | `EMatchOutcome` | 2 | confirmed | TeamWin / Error / MatchDraw |
| `winning_team` | `ECitadelLobbyTeam` | 3 | confirmed | |
| `players` | `repeated Players` | 4 | confirmed | Per-player stats array |
| `start_time` | `uint32` | 5 | confirmed | Unix timestamp of match start |
| `match_id` | `uint64` | 6 | confirmed | |
| `legacy_objectives_mask` | `uint32` | 8 | confirmed | 32-bit bitmask; deprecated in favor of 64-bit masks |
| `game_mode` | `ECitadelGameMode` | 9 | confirmed | |
| `match_mode` | `ECitadelMatchMode` | 10 | confirmed | |
| `objectives` | `repeated Objective` | 11 | confirmed | Per-objective destruction data |
| `match_paths` | `CMsgMatchPlayerPathsData` | 12 | confirmed | Sampled movement paths for all players |
| `damage_matrix` | `CMsgMatchPlayerDamageMatrix` | 13 | confirmed | Pairwise damage dealt/received matrix |
| `match_pauses` | `repeated Pause` | 14 | confirmed | Pause records |
| `custom_user_stats` | `repeated CustomUserStatInfo` | 15 | confirmed | Custom stat schema descriptors |
| `watched_death_replays` | `repeated WatchedDeathReplay` | 16 | confirmed | |
| `objectives_mask_team0` | `uint64` | 17 | confirmed | 64-bit objectives bitmask team 0 |
| `objectives_mask_team1` | `uint64` | 18 | confirmed | 64-bit objectives bitmask team 1 |
| `mid_boss` | `repeated MidBoss` | 19 | confirmed | Mid-boss kill records |
| `is_high_skill_range_parties` | `bool` | 20 | confirmed | |
| `low_pri_pool` | `bool` | 21 | confirmed | |
| `new_player_pool` | `bool` | 22 | confirmed | |
| `average_badge_team0` | `uint32` | 23 | inferred | Average rank badge for team 0 |
| `average_badge_team1` | `uint32` | 24 | inferred | Average rank badge for team 1 |
| `game_mode_version` | `uint32` | 25 | confirmed | |
| `rewards_eligible` | `bool` | 26 | confirmed | |
| `not_scored` | `bool` | 27 | confirmed | If true, match did not count for ranking |
| `team_score` | `repeated uint32` | 28 | inferred | Per-team score (StreetBrawl etc.) |
| `match_tracked_stats` | `repeated CMsgTrackedStat` | 29 | confirmed | Match-level tracked stats |
| `teams` | `repeated Teams` | 30 | confirmed | Team-level tracked stats |
| `bot_difficulty` | `ECitadelBotDifficulty` | 32 | confirmed | |
| `street_brawl_rounds` | `repeated StreetBrawlRound` | 33 | confirmed | StreetBrawl mode only |

**Product alignment:** Game phase detection (`duration_s`, `mid_boss`, `objectives`), objective damage breakdown (`objectives`), hero matchup (`players`, `damage_matrix`), draft/ban analysis (`hero_id` on players), fight classification (`damage_matrix`).

---

### `CMsgMatchMetaDataContents.Players`

**Data richness:** Rich

| Field | Type | # | Confidence | Notes |
|-------|------|---|------------|-------|
| `account_id` | `uint32` | 1 | confirmed | |
| `player_slot` | `uint32` | 2 | confirmed | |
| `death_details` | `repeated Deaths` | 3 | confirmed | Per-death breakdown |
| `items` | `repeated Items` | 4 | confirmed | Full item purchase/sell history |
| `stats` | `repeated PlayerStats` | 5 | confirmed | Time-series stats snapshots |
| `team` | `ECitadelLobbyTeam` | 6 | confirmed | |
| `party` | `uint32` | 16 | confirmed | **Patched version only** -- party group ID; absent from base message |
| `kills` | `uint32` | 8 | confirmed | Final kill count |
| `deaths` | `uint32` | 9 | confirmed | |
| `assists` | `uint32` | 10 | confirmed | |
| `net_worth` | `uint32` | 11 | confirmed | Final net worth in souls |
| `hero_id` | `uint32` | 12 | confirmed | |
| `last_hits` | `uint32` | 13 | confirmed | |
| `denies` | `uint32` | 14 | confirmed | |
| `ability_points` | `uint32` | 15 | confirmed | |
| `assigned_lane` | `uint32` | 17 | confirmed | Lane assignment using `CMsgLaneColor` encoding: Yellow=1, Blue=4, Purple=6 (Green=3 not observed in 3 replays but expected). NOT sequential 0-3 or 1-4 -- confirmed by probe 2026-04-10. |
| `level` | `uint32` | 18 | confirmed | Final hero level |
| `pings` | `repeated Ping` | 19 | confirmed | |
| `ability_stats` | `repeated AbilityStat` | 20 | confirmed | Per-ability stat values |
| `stats_type_stat` | `repeated float` | 21 | hypothesis | Opaque float array; likely a parallel stats-type array |
| `book_rewards` | `repeated BookReward` | 22 | confirmed | XP book rewards earned |
| `abandon_match_time_s` | `uint32` | 23 | confirmed | 0 if no abandon; > 0 = game time of abandon |
| `hero_data` | `CMsgPlayerHeroData` | 25 | confirmed | Cosmetic hero loadout (stripped in patched version) |
| `rewards_eligible` | `bool` | 26 | confirmed | |
| `accolades` | `repeated PlayerAccolade` | 27 | confirmed | End-of-match accolade badges (stripped in patched version) |
| `mvp_rank` | `uint32` | 28 | inferred | 1 = MVP (stripped in patched version) |
| `earned_holiday_award_2025` | `bool` | 29 | confirmed | Seasonal award flag (stripped in patched version) |
| `power_up_buffs` | `repeated PowerUpBuff` | 30 | confirmed | Power-up states at match end (stripped in patched version) |
| `player_tracked_stats` | `repeated CMsgTrackedStat` | 48 | confirmed | Tracked stats (stripped in patched version) |

**Note on `party` field:** The base `CMsgMatchMetaDataContents.Players` does NOT include `party`. It is only present in `CMsgMatchMetaDataContentsPatchedl.Players` (field 16). See Section 5 for patched message details.

**Product alignment:** Hero matchup by player (KDA, net_worth, hero_id), lane pressure (`assigned_lane`, `last_hits`, `denies` from PlayerStats), solo time inference (`assigned_lane`, `abandon_match_time_s`), draft/ban analysis (`hero_id`), party vs. solo segmentation (`party` -- patched only).

---

### `CMsgMatchMetaDataContents.PlayerStats` (time-series snapshot)

**Data richness:** Very rich -- ~50 fields per snapshot entry; multiple entries per player

Each entry is one snapshot at `time_stamp_s`. The snapshot interval is NOT fixed. Confirmed by probe (2026-04-10): nominal interval is 300s (5 minutes), with an initial 180s interval at match start and a final fractional interval to match end. Observed snapshot counts: 10 per player for ~2400s matches, 13 per player for ~3048s matches. Unique intervals observed across 3 replays: [48, 180, 245, 298, 300]. Do not assume fixed 300s granularity for time-windowed analysis.

| Field | Type | # | Confidence | Notes |
|-------|------|---|------------|-------|
| `time_stamp_s` | `uint32` | 1 | confirmed | Snapshot time in match seconds |
| `net_worth` | `uint32` | 2 | confirmed | Total souls at this timestamp |
| `gold_player` | `uint32` | 3 | confirmed | Souls from player kills |
| `gold_player_orbs` | `uint32` | 4 | confirmed | Souls from player kill orbs |
| `gold_lane_creep_orbs` | `uint32` | 5 | confirmed | Souls from lane creep orbs |
| `gold_neutral_creep_orbs` | `uint32` | 6 | confirmed | Souls from neutral creep orbs |
| `gold_boss` | `uint32` | 7 | confirmed | Souls from boss kills |
| `gold_boss_orb` | `uint32` | 8 | confirmed | Souls from boss orbs |
| `gold_treasure` | `uint32` | 9 | confirmed | Souls from treasure chests |
| `gold_denied` | `uint32` | 10 | confirmed | Souls from denies |
| `gold_death_loss` | `uint32` | 11 | confirmed | Souls lost on death |
| `gold_lane_creep` | `uint32` | 12 | confirmed | Souls from lane creep last-hits |
| `gold_neutral_creep` | `uint32` | 13 | confirmed | Souls from neutral camps |
| `kills` | `uint32` | 14 | confirmed | Cumulative kills at this timestamp |
| `deaths` | `uint32` | 15 | confirmed | |
| `assists` | `uint32` | 16 | confirmed | |
| `creep_kills` | `uint32` | 17 | confirmed | Lane creep last-hits |
| `neutral_kills` | `uint32` | 18 | confirmed | |
| `possible_creeps` | `uint32` | 19 | inferred | Total available creeps in lane -- enables CS% calculation; not available from live events |
| `creep_damage` | `uint32` | 20 | confirmed | Damage dealt to creeps |
| `player_damage` | `uint32` | 21 | confirmed | Damage dealt to players |
| `neutral_damage` | `uint32` | 22 | confirmed | |
| `boss_damage` | `uint32` | 23 | confirmed | Damage dealt to bosses/objectives |
| `denies` | `uint32` | 24 | confirmed | |
| `player_healing` | `uint32` | 25 | confirmed | Healing dealt to other players |
| `ability_points` | `uint32` | 26 | confirmed | |
| `self_healing` | `uint32` | 27 | confirmed | |
| `player_damage_taken` | `uint32` | 28 | confirmed | |
| `max_health` | `uint32` | 29 | confirmed | Hero max health at this snapshot |
| `weapon_power` | `uint32` | 30 | inferred | Bullet damage stat |
| `tech_power` | `uint32` | 31 | inferred | Ability damage stat |
| `shots_hit` | `uint32` | 32 | confirmed | |
| `shots_missed` | `uint32` | 33 | confirmed | |
| `damage_absorbed` | `uint32` | 34 | confirmed | |
| `absorpption_provided` | `uint32` | 35 | confirmed | Field name typo preserved verbatim from proto |
| `hero_bullets_hit` | `uint32` | 36 | confirmed | Bullets that hit enemy heroes specifically |
| `hero_bullets_hit_crit` | `uint32` | 37 | confirmed | Crit hits on heroes |
| `heal_prevented` | `uint32` | 38 | confirmed | |
| `heal_lost` | `uint32` | 39 | confirmed | |
| `gold_sources` | `repeated GoldSource` | 40 | confirmed | Per-source gold breakdown at this snapshot |
| `custom_user_stats` | `repeated CustomUserStat` | 41 | confirmed | |
| `damage_mitigated` | `uint32` | 42 | confirmed | |
| `level` | `uint32` | 43 | confirmed | |
| `player_barriering` | `uint32` | 44 | inferred | Barrier HP applied to other players |
| `teammate_healing` | `uint32` | 45 | confirmed | |
| `teammate_barriering` | `uint32` | 46 | confirmed | |
| `self_damage` | `uint32` | 47 | confirmed | Self-inflicted damage |
| `bullet_kills` | `uint32` | 48 | confirmed | |
| `melee_kills` | `uint32` | 49 | confirmed | |
| `ability_kills` | `uint32` | 50 | confirmed | |
| `headshot_kills` | `uint32` | 51 | confirmed | |

**Product alignment:** This is the richest single message for lane pressure and hero matchup. `possible_creeps` enables CS% calculations (unavailable from live events). Time-series enables gold lead curves across game phases. `boss_damage` provides aggregate objective damage. For fight classification, `player_damage` delta between snapshots can bound fight windows.

---

### `CMsgMatchMetaDataContents.Deaths`

**Data richness:** Medium

| Field | Type | # | Confidence | Notes |
|-------|------|---|------------|-------|
| `game_time_s` | `uint32` | 1 | confirmed | Match time of death |
| `time_to_kill_s` | `float` | 2 | inferred | Duration of the killing engagement |
| `killer_player_slot` | `uint32` | 9 | confirmed | |
| `death_pos` | `Position` | 10 | confirmed | World position of death |
| `killer_pos` | `Position` | 11 | confirmed | World position of killer at kill moment |
| `death_duration_s` | `uint32` | 12 | confirmed | Respawn time (death timer duration) |

**Product alignment:** Fight classification (`time_to_kill_s` distinguishes burst kills from sustained fights), solo time tracking (`death_duration_s` = time player is removed from map), map-zone attribution (position fields). `time_to_kill_s` is not reconstructable from individual live `Damage` (300) events.

---

### `CMsgMatchMetaDataContents.Items`

**Data richness:** Medium-high

| Field | Type | # | Confidence | Notes |
|-------|------|---|------------|-------|
| `game_time_s` | `uint32` | 1 | confirmed | Purchase time |
| `item_id` | `uint32` | 2 | confirmed | |
| `upgrade_id` | `uint32` | 3 | inferred | Upgrade variant ID if item was upgraded |
| `sold_time_s` | `uint32` | 4 | confirmed | 0 if not sold; > 0 = sale time |
| `flags` | `uint32` | 5 | hypothesis | Opaque flags; may encode active/passive state |
| `imbued_ability_id` | `uint32` | 6 | confirmed | Ability imbued into this item slot (flex slot) |

**Product alignment:** Hero matchup by player -- full item purchase + sell timeline. Richer than `CCitadelUserMessage_ItemPurchaseNotification` (ID 360) from live replay because it includes `sold_time_s` and `imbued_ability_id`. Enables exact build reconstruction at any match timestamp.

---

### `CMsgMatchMetaDataContents.Objective`

**Data richness:** Rich

| Field | Type | # | Confidence | Notes |
|-------|------|---|------------|-------|
| `legacy_objective_id` | `ECitadelObjective` | 1 | confirmed | Global objective enum value |
| `destroyed_time_s` | `uint32` | 2 | confirmed | Match time when objective was destroyed |
| `creep_damage` | `uint32` | 4 | confirmed | Damage to this objective by creeps |
| `creep_damage_mitigated` | `uint32` | 5 | confirmed | |
| `player_damage` | `uint32` | 6 | confirmed | Damage by players |
| `player_damage_mitigated` | `uint32` | 7 | confirmed | |
| `first_damage_time_s` | `uint32` | 8 | confirmed | When objective first took damage |
| `team_objective_id` | `ECitadelTeamObjective` | 9 | confirmed | Team-relative objective ID |
| `team` | `ECitadelLobbyTeam` | 10 | confirmed | Which team owns this objective |
| `player_spirit_damage` | `uint32` | 11 | inferred | Spirit/ability damage component of player damage |

**Product alignment:** Objective damage breakdown (all fields, spirit vs. bullet split unique to this message), game phase detection (`destroyed_time_s`, `first_damage_time_s`). `player_spirit_damage` is not available from `BossDamaged` (ID 348) live events.

---

### `CMsgMatchMetaDataContents.MidBoss`

**Data richness:** Low (but precise)

| Field | Type | # | Confidence | Notes |
|-------|------|---|------------|-------|
| `team_killed` | `ECitadelLobbyTeam` | 1 | confirmed | Which team killed the mid boss |
| `team_claimed` | `ECitadelLobbyTeam` | 2 | confirmed | Which team claimed the mid boss buff |
| `destroyed_time_s` | `uint32` | 3 | confirmed | Kill time in match seconds |

**Note:** `team_killed != team_claimed` is confirmed possible -- one team kills the boss and the other steals the buff. Observed in replay `55423930_379917638` at match second 2947: team0 killed, team1 claimed. Confirmed by probe 2026-04-10.

**Product alignment:** Game phase detection -- each entry in the `repeated mid_boss` array represents one kill cycle. Paired with `MidBossSpawned` (ID 349) from live replay data, gives full mid-boss lifecycle records.

---

### `CMsgMatchMetaDataContents.Pause`

**Data richness:** Low

| Field | Type | # | Confidence | Notes |
|-------|------|---|------------|-------|
| `game_time_s` | `uint32` | 1 | confirmed | |
| `pause_duration_s` | `uint32` | 2 | confirmed | |
| `player_slot` | `uint32` | 3 | confirmed | Who initiated the pause |

**Product alignment:** Solo time tracking and any time-based analysis -- subtract `pause_duration_s` from durations that cross a pause boundary. Pauses invalidate time-window calculations.

---

### `CMsgMatchMetaDataContents.GoldSource` (nested in `PlayerStats`)

**Data richness:** Low individually; medium in aggregate

| Field | Type | # | Confidence | Notes |
|-------|------|---|------------|-------|
| `source` | `EGoldSource` | 1 | confirmed | |
| `kills` | `uint32` | 2 | confirmed | Kill events for this source |
| `damage` | `uint32` | 3 | confirmed | Damage dealt for this source type |
| `gold` | `uint32` | 4 | confirmed | Gold earned from this source |
| `gold_orbs` | `uint32` | 5 | confirmed | Gold earned from orb drops from this source |

**Product alignment:** Lane pressure (creep gold vs. kill gold breakdown), deny tracking (`Denies` source kill count = deny count).

---

### `CMsgMatchPlayerPathsData`

**Data richness:** Rich

Embedded in `CMsgMatchMetaDataContents.MatchInfo.match_paths`. Contains sampled movement paths for all players.

#### Top-level fields

| Field | Type | # | Confidence | Notes |
|-------|------|---|------------|-------|
| `version` | `uint32` | 1 | confirmed | |
| `interval_s` | `float` | 2 | confirmed | Sampling interval in seconds |
| `x_resolution` | `uint32` | 3 | inferred | Coordinate quantization resolution |
| `y_resolution` | `uint32` | 4 | inferred | |
| `paths` | `repeated Path` | 5 | confirmed | One Path entry per player |

#### `Path` sub-message fields

| Field | Type | # | Confidence | Notes |
|-------|------|---|------------|-------|
| `player_slot` | `uint32` | 1 | confirmed | |
| `x_min` | `float` | 2 | confirmed | Bounding box for coordinate decoding |
| `y_min` | `float` | 3 | confirmed | |
| `x_max` | `float` | 4 | confirmed | |
| `y_max` | `float` | 5 | confirmed | |
| `x_pos` | `repeated uint32` | 6 | confirmed | Packed quantized X positions |
| `y_pos` | `repeated uint32` | 7 | confirmed | Packed quantized Y positions |
| `health` | `repeated uint32` | 9 | confirmed | Health at each path sample |
| `combat_type` | `repeated ECombatType` | 10 | confirmed | Combat state at each sample |
| `move_type` | `repeated EMoveType` | 11 | confirmed | Movement type at each sample |

**Coordinate decode (hypothesis -- unvalidatable without path data):** `x_world = x_min + (x_pos[i] / x_resolution) * (x_max - x_min)`

**IMPORTANT -- probe finding (2026-04-10):** `CMsgMatchPlayerPathsData` was **absent** from all 3 verification replays (`paths_player_count=0`, `interval_s=None`, `x_resolution=None`). The `match_paths` field decoded as `None` in every replay tested. This sub-message may be version-gated, stripped in the patched variant, or only present in replays from certain game versions. The coordinate decode formula and all per-path fields remain unvalidated against real data. Do not build features on this sub-message until presence is confirmed in a newer or different replay.

**Product alignment:** Solo time tracking -- the most direct data source for map presence analysis (if present). `combat_type` per sample combined with position enables: time in each map zone, time in combat vs. farming vs. roaming, proximity to teammates for solo detection without entity polling. May be more reliable than entity-level polling because it is pre-aggregated by the server.

---

### `CMsgMatchPlayerDamageMatrix`

**Data richness:** Very rich

Embedded in `CMsgMatchMetaDataContents.MatchInfo.damage_matrix`. Full pairwise damage between all players, broken down by ability/source and stat type, with time sampling.

#### Top-level fields

| Field | Type | # | Confidence | Notes |
|-------|------|---|------------|-------|
| `damage_dealers` | `repeated DamageDealer` | 1 | confirmed | One entry per player who dealt damage |
| `sample_time_s` | `repeated uint32` | 2 | confirmed | Time axis for damage samples |
| `source_details` | `SourceDetails` | 3 | confirmed | Stat type and source name labels |

#### `DamageDealer` sub-message

| Field | Type | # | Confidence |
|-------|------|---|------------|
| `dealer_player_slot` | `uint32` | 1 | confirmed |
| `damage_sources` | `repeated DamageSource` | 2 | confirmed |

#### `DamageSource` sub-message

| Field | Type | # | Confidence | Notes |
|-------|------|---|------------|-------|
| `damage_to_players` | `repeated DamageToPlayer` | 2 | confirmed | |
| `source_details_index` | `uint32` | 4 | confirmed | Index into `source_details.source_name` / `stat_type` |

#### `DamageToPlayer` sub-message

| Field | Type | # | Confidence | Notes |
|-------|------|---|------------|-------|
| `target_player_slot` | `uint32` | 1 | confirmed | |
| `damage` | `repeated uint32` | 2 | confirmed | Packed damage values parallel to `sample_time_s` |

#### `SourceDetails` sub-message

| Field | Type | # | Confidence | Notes |
|-------|------|---|------------|-------|
| `stat_type` | `repeated EStatType` | 1 | confirmed | Parallel array with `source_name` |
| `source_name` | `repeated string` | 2 | confirmed | Human-readable source names (e.g. ability names) |

**Confirmed probe data (2026-04-10):** `sample_time_s` cadence is 180s (3-minute intervals) with a final fractional sample to match end. Across 3 replays: 13 dealers per match (12 players + 1 extra, likely NPC/env source), 13-17 sample buckets. Source names confirmed as human-readable ability/weapon strings (e.g. `"citadel_weapon_inferno_set"`, `"base_stat_regen"`, `"ability_afterburn"`, `"UnknownAbility"`).

**Product alignment:** Objective damage breakdown (per-ability damage with time windows), hero matchup by player (damage dealt/received between specific hero pairs), fight classification (identify fight participants by damage exchange bursts). The `sample_time_s` array enables time-windowed analysis for isolating individual fights.

---

## Section 3: GC-Only Messages (Not Accessible From Replays)

Documented here for completeness and for future GC API integration.

### `CMsgHeroSelectionMatchInfo`

| Field | Type | # | Confidence | Notes |
|-------|------|---|------------|-------|
| `hero_selections` | `repeated Hero` | 1 | confirmed | Heroes selected with priority ordering |
| `hero_selections[].hero_id` | `uint32` | 1 | confirmed | |
| `hero_selections[].priority` | `uint32` | 2 | inferred | Selection priority order |
| `banned_heroes` | `repeated uint32` | 2 | confirmed | Hero IDs banned in this selection |

**Product alignment:** Draft / ban analysis (GC API path). Richer than `CCitadelUserMsg_BannedHeroes` (ID 366) from replay because it includes hero selection priority. For replay-based draft analysis, use ID 366 instead.

---

### `CMsgHeroBuild` (GC API)

Key fields:

| Field | Type | # | Confidence | Notes |
|-------|------|---|------------|-------|
| `hero_build_id` | `uint32` | 1 | confirmed | |
| `hero_id` | `uint32` | 2 | confirmed | |
| `author_account_id` | `uint32` | 3 | confirmed | |
| `last_updated_timestamp` | `uint32` | 4 | confirmed | |
| `name` | `string` | 5 | confirmed | |
| `details` | `Details_V0` | 10 | confirmed | Contains `mod_categories` (item lists) and `ability_order` |
| `details.mod_categories[].mods[].ability_id` | `uint32` | 1 | confirmed | Item/ability ID in this build slot |
| `details.ability_order.currency_changes[].ability_id` | `uint32` | 1 | confirmed | Skill leveling sequence |
| `details.ability_order.currency_changes[].delta` | `int32` | 3 | confirmed | Souls cost/refund for this action |

**Product alignment:** Hero matchup by player (GC API path). High-rated shared builds provide archetype context for evaluating player build deviations. `details.ability_order` encodes the recommended item purchase sequence with soul costs.

---

### `CMsgAccountHeroStats` (GC API)

| Field | Type | # | Confidence | Notes |
|-------|------|---|------------|-------|
| `hero_id` | `uint32` | 1 | confirmed | |
| `stat_id` | `repeated uint32` | 2 | confirmed | Stat type IDs (schema not in this proto) |
| `total_value` | `repeated uint64` | 3 | confirmed | Parallel array with `stat_id` |
| `medals_bronze/silver/gold` | `repeated uint32` | 4-6 | confirmed | Medal counts by stat |

**Product alignment:** Hero matchup by player (GC API path). Career stats per hero as baseline for comparing in-match performance. `stat_id` values are not defined in this proto -- cross-reference with Deadlock game resource files (hypothesis).

---

### `CMsgTrackedStat`

| Field | Type | # | Confidence | Notes |
|-------|------|---|------------|-------|
| `tracked_stat_id` | `uint32` | 1 | confirmed | Opaque ID; schema not in this proto |
| `tracked_stat_value` | `int32` | 2 | confirmed | Signed to allow negative deltas |

**Product alignment:** Hero matchup -- appears as `repeated CMsgTrackedStat player_tracked_stats` in `CMsgMatchMetaDataContents.Players` (post-match blob). Also in `Teams` and `MatchInfo.match_tracked_stats`. `tracked_stat_id` values require cross-reference with Deadlock game resource definitions.

---

### Other GC-Only Messages (no analytics alignment)

| Message | Category | Notes |
|---------|----------|-------|
| `CSOCitadelLobby` | Match lobby | Contains `match_id`, `match_mode` for GC-side filtering |
| `CSOCitadelParty` | Party object | Has `party` composition, `hero_roster`, `low_priority_games_remaining` |
| `CSOCitadelHideoutLobby` | Social hub | Cosmetic/social system |
| `CMsgStartFindingMatchInfo` | Client-to-GC | Matchmaking request |
| `CMsgRegionPingTimesClient` | Client-to-GC | Network latency report |
| `CMsgEquippedItemList` / `CMsgPlayerHeroData` | Steam inventory | Cosmetic loadout |
| `CMsgGCAccountData` | Account | `cheater_report_score: float` -- potential data quality filter |
| `CMsgHeroBuildPreference` / `CMsgHeroReleaseVoteTally` | Build system | Community voting |
| `CMsgAnyToGCReportAsserts` | Internal telemetry | Never in replays |
| `CLobbyData_PostMatchSurvey` | Post-match | No analytics value |

---

## Section 4: Product Alignment Summary

| Focus Area | Post-Match Blob (`CMsgMatchMetaDataContents`) | Live Replay (`on_packet`) | Live Replay (`on_entity`) |
|------------|-----------------------------------------------|---------------------------|---------------------------|
| **Game phase detection** | `MidBoss.destroyed_time_s`, `Objective.destroyed_time_s`, `Objective.first_damage_time_s` | `MidBossSpawned` (349), `BossKilled` (347), `GameOver` (346) | Objective entity health |
| **Lane pressure** | `PlayerStats` (`gold_lane_creep`, `denies`, `possible_creeps` time-series), `CMsgMatchPlayerPathsData` | `CurrencyChanged` (345), `GoldHistory` (313) | Creep entity positions |
| **Fight classification** | `Deaths` (`time_to_kill_s`, positions), `CMsgMatchPlayerDamageMatrix` (pairwise damage), `CMsgMatchPlayerPathsData.ECombatType` | `HeroKilled` (319), `Damage` (300), `KillStreak` (351) | Player positions |
| **Hero matchup / player performance** | `Players` (KDA, items, stats), `PlayerStats` (50+ fields), `CMsgMatchPlayerDamageMatrix` | `GetDamageStatsResponse` (339), `ItemPurchaseNotification` (360) | Hero entity state |
| **Solo time tracking** | `CMsgMatchPlayerPathsData` (proximity + `combat_type` over time), `Deaths.death_duration_s`, `Pause.pause_duration_s` | `PlayerRespawned` (353), `HeroKilled` (319) | Player positions |
| **Objective damage breakdown** | `Objective` (per-obj damage, spirit/bullet split), `PlayerStats.boss_damage`, `CMsgMatchPlayerDamageMatrix` | `BossDamaged` (348), `Damage` (300) | Objective entity health |
| **Draft / ban analysis** | `Players.hero_id` (confirmed picks) | `BannedHeroes` (366) | -- |

### Post-Match Blob vs. Live Replay Tradeoffs

The post-match blob provides substantially richer data than live replay events for most features:

- **Lane pressure:** `PlayerStats.possible_creeps` enables CS% -- not available from live `CurrencyChanged` events
- **Fight classification:** `Deaths.time_to_kill_s` and `CMsgMatchPlayerDamageMatrix` are not reconstructable from individual live `Damage` events
- **Solo time:** `CMsgMatchPlayerPathsData` with `combat_type` per position sample may be more reliable than entity polling
- **Objective damage breakdown:** `Objective.player_spirit_damage` (spirit vs. bullet split) is not available from `BossDamaged` (ID 348)

Tradeoff: blob data is only available at match end and has lower time resolution than tick-by-tick events. For features requiring real-time updates or sub-minute timing (e.g., exact fight start tick), live replay events are necessary.

---

## Section 5: Patched Message Details

### `CMsgMatchMetaDataContentsPatched`

**Rust struct name confirmed:** `CMsgMatchMetaDataContentsPatched` (no trailing `l`). The `patch.proto` source file uses `CMsgMatchMetaDataContentsPatchedl` (trailing lowercase `l`) but prost generates `CMsgMatchMetaDataContentsPatched`. Confirmed in `/tmp/parser-target/debug/build/valveprotos-*/out/deadlock.rs` in the container (2026-04-10).

**Decode path confirmed:** The bytes from `CCitadelUserMsg_PostMatchDetails.match_details` decode directly as `CMsgMatchMetaDataContentsPatched` -- no intermediate `CMsgMatchMetaData` envelope. See Section 2 critical note.

Defined in `patch.proto` in the same repo. A stripped-down version of `CMsgMatchMetaDataContents` with sensitive fields removed from the `Players` sub-message.

**Name discrepancy alert:** Three names exist across sources:
- `patch.proto` -- `CMsgMatchMetaDataContentsPatchedl` (trailing lowercase `l`)
- `citadel_gcmessages_common.proto` (inline copy) -- `CMsgMatchMetaDataContentsPatchedMsgMatchMetaDataContentsPatchedMsgHeroReleaseVoteT` (appears malformed/concatenated)
- haste migration docs (`deadlock-api-haste-reference.md`) -- `CMsgMatchMetaDataContentsPatched` (no trailing `l`)

**Resolved (2026-04-10):** The correct Rust struct name is `CMsgMatchMetaDataContentsPatched` (no trailing `l`). The haste migration docs form was correct. The `patch.proto` trailing `l` is a source artifact that prost strips during name normalization.

**Fields stripped from `Players` in patched version:**
- `hero_data` (field 25)
- `player_tracked_stats` (field 48)
- `accolades` (field 27)
- `mvp_rank` (field 28)
- `earned_holiday_award_2025` (field 29)
- `power_up_buffs` (field 30)

**Fields added in patched version:**
- `party: uint32` (field 16) -- confirmed -- party group ID; enables party vs. solo queue segmentation. **This field is absent from the base message and is only available via the patched variant.**

---

## Section 6: Open Questions

1. **`assigned_lane` encoding** -- **RESOLVED (2026-04-10).** Values confirmed as `CMsgLaneColor` encoding: `{1, 4, 6}` (Yellow=1, Blue=4, Purple=6) across all 3 verification replays. NOT sequential 0-indexed or 1-indexed. The fourth lane (Green=3) was not observed but is expected. Map: 1=Yellow, 3=Green, 4=Blue, 6=Purple.

2. **`tracked_stat_id` schema** -- still open. `CMsgTrackedStat.tracked_stat_id` values are opaque integers with no enum in this file. Cross-reference with Deadlock game resource files (not available in this proto) to map IDs to stat names.

3. **`CMsgMatchPlayerPathsData` coordinate decode** -- **BLOCKED (2026-04-10).** `CMsgMatchPlayerPathsData` is absent from all 3 verification replays (`match_paths` decoded as `None`). Cannot validate the hypothesis formula `x_world = x_min + (x_pos[i] / x_resolution) * (x_max - x_min)` without path data present. Re-run against newer replays or a different game version to confirm field presence.

4. **`PlayerStats` snapshot interval** -- **RESOLVED (2026-04-10).** Not a fixed interval. Nominal cadence: 300s (5-minute snapshots) with an initial 180s bucket and a final fractional bucket to match end. Snapshot count: 10 per player for ~2400s matches, 13 per player for ~3048s matches. Unique interval values observed across 3 replays: [48, 180, 245, 298, 300]. The mode is 300s but the first snapshot is always at 180s and the last aligns to match end.

5. **`mid_boss` steal mechanic** -- **RESOLVED (2026-04-10).** `team_killed != team_claimed` is confirmed possible. Observed in replay `55423930_379917638` at match second 2947: `team_killed=0, team_claimed=1`. 7 total mid_boss kills across 3 replays, 1 steal observed.

6. **Patched message Rust struct name** -- **RESOLVED (2026-04-10).** The correct Rust struct name is `CMsgMatchMetaDataContentsPatched` (no trailing `l`). Confirmed by inspecting prost-generated code in the container build output (`/tmp/parser-target/debug/build/valveprotos-*/out/deadlock.rs`). The `patch.proto` source uses `Patchedl` but prost normalizes it.
