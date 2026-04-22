# Entity Types Reference
**Last Updated:** 2026-04-07 (validation pass against real demo via `probe_all_entity_classes`)
**Purpose:** Catalog of Deadlock entity classes accessible via haste `on_entity` subscriptions. Entity-type focused -- not field-level. For field semantics and gotchas, see `private/specs/entity-fields-reference.md`.

**Scope discipline:** This file covers entity *class names* (subscription target, inheritance chain, lifecycle, game role, parser status). Field-level gotchas (e.g. `m_lifeState` semantics, ghost-creep bug, cell-coordinate decoding) live in `entity-fields-reference.md` and are linked rather than repeated here.

---

## Source Verification Key

| Label | URL |
|-------|-----|
| `[haste-position]` | https://raw.githubusercontent.com/blukai/haste/main/examples/deadlock-position.rs |
| `[haste-gametime]` | https://raw.githubusercontent.com/blukai/haste/main/examples/deadlock-gametime.rs |
| `[haste-lifestate]` | https://raw.githubusercontent.com/blukai/haste/main/examples/lifestate.rs |
| `[parser-constants]` | `parser/src/entities/constants.rs` |
| `[parser-replay]` | `parser/src/replay_parser.rs` |
| `[parser-boss]` | `parser/src/tracking/boss_tracker.rs` |
| `[parser-creep]` | `parser/src/tracking/creep_tracker.rs` |
| `[probe-classes]` | `private/engineering/tools/probe_all_entity_classes.rs` -- decodes CDemoSendTables -> CsvcMsgFlattenedSerializer via prost to enumerate every serializer class name and its fields in a given `.dem`. Authoritative source for "does class X exist?" and "does field Y live on class X?" Proprietary reference tooling kept in the private submodule; NOT part of the parser crate's normal build. Copy into `parser/src/bin/` temporarily to run, then delete the copy. Run instructions are in the file header. |
| `[runtime-census]` | `private/engineering/tools/probe_entity_counts.rs` -- runtime on_entity tally: CREATE / UPDATE / DELETE counts and unique entity-index counts per class, resolved to class names via a SendTables hash→name map. Authoritative source for "does class X actually instantiate?" and "how many instances spawn?" in a given replay. Full census recorded in `private/specs/entity-types-runtime-census.md`. |
| `[proto-gcmessages-common]` | https://raw.githubusercontent.com/deadlock-api/valveprotos-rs/master/protos/deadlock/citadel_gcmessages_common.proto |

**Note on schema dumps:** Earlier drafts of this doc cited `SteamDatabase/GameTracking-Deadlock` as a schema source. That repo tracks `.vpk` asset changes and does NOT contain server schema dumps; citations to it have been removed. For entity-class and field-level ground truth, run `probe_all_entity_classes` against a real demo (see `parser-mental-model.md` -- "Entity Field Lookup Tools"). The `deadlock-api/haste` fork strips `preserve-metadata`, so class names are not recoverable through the runtime entity API -- decoding SendTables directly is the only path.

**Validation pass (2026-04-07):** All `(unverified)` markers in this doc were resolved by running `probe_all_entity_classes` against `parser/src/replays/68175583_527726523.dem` with filter `CNPC_` (and `CCitadel_` for building/powerup classes). Field-presence claims are now either `[probe-classes]`-confirmed or explicitly scoped to what the probe could not see (e.g. runtime values, lifecycle timing).

---

## Inheritance Chains

All Deadlock combat NPCs derive from a common Source 2 / Citadel hierarchy. Understanding this chain determines which fields are available on any given class.

```
CBaseEntity
  └── CBaseCombatCharacter
        └── CAI_BaseNPC               -- m_NPCState, navigation, scheduling
              └── CAI_CitadelNPC      -- adds m_iBaseGoldReward, m_CCitadelAbilityComponent, m_CCitadelRegenComponent
                    ├── CNPC_Trooper           -- + m_iLane, m_iLaneSlot [probe-classes confirmed]
                    ├── CNPC_TrooperBoss       -- Guardian; m_iLane present [probe-classes confirmed]
                    ├── CNPC_TrooperNeutral    -- neutral camp NPC; m_iLane ABSENT [probe-classes confirmed]
                    ├── CNPC_TrooperNeutralNodeMover  -- neutral support/mover (discovered via probe; role uncharacterized)
                    ├── CNPC_Boss_Tier2        -- Walker; m_iLane present [probe-classes confirmed]
                    ├── CNPC_Boss_Tier3        -- Patron / main boss; m_iLane present [probe-classes confirmed]
                    ├── CNPC_BarrackBoss       -- Base Guardian; m_iLane present [probe-classes confirmed]
                    ├── CNPC_MidBoss           -- mid boss; m_iLane ABSENT (neutral) [probe-classes confirmed]
                    ├── CNPC_Neutral_SinnersSacrifice         -- punchable neutral entity
                    ├── CNPC_Neutral_SinnersSacrifice_Hideout -- companion static of SinnersSacrifice (discovered via probe)
                    ├── CNPC_BaseDefenseSentry  -- base defense unit; m_iLane ABSENT [probe-classes confirmed]
                    └── CNPC_ShieldedSentry     -- shielded base defense unit; m_iLane ABSENT [probe-classes confirmed]
```

Field inheritance from base classes (confirmed via field presence on concrete subclasses in the probe run; class-hierarchy edges themselves are inferred from which fields the probe saw on each class):
- `CBaseEntity`: `m_iHealth`, `m_lifeState`, `m_iTeamNum`, `m_flCreateTime`, `m_spawnflags`, position via `CBodyComponent`.
- `CAI_BaseNPC`: `m_NPCState`, navigation.
- `CAI_CitadelNPC`: `m_iBaseGoldReward`, `m_CCitadelAbilityComponent`, `m_CCitadelRegenComponent`.

Non-NPC entity classes (no AI hierarchy):
```
CBaseEntity
  └── CCitadelPlayerController    -- player session metadata, hero assignment
  └── CCitadelPlayerPawn          -- hero combat entity, position, health
  └── CCitadelGameRulesProxy      -- game clock, match state
  └── CItemXP                     -- soul orb (dropped on unit death)
  └── CCitadel_Destroyable_Building           -- Shrine-type destroyable structure; m_iLane ABSENT [probe-classes confirmed]
  └── CCitadel_PunchablePowerup               -- Bridge Buff / Rune (Heavy-Melee-to-claim powerup) [probe-classes]
  └── CCitadelItemPickupRejuv                 -- Rejuvenator pickup [probe-classes]
  └── CCitadelItemPickupIdol                  -- Golden Idol pickup [probe-classes]
  └── CCitadelItemPunchableNeutralGold        -- Punchable gold stash (neutral gold pile) [probe-classes]
  └── CCitadelItemPickupRejuvHeroTest         -- sandbox / hero-test Rejuv variant [probe-classes]
  └── CCitadelItemPickup                      -- base class for pickup items [probe-classes]
  └── CCitadel_BreakableProp                  -- parent: destructible world prop [probe-classes, runtime-confirmed]
        ├── CCitadel_BreakablePropPickup             -- parent: breakable that becomes a pickup [probe-classes]
        │     ├── CCitadel_BreakablePropGoldPickup       -- gold drop [probe-classes, runtime]
        │     ├── CCitadel_BreakablePropHealthPickup     -- health drop [probe-classes, runtime]
        │     └── CCitadel_BreakablePropModifierPickup   -- modifier/buff drop [probe-classes, runtime]
        ├── CCitadel_BreakableDroppedGoldPickup      -- transient gold orb (high churn) [probe-classes, runtime]
        └── CCitadel_BreakableDroppedNecroPickup     -- Graves-specific drop [probe-classes, not observed in single-match probe]
  └── CCitadel_PickupItemSpawner              -- controller / spawner for pickup items [probe-classes, runtime]
  └── CCitadel_HeroTestOrbSpawner             -- sandbox / hero-test orb spawner [probe-classes, runtime -- role [I] inferred]
  └── CCitadel_HideOutTargetSpawner           -- post-match hideout target spawner [probe-classes, not observed in match phase]
  └── CCitadel_GraveStone_Blocker             -- Graves ability blocker
  └── CNecro_HauntingSkullEntity              -- Graves summon
  └── CNPC_NecroSkele                         -- Graves summon skeleton
  └── CProjectile_Priest_SlideTrap_Projectile -- Venator slide trap projectile
```

---

## Entity Catalog

### CCitadelPlayerController
**Hash constant:** `CCITADELPLAYERCONTROLLER_ENTITY` `[parser-constants]`
**Inheritance:** CBaseEntity (not a combat character)
**Game role:** Persistent session entity for a player slot. Exists for the full match duration. Holds player identity, hero assignment, and lane assignment. One per player (12 total in a full match).
**Lifecycle:** Created at match load; never destroyed mid-match.
**Key identifying fields:**
- `m_iszPlayerName` -- player display name (Box<[u8]> in haste; use from_utf8_lossy)
- `m_steamID` -- Steam ID 64
- `m_hPawn` -- ehandle to the CCitadelPlayerPawn
- `m_iTeamNum` -- team (2 = Amber/team0, 3 = Sapphire/team1)
- `m_nOriginalLaneAssignment` / `m_nAssignedLane` -- lane (1-4; 0 before lock)
- `m_bLaneSwapLocked` -- true once lane assignment is final
- `m_PlayerDataGlobal.m_nHeroID` -- hero numeric ID
- `m_unLobbyPlayerSlot` -- stable per-player ID for array indexing
**Parser status:** Subscribed -- CREATE handled in `on_entity` (via `get_custom_id`); UPDATE polled for lane lock via `check_and_update_lane_lock`. `[parser-replay]`
**Product alignment:** All features -- player identity, hero, lane, and team are prerequisites for every per-player metric. F1-F14.

---

### CCitadelPlayerPawn
**Hash constant:** `CCITADELPLAYERPAWN_ENTITY` `[parser-constants]`
**Inheritance:** CBaseEntity → CBaseCombatCharacter (not an NPC, no `m_NPCState`)
**Game role:** The hero combat entity. Carries position, health, life state, and ability state. One per player per match; does NOT respawn as a new entity -- the same entity cycles through death/respawn via `m_lifeState` transitions.
**Lifecycle:** Created at match load; persists for entire match (including when hero is dead, for corpse rendering and respawn). Never DELETEd mid-match. `[haste-lifestate]`
**Key identifying fields:**
- `m_lifeState` -- 0=ALIVE, 2=DEAD. The correct death signal for heroes (use `0 → 2` transition). See `entity-fields-reference.md` for gotchas.
- Position via `CBodyComponent` cell-coordinate fields. See `entity-fields-reference.md` for correct fkey_from_path values.
- `m_hOwnerEntity` -- ehandle back to `CCitadelPlayerController`
**Parser status:** Subscribed -- positions polled every second in `on_tick_end` via `should_track_position`. Player identity extracted via `get_custom_id` cross-referencing `CCitadelPlayerController`. `[parser-replay]`
**Product alignment:** F2 (death tracking via lifeState transition), F6 (fight classification -- player proximity), F7 (solo time), F8 (archetype metrics per player).
**Notes:** Hero death detection requires `m_lifeState → 2` transition in `on_entity`, not DELETE events. Positions are tracked but `m_lifeState` transitions are not yet recorded to a timeline -- gap for F2.

---

### CCitadelGameRulesProxy
**Hash constant:** `DEADLOCK_GAMERULES_ENTITY` `[parser-constants]`
**Inheritance:** CBaseEntity
**Game role:** Singleton entity holding match-global game state. Contains the game clock (`m_pGameRules.m_flGameStartTime`) and other rules state.
**Lifecycle:** Created at match load; persists full match.
**Key identifying fields:**
- `m_pGameRules.m_flGameStartTime` -- replay-time seconds when match officially started. Used to compute `match_start_time_s`. `[haste-gametime]`
**Parser status:** Subscribed -- `handle_game_rules` extracts `m_flGameStartTime` on every UPDATE. `[parser-replay]`
**Product alignment:** Foundation for all timeline features -- without match start time, no match-relative timestamps are possible. Prerequisite for F1-F14.

---

### CNPC_Trooper (Lane Creeps)
**Hash constant:** `CNPC_TROOPER_ENTITY` `[parser-constants]`
**Inheritance:** CBaseEntity → CBaseCombatCharacter → CAI_BaseNPC → CAI_CitadelNPC → CNPC_Trooper `[schema-CNPC_Trooper]`
**Game role:** Standard lane creep. 4 per wave per lane per team (8 waves active at once across 4 lanes × 2 teams). Marches from base toward enemy objectives. The primary soul-farm target.
**Lifecycle:** Entity slots are allocated at match start and NEVER deleted during match play -- zero DELETE events observed in a full match. Slots recycle in-place: `m_lifeState = DEAD → ALIVE` signals a new wave reusing the same entity index. See `entity-fields-reference.md` for full recycling documentation.
**Key identifying fields:**
- `m_iLane` -- lane assignment (1-4). Pre-spawn value is 0; gate on `lane != 0` before tracking.
- `m_iLaneSlot` -- slot within the wave (0-3)
- `m_NPCState` -- active state; Deadlock-specific values 10 (DYING_CITADEL) and 12 (DEAD_CITADEL) are not in standard Source 2 SDK. See `entity-fields-reference.md` for full state table.
- `m_lifeState` -- 0=ALIVE, 2=DEAD
- `m_iHealth` -- current health; cage entity health sentinel = 1
- `m_iTeamNum` -- team (2 or 3)
**Parser status:** Fully subscribed -- CREATE/UPDATE handled by `CreepTracker`; positions polled in `on_tick_end`. Includes ghost-creep suppression via health + lifeState whitelist. `[parser-creep]` `[parser-replay]`
**Product alignment:** F1 (souls from LaneCreeps source -- correlate CurrencyChanged events with creep positions), F5 (farm efficiency -- lane farm vs. other sources), F9 (lane state at walker contact time).
**Quirks:** Cage entities (health=1, m_MoveType=0) exist as zipline sprites for each creep. They share the same entity class and are filtered by `health > 1`. See `entity-fields-reference.md`.

---

### CNPC_TrooperBoss (Guardian)
**Hash constant:** `CNPC_TROOPERBOSS_ENTITY` `[parser-constants]`
**Inheritance:** CBaseEntity → ... → CAI_CitadelNPC. Concrete subclass of the trooper-boss lineage; `m_NPCState`, `m_iHealth`, `m_iMaxHealth`, `m_iLane`, `m_iTeamNum` all confirmed present on the class. `[probe-classes]`
**Game role:** Guardian -- the first lane objective in each lane. 4 per team (one per lane). Destroying it opens the path to the Walker. In-game called "Guardian."
**Lifecycle:** Spawns at match start in each lane. Destroyed by enemy team; DELETE fires on death. One entity per Guardian position (does not respawn in normal matches).
**Key identifying fields:**
- `m_iLane` -- which lane (1-4)
- `m_iTeamNum` -- owning team
- `m_iHealth` / `m_iMaxHealth` -- health state; tracked damage-driven via `BossTracker`
- `m_lifeState` -- death signal
**Parser status:** Subscribed -- CREATE/DELETE handled by `BossTracker`; health sampled on each damage event. Positions polled in `on_tick_end`. `[parser-boss]` `[parser-replay]`
**Product alignment:** F9 (walker first contact requires knowing guardian is already dead; guardian death timestamp provides gate), F6 (guardian fights are engagement points).
**ECitadelObjective mapping:** Tier1 objectives (lanes 1-4) in `ECitadelObjective` enum; maps to `k_eCitadelObjective_Team0_Tier1_Lane[1-4]` and `k_eCitadelObjective_Team1_Tier1_Lane[1-4]`. `[proto-gcmessages-common]`

---

### CNPC_Boss_Tier2 (Walker)
**Hash constant:** `CNPC_BOSS_TIER2_ENTITY` `[parser-constants]`
**Inheritance:** CBaseEntity → ... → CAI_CitadelNPC. `m_iLane`, `m_iHealth`, `m_iMaxHealth`, `m_iTeamNum`, `m_NPCState` confirmed on the class. `[probe-classes]`
**Game role:** Walker -- the second lane objective in each lane. 4 per team. Destroying it grants team-wide buffs and opens the path to the Base Guardian. In-game called "Walker."
**Lifecycle:** Spawns at match start. Destroyed by enemy team; DELETE fires on death.
**Key identifying fields:** Same as Guardian: `m_iLane`, `m_iTeamNum`, `m_iHealth`, `m_iMaxHealth`, `m_lifeState`.
**Parser status:** Subscribed -- CREATE/DELETE handled by `BossTracker`; health tracked damage-driven; positions polled. `[parser-boss]`
**Product alignment:** F9 (walker first contact -- `BossDamaged` ID 348 detects first hit; entity health timeline shows exact progress). The single highest-value entity for F9; requires cross-referencing damage events with entity health timeline.
**ECitadelObjective mapping:** Tier2 objectives in `ECitadelObjective` enum. `[proto-gcmessages-common]`

---

### CNPC_BarrackBoss (Base Guardian)
**Hash constant:** `CNPC_BARRACKBOSS_ENTITY` `[parser-constants]`
**Inheritance:** CBaseEntity → ... → CAI_CitadelNPC. `m_iLane`, `m_iHealth`, `m_iTeamNum`, `m_NPCState` confirmed on the class. `[probe-classes]`
**Game role:** Base Guardian -- third objective after Walker in each lane. 4 per team. Destroying it damages the patron shields and may grant additional access. In-game UI calls this "Base Guardian."
**Lifecycle:** Spawns at match start. Destroyed by enemy; DELETE fires.
**Key identifying fields:** `m_iLane`, `m_iTeamNum`, `m_iHealth`, `m_lifeState`.
**Parser status:** Subscribed -- CREATE/DELETE/damage in `BossTracker`; positions polled. `[parser-boss]`
**Product alignment:** F9 (late-game objective sequencing), F6 (base guardian fights are large teamfights).
**ECitadelObjective mapping:** `k_eCitadelObjective_Team[0|1]_BarrackBoss_Lane[1-4]`. `[proto-gcmessages-common]`

---

### CCitadel_Destroyable_Building (Shrine)
**Hash constant:** `CCITADEL_DESTROYABLE_BUILDING_ENTITY` `[parser-constants]`
**Inheritance:** CBaseEntity (static destructible building; NOT in the NPC hierarchy -- `m_NPCState` is absent from the class). `[probe-classes]`
**Game role:** Destroyable shrine structures scattered along each lane route. Must be destroyed before the Base Guardian becomes attackable. Exact count and placement per lane is not derivable from entity data alone (requires map/geometry inspection).
**Lifecycle:** Spawns at match start; DELETE fires on destruction.
**Key identifying fields:** `m_iTeamNum`, `m_iHealth`, `m_lifeState`, plus building-specific `m_bDestroyed`, `m_bFinal`, `m_vecWeakPoints`. **`m_iLane` is ABSENT from this class** -- lane attribution MUST be inferred from world position. `[probe-classes]`
**Parser status:** Subscribed -- CREATE/DELETE in `BossTracker`; health tracked; positions polled. `[parser-boss]`
**Product alignment:** F9 (shrine destruction is part of the objective sequence that determines when a Base Guardian is reachable).
**Notes:** The `m_bFinal` flag is a strong candidate for distinguishing the final/last shrine in an objective sequence from the rest (confirm semantics against a live match). `m_vecWeakPoints` hints at multi-weakpoint damage accounting -- not currently used by the parser.

---

### CNPC_Boss_Tier3 (Patron)
**Hash constant:** `CNPC_BOSS_TIER3_ENTITY` `[parser-constants]`
**Inheritance:** CBaseEntity → ... → CAI_CitadelNPC. `m_iLane` is present on the class even though Patron is "base"; `m_iHealth`, `m_iMaxHealth`, `m_iTeamNum`, `m_NPCState` also confirmed. `[probe-classes]`
**Game role:** Patron (also called "Titan" in objective mask terminology) -- the final base objective. One per team. Destroying it ends the match.
**Lifecycle:** Spawns at match start; DELETE fires on destruction (match ends).
**Key identifying fields:** `m_iTeamNum`, `m_iHealth`, `m_iMaxHealth`, `m_lifeState`.
**Parser status:** Subscribed -- CREATE/DELETE/damage in `BossTracker`; positions polled. `[parser-boss]`
**Product alignment:** F2 (patron death = match end timestamp, frames all other durations), phase gating for all phase-segmented features.
**ECitadelObjective mapping:** `k_eCitadelObjective_Team[0|1]_Titan`. `[proto-gcmessages-common]`

---

### CNPC_MidBoss (Mid Boss)
**Hash constant:** `CNPC_MIDBOSS_ENTITY` `[parser-constants]`
**Inheritance:** CBaseEntity → ... → CAI_CitadelNPC. `m_NPCState`, `m_iHealth`, `m_iMaxHealth`, `m_iTeamNum` confirmed. `[probe-classes]`
**Game role:** Mid boss -- the neutral map-center objective that spawns at a defined game time, awarding a team buff to the team that kills it. Multiple spawn cycles occur during a match. **Not related to `CNPC_Neutral_SinnersSacrifice`** -- despite any colloquial overlap, the mid boss and the Sinner's Sacrifice punchable neutrals are distinct entity classes with distinct mechanics.
**Lifecycle:** Spawns via `CCitadelUserMsg_MidBossSpawned` (ID 349) trigger; destroyed by a team triggering `CCitadelUserMsg_BossKilled` (ID 347). Respawns after a timer; the exact respawn interval is not exposed as an entity field and should be measured by observing message-event spacing in a live match.
**Key identifying fields:** `m_iHealth`, `m_iMaxHealth`, `m_lifeState`. **`m_iLane` is ABSENT from this class** (neutral entity). `[probe-classes]`
**Parser status:** Positions polled in `on_tick_end`. CREATE/DELETE not explicitly handled by `BossTracker` (is_boss_entity returns false for this hash -- gap). Health is NOT damage-driven tracked for mid boss. `[parser-boss]` `[parser-replay]`
**Product alignment:** F6 (mid boss fights are teamfight events), game phase detection (MidBossSpawned marks laning-to-midgame boundary -- combine entity lifecycle with message ID 349).
**Gap:** Mid boss health timeline not tracked (BossTracker excludes this hash). Walker-first-contact analogue for mid boss is not implemented. This is a notable gap for game-phase features.

---

### CNPC_TrooperNeutral (Neutral Camp NPC)
**Hash constant:** `CNPC_TROOPERNEUTRAL_ENTITY` `[parser-constants]`
**Inheritance:** CBaseEntity → ... → CAI_CitadelNPC. `m_NPCState`, `m_iHealth`, `m_iBaseGoldReward`, `m_iTeamNum` confirmed; **`m_iLane` is ABSENT from this class.** `[probe-classes]`
**Game role:** Neutral camp NPC. Deadlock has numerous jungle camps with varying difficulty and soul rewards. These are the farmable jungle monsters distinct from lane troopers.
**Lifecycle:** Spawns at match start (or camp respawn timer). DELETE/recycling behavior for neutrals has not been measured end-to-end; the safest assumption is the same DEAD→ALIVE in-place recycling pattern used by `CNPC_Trooper`. Respawn interval must be measured from tick data rather than entity fields.
**Key identifying fields:** `m_iHealth`, `m_iTeamNum` (neutral team), `m_lifeState`, `m_iBaseGoldReward` (useful for camp-tier classification). Camp-identity attribution requires matching spawn position to camp zones -- there is no "camp id" field.
**Parser status:** Positions polled in `on_tick_end`. CREATE/DELETE NOT handled by any tracker; no lifecycle data extracted. `[parser-replay]`
**Product alignment:** F5 (farm efficiency -- jungle income source), F10 (stolen neutral camps -- detect when enemy player is near a neutral camp at time of CurrencyChanged with source=Neutrals). HIGH IMPACT gap.
**Gap:** No lifecycle tracking for neutral camps. Cannot determine: which camp was cleared, when it was cleared, who cleared it, or when it respawns. All F10 feature logic depends on this.

---

### CNPC_Neutral_SinnersSacrifice
**Hash constant:** `CNPC_NEUTRAL_SINNERSSACRIFICE_ENTITY` `[parser-constants]`
**Inheritance:** CBaseEntity → ... → CAI_CitadelNPC. `m_NPCState`, `m_iHealth`, `m_iTeamNum` confirmed; `m_iLane` ABSENT. A companion class `CNPC_Neutral_SinnersSacrifice_Hideout` also exists in the serializer table -- likely the static "pedestal" or spawner paired with the punchable entity. `[probe-classes]`
**Game role:** Punchable neutral entity tied to the Sinner's Sacrifice mechanic (player interacts via Heavy Melee near the hideout, roughly analogous to the bridge-buff activation style). Exact reward and activation window have not been observed in a live match from this pass; treat as laning-phase side objective.
**Lifecycle:** CREATE at match start (or at a scheduled trigger near the hideout); DELETE on punch/consumption. Not directly observed in this pass.
**Key identifying fields:** `m_iHealth`, `m_iTeamNum`, `m_lifeState`. Any interaction/consumption state is likely surfaced via the companion `_Hideout` entity rather than on this class.
**Parser status:** Positions polled in `on_tick_end`. No lifecycle tracking. `[parser-replay]`
**Product alignment:** F5 (farm efficiency -- optional early income), F6 (engagement around the interaction point). Lower priority than lane/neutral camps until reward value is measured.

---

### CNPC_BaseDefenseSentry
**Hash constant:** `CNPC_BASE_DEFENSE_SENTRY_ENTITY` `[parser-constants]`
**Inheritance:** CBaseEntity → ... → CAI_CitadelNPC. `m_NPCState`, `m_iHealth`, `m_iTeamNum` confirmed; `m_iLane` ABSENT. `[probe-classes]`
**Game role:** Base defense sentry -- stationary defensive NPC in the base area. Engages attackers but is not itself part of the objective sequence the attacking team must destroy.
**Lifecycle:** Spawns at match start; may be destroyed by enemy team.
**Key identifying fields:** `m_iTeamNum`, `m_iHealth`, `m_lifeState`.
**Parser status:** Positions polled in `on_tick_end`. No lifecycle tracking beyond position. `[parser-replay]`
**Product alignment:** Low for current roadmap. Background context for fight classification near base.

---

### CNPC_ShieldedSentry
**Hash constant:** `CNPC_SHIELDEDSENTRY_ENTITY` `[parser-constants]`
**Inheritance:** CBaseEntity → ... → CAI_CitadelNPC. `m_NPCState`, `m_iHealth`, `m_iTeamNum` confirmed; `m_iLane` ABSENT. `[probe-classes]`
**Game role:** Shielded variant of base defense sentry. The name and presence of two such entities per team in the probe output (one per Titan-shield slot) is consistent with the `TitanShieldGenerator[1-2]` slots in `ECitadelObjective`, though a 1:1 mapping has not been confirmed by cross-referencing an objective-mask transition in a live match.
**Lifecycle:** Spawned at match start; expected to be destroyed to remove patron shields. Lifecycle not independently observed in this pass.
**Key identifying fields:** `m_iTeamNum`, `m_iHealth`, `m_lifeState`.
**Parser status:** Positions polled. No lifecycle tracking. `[parser-replay]`
**Product alignment:** F9 late-game phase signal -- if these map to `TitanShieldGenerator`, their destruction order and timing mark the transition into "Patron exposed" state.
**ECitadelObjective mapping:** Candidate match: `k_eCitadelObjective_Team[0|1]_TitanShieldGenerator[1-2]`. Verification path: watch for `ObjectiveMask` bit flips (ID 324) coinciding with this entity's health timeline hitting zero in a live demo. `[proto-gcmessages-common]`

---

### CItemXP (Soul Orb)
**Hash constant:** `CITEMXP_ENTITY` `[parser-constants]`
**Inheritance:** CBaseEntity (item pickup; not in NPC hierarchy)
**Game role:** Soul orb spawned on unit death (lane creep, neutral, hero). Players shoot it to claim the souls; enemies can deny it by shooting it first (or it despawns). Primary mechanism for souls to flow from kills to players.
**Lifecycle:** Spawns (CREATE) when a unit dies. Despawns (DELETE) when claimed or denied, or after timeout. Lifecycle is brief -- typically under 10 seconds. `[parser-constants]` (comment on `CITEMXP_ENTITY`)
**Key identifying fields:**
- Position via `CBodyComponent` -- where the orb is floating on the map
- No `m_iTeamNum` allegiance (neutral pickup)
- No field on the class directly links an orb back to the unit that spawned it -- that association must be reconstructed from create-time proximity to a just-died NPC. Soul face value similarly has no confirmed-name field on the class and would need to be inferred from the accompanying `CurrencyChanged` event on pickup.
**Parser status:** Positions polled in `on_tick_end`. CREATE/DELETE not tracked; no association with the kill that spawned each orb. `[parser-replay]`
**Product alignment:** F1 (souls/sec -- orbs are the physical mechanism; connecting orb despawn to CurrencyChanged events would give per-kill soul value), F5 (farm efficiency -- which orbs were claimed vs. denied). Mentioned explicitly in `current-options.md` as a deferred parser data source.
**Gap:** No spawn/despawn lifecycle tracking. No association between orb entities and the kills that created them. The `current-options.md` file explicitly flags this: "requires tracking spawn/despawn lifecycle and associating each orb with the kill that created it." `[parser-constants]` (comment at line 31)

---

### CCitadel_GraveStone_Blocker
**Hash constant:** `CCITADEL_GRAVESTONE_BLOCKER_ENTITY` `[parser-constants]`
**Inheritance:** CBaseEntity. Not in the NPC hierarchy (no `m_NPCState`). `[probe-classes]`
**Game role:** Ability entity tied to a hero ability that creates a blocker/gravestone. The name is consistent with a Calico/Vyper-style ability; direct hero attribution has not been confirmed by observing the owning ability in a live match.
**Lifecycle:** Short-lived; exists during the ability duration.
**Parser status:** Hash is defined in constants; position polled would fail (`get_custom_id` returns 35 but entity is not in `should_track_position` whitelist -- gap). Actually: the entity hash 35 is in `get_custom_id` match but not in `should_track_position` -- so it is NOT position-tracked. `[parser-replay]`
**Product alignment:** None for current roadmap.

---

### CNecro_HauntingSkullEntity / CNPC_NecroSkele
**Hash constants:** `CNECRO_HAUNTINGSKULL_ENTITY`, `CNPC_NECROSKELE_ENTITY` `[parser-constants]`
**Inheritance:** `CNPC_NecroSkele` is an NPC-lineage class (`m_NPCState` present); `CNecro_HauntingSkullEntity` is a non-NPC projectile/effect entity. `[probe-classes]`
**Game role:** Summon entities whose naming matches an older "necromancer" theme; the current hero identity (likely a Wraith/Graves-lineage character) would need to be confirmed by watching which player controller's owner-entity chain points at these on creation.
**Lifecycle:** Spawned by Wraith ability use; despawn on ability end or death.
**Parser status:** Hash defined in constants; both appear in `get_custom_id` (IDs 33, 34) but are NOT in `should_track_position` -- not position-tracked. `[parser-replay]`
**Product alignment:** None for current roadmap. Could contribute to ability usage detection for F6 (fight classification) in future.

---

### CProjectile_Priest_SlideTrap_Projectile
**Hash constant:** `CPROJECTILE_PRIEST_SLIDETRAP_ENTITY` `[parser-constants]`
**Inheritance:** CBaseEntity (projectile)
**Game role:** Lash (Priest hero) slide trap projectile. Can appear as an attacker entity in damage records.
**Lifecycle:** Short-lived; exists during projectile flight.
**Parser status:** NOT position-tracked. Used in damage attribution: `get_damage_entity_id` returns `entity.index()` for this hash (not a fixed custom ID) to uniquely identify projectile damage sources. `[parser-replay]`
**Product alignment:** Damage attribution accuracy for F6 (fight classification). Not a standalone feature target.

---

## Powerups and Map Pickups

This section covers non-NPC, non-projectile entity classes representing map-placed powerups and pickups. None are currently subscribed by the parser. All classes below are confirmed present in a live demo's SendTable. `[probe-classes]`

### CCitadel_PunchablePowerup (Bridge Buffs / Runes)
**Inheritance:** CBaseEntity (no NPC lineage). 83 fields observed on the class. `[probe-classes]`
**Game role:** The "Bridge Buff" (also referred to as Rune) pickup that spawns on the bridges between lanes. A player claims the buff by performing a Heavy Melee Attack (the class name `Punchable` reflects this activation mechanic). Four buff variants are cycled -- Casting, Gun, Movement, Survival (improved equivalents of the Casting/Weapon/Spirit/Vitality item shop categories) -- and the active buff lasts 160 seconds from the moment it is claimed.
**Spawn timing:** First spawn at match clock 5:00, and every 5:00 thereafter (i.e. 5, 10, 15, 20 ... minutes match-relative). Each cycle places a new claimable powerup on the map. The user's "spawns every 5 minutes starting at 10:00" recollection is close but slightly off -- the first spawn is at 5:00, not 10:00.
**Lifecycle:** CREATE at spawn time; the entity becomes inactive/DELETEs on claim or on expiration. `m_bActive` is a strong candidate for the "currently claimable" flag.
**Key identifying fields (from probe, partial list):**
- `m_bActive` -- likely the claimable/active flag
- `m_sPickupName` -- powerup variant identifier (Casting / Gun / Movement / Survival)
- `m_flCreateTime` -- creation timestamp (replay-time)
- `m_pModifierProp` -- pointer/reference to the buff-granting modifier prop
- `m_hVacuumTarget` -- ehandle to the player being pulled toward the pickup (vacuum on-claim effect)
- `m_hOwner` -- ehandle to the current owner (post-claim?) or spawner
- Position via `CBodyComponent`
**Parser status:** NOT tracked. Not registered in `parser/src/entities/constants.rs`. Not in `should_track_position` whitelist.
**Product alignment:**
- F6 (fight classification) -- bridge-buff contests are a reliable early-mid-game engagement marker; knowing which team claimed which buff would disambiguate "pickup fight" from "random brawl".
- F8 (archetype metrics) -- buff variant (Gun vs. Movement vs. Casting vs. Survival) correlates with player archetype; who claims which is a playstyle signal.
- F11 / F12 (pressure and tempo) -- buffs shift lane-pressure potential for 160s after claim; correlating claim events with subsequent objective damage would expose "buff snowball" patterns.
- Game phase detection -- first bridge buff at 5:00 is the earliest neutral objective event in the match and aligns well with the laning-to-midgame transition boundary.

### CCitadelItemPickupRejuv (Rejuvenator)
**Inheritance:** CBaseEntity. `[probe-classes]`
**Game role:** The Rejuvenator -- the late-game neutral pickup that grants a team-wide hero-and-creep buff (the Deadlock analogue of Dota 2's Aegis/Cheese mechanic).
**Parser status:** NOT tracked.
**Product alignment:** F9 (late-game objective sequencing), F6 (rejuv fights are the largest teamfights of the match).

### CCitadelItemPickupIdol (Golden Idol)
**Inheritance:** CBaseEntity. `[probe-classes]`
**Game role:** Golden Idol pickup -- map collectible tied to the idol drop/deliver mechanic.
**Parser status:** NOT tracked.
**Product alignment:** F9 (idol is a secondary objective), F6 (idol contests are an engagement source).

### CCitadelItemPunchableNeutralGold
**Inheritance:** CBaseEntity. `[probe-classes]`
**Game role:** Punchable neutral gold stash (the "breakable gold pile" collectibles scattered on the map that drop souls when punched).
**Parser status:** NOT tracked.
**Product alignment:** F5 (farm efficiency -- supplementary income source), F1 (soul accounting completeness).

### CCitadel_BreakableProp (family)
**Inheritance:** CBaseEntity. Citadel-specific subclass of the Source 2 `CBreakableProp` hierarchy (the `CBreakable`, `CBreakableProp`, and `CPropAnimatingBreakable` base classes are also present in the SendTables). `[probe-classes]`
**Game role:** Destructible world props -- the "interactable sundry" layer of the map: crates, barrels, boxes, decorative pottery, etc. On destruction, a prop can drop a gold packet, a health orb, or a modifier buff. The runtime census confirms this family is **the highest-cardinality entity layer in a match**: 691 unique `CCitadel_BreakableProp` indices observed in a single replay, with the dropped-gold transient alone churning through ~1,680 create events across only 36 reused slots. `[probe-classes]` `[runtime-census]`
**Full family (all confirmed present in SendTables; runtime status noted where measured):**
| Class | Role | Runtime (1 match, [census]) |
|---|---|---|
| `CCitadel_BreakableProp` | Parent: destructible world prop, may drop nothing | 1,931 CREATE / 691 unique [C] |
| `CCitadel_BreakablePropPickup` | Intermediate parent for props that always drop a pickup on destruction | not directly observed (abstract?) |
| `CCitadel_BreakablePropGoldPickup` | Breakable prop that drops gold | 479 CREATE / 36 unique [C] |
| `CCitadel_BreakablePropHealthPickup` | Breakable prop that drops a health orb | 433 CREATE / 11 unique [C] |
| `CCitadel_BreakablePropModifierPickup` | Breakable prop that drops a modifier buff | 138 CREATE / 29 unique [C] |
| `CCitadel_BreakableDroppedGoldPickup` | Transient gold-orb entity spawned when any gold-dropping breakable is destroyed (pickup stage) | 1,680 CREATE / 36 unique [C] -- heavy slot reuse |
| `CCitadel_BreakableDroppedNecroPickup` | Graves-specific drop variant (corpse / necro pickup) | not observed in probed match [C] -- no Graves pick |
**Lifecycle note [I]:** The high `CREATE:unique_idx` ratio on `CCitadel_BreakableProp` (1931 / 691 ≈ 2.8×) is unusual -- for most destructibles we would expect close to 1:1 if each prop spawns once. This suggests either aggressive re-creation across rounds, or the Breakable family fires CREATE on some state transition (e.g. respawn) that is not a literal new-entity event. **Not validated**; do not treat `CREATE` on this class as a reliable "prop first appeared" signal without further probing.
**Parser status:** NOT tracked. Neither `parser/src/entities/constants.rs` nor `should_track_position` references any class in this family.
**Product alignment:** F5 (farm completeness -- props account for a non-trivial fraction of mid-lane gold), and a secondary signal for F6 where prop breaking often occurs during or right before a fight.

### CCitadel_PickupItemSpawner
**Inheritance:** CBaseEntity. `[probe-classes]`
**Game role [I]:** Spawner / controller entity that manages lifecycle of map pickup items (Rejuv / Idol / breakable-prop drops). Not a pickup itself. **Inferred from naming** -- not directly validated against pickup spawn events. Runtime census shows 2 instances with ~15 updates per instance, consistent with a state-machine that ticks a respawn timer rather than a transient pickup. `[runtime-census]`
**Parser status:** NOT tracked.
**Product alignment:** Would complement Rejuv / Idol / breakable-prop tracking if we ever want to know *where* spawns are anchored on the map vs just *when* they fire.

### CCitadel_HeroTestOrbSpawner
**Inheritance:** CBaseEntity. `[probe-classes]`
**Game role [I]:** Almost certainly tied to the Deadlock hero sandbox / practice mode -- a "test orb" spawner used for hero-ability testing in an isolated map area. Appeared in a real match replay with 2 instances and 0 updates, suggesting passive static anchors. **Inferred**; needs position probing or haste-inspector inspection to confirm whether the instances are inside or outside the playable map.
**Parser status:** NOT tracked and no product relevance.

### CCitadel_HideOutTargetSpawner
**Inheritance:** CBaseEntity. `[probe-classes]`
**Game role [I]:** Matching the naming pattern of `CNPC_Neutral_Hideout_Cat` / `CNPC_Neutral_Hideout_Rabbit`, this is likely the spawner for interactive targets in the post-match **Hideout** social space. **Not observed at runtime** in the probed match (which did not include the post-match hideout phase). No product relevance.

### CCitadelItemPickupRejuvHeroTest
**Inheritance:** CBaseEntity. Sibling of `CCitadelItemPickupRejuv`. `[probe-classes]`
**Game role [I]:** Sandbox / hero-test variant of the Rejuvenator pickup -- same pattern as `CCitadel_HeroTestOrbSpawner`. Present in SendTables but not observed in any live-match probe. No product relevance beyond "don't confuse it with the real Rejuv pickup".

---

## Entity Classes Known to Exist but Not Yet in Parser

The following entity class names have been identified from game knowledge and naming patterns but are NOT currently registered in `parser/src/entities/constants.rs`. They are candidates for future subscription.

| Entity Class | Game Role | Evidence | Product Relevance |
|---|---|---|---|
| `CNPC_TrooperNeutralNodeMover` | Neutral mover/support entity paired with `CNPC_TrooperNeutral` (role unconfirmed) | `[probe-classes]` | F10, F5 context |
| `CNPC_Neutral_SinnersSacrifice_Hideout` | Static companion of the Sinner's Sacrifice punchable entity | `[probe-classes]` | F5 side objective context |
| `CCitadel_PunchablePowerup` | Bridge buffs / runes (see Powerups section) | `[probe-classes]` | F6, F8, F11, F12, phase detection -- HIGH |
| `CCitadelItemPickupRejuv` | Rejuvenator | `[probe-classes]` | F9, F6 -- HIGH |
| `CCitadelItemPickupIdol` | Golden Idol | `[probe-classes]` | F9, F6 |
| `CCitadelItemPunchableNeutralGold` | Punchable gold stash | `[probe-classes]` | F5, F1 |
| `CCitadel_BreakableProp` family (7 classes) | Destructible world props + gold/health/modifier drops -- full family documented above | `[probe-classes]` `[runtime-census]` | F5 completeness |
| `CCitadel_PickupItemSpawner` | Spawner controller for map pickups (2 runtime instances) | `[probe-classes]` `[runtime-census]` | Low -- complements pickup tracking only |
| `CCitadel_HeroTestOrbSpawner` | [I] Sandbox / hero-test orb spawner (2 runtime instances, 0 updates) | `[probe-classes]` `[runtime-census]` | None |
| `CCitadel_HideOutTargetSpawner` | [I] Post-match hideout target spawner (not observed in match phase) | `[probe-classes]` | None |
| `CCitadelItemPickupRejuvHeroTest` | [I] Sandbox / hero-test Rejuv variant | `[probe-classes]` | None |
| `CNPC_Neutral_Bug` | [I] Neutral jungle creature not previously cataloged (97 CREATE / 69 unique in one match) | `[probe-classes]` `[runtime-census]` | F10, F5 context |
| `CCitadelObserverPawn` | Spectator / observer pawn; 80 unique slots in one match | `[probe-classes]` `[runtime-census]` | None (but useful for filtering out non-player pawns) |
| `CCitadelTeam` | Team state entities (5 instances per match -- 2 teams + 3 auxiliary slots [I]) | `[probe-classes]` `[runtime-census]` | Low |
| Mid boss buff entity | Which team secured the mid boss buff | Mentioned in `current-options.md`; entity class not yet located in SendTable | Deferred |
| Hero ability entities (per hero) | Ability state tracking | Many named ability classes exist in the SendTable | F6 (fight classification) future |

Note: The mid boss "buff carrier" entity has not been isolated in the probe output. The associated game event is covered by `CCitadelUserMsg_BossKilled` (ID 347) and the `CNPC_MidBoss` lifecycle, so the buff-carrier entity is not currently a blocker for F6 phase detection.

---

## ECitadelObjective -- Entity-to-Objective Mapping

The `ECitadelObjective` enum (from `citadel_gcmessages_common.proto`) maps objective bitmask positions to specific lane structures. This bridges entity classes to `ObjectiveMask` (ID 324) message values. `[proto-gcmessages-common]`

| Objective enum values | Entity class | Team |
|---|---|---|
| `Team0_Tier1_Lane[1-4]` | `CNPC_TrooperBoss` | 0 |
| `Team0_Tier2_Lane[1-4]` | `CNPC_Boss_Tier2` | 0 |
| `Team0_Titan` | `CNPC_Boss_Tier3` | 0 |
| `Team0_TitanShieldGenerator[1-2]` | `CNPC_ShieldedSentry` (candidate match; verification path: watch `ObjectiveMask` ID 324 bit flips against shielded-sentry health timeline) | 0 |
| `Team0_BarrackBoss_Lane[1-4]` | `CNPC_BarrackBoss` | 0 |
| `Team1_*` | Mirror of above | 1 |
| `Neutral_Mid` | `CNPC_MidBoss` | N/A |

`CCitadel_Destroyable_Building` (Shrine) does not appear to have a direct `ECitadelObjective` entry. Because the class has no `m_iLane` field either, lane/objective attribution for shrines must be reconstructed from world position and the surrounding lane's objective-mask bits rather than from a direct enum entry. `[probe-classes]`

---

## Lane Color to Lane Number Mapping

`CMsgLaneColor` enum from `citadel_gcmessages_common.proto` maps color to lane number. `[proto-gcmessages-common]`

| Color | Enum value | m_iLane value |
|---|---|---|
| Yellow | 1 | 1 (candidate pairing -- verify) |
| Green | 3 | 2 (candidate pairing -- verify) |
| Blue | 4 | 3 (candidate pairing -- verify) |
| Purple | 6 | 4 (candidate pairing -- verify) |

Lane-to-color pairing is used by `CCitadelUserMsg_TeamMsg` (ID 352) `lane_color` field. The mapping between `m_iLane` integer (1-4) on NPC entities and `CMsgLaneColor` enum values has not been confirmed from a single source. Validation path: log `(m_iLane, lane_color)` tuples from `CCitadelUserMsg_TeamMsg` against a matching guardian/walker position in a live demo, or check against `CCitadelUserMsg_LaneAssigned` if present. Treat the specific pairings above as candidate matches only.

---

## Gap Analysis

Ranked by product-strategy impact. All items below represent entity types that are available in replay data but either not tracked, or only partially tracked, by our parser.

---

### Gap 1: CNPC_TrooperNeutral -- Neutral Camp Lifecycle Not Tracked
**Missing:** CREATE/DELETE event handling, camp-identity association, respawn timing.
**What we have:** Positions polled in `on_tick_end` (so we know where neutral NPCs are each second), but no spawn/death events captured.
**Why it matters:**
- F10 (stolen enemy neutral camps) is fully blocked without knowing *when* a neutral camp was cleared and *who cleared it*. The feature depends on correlating a `CurrencyChanged` event (source=Neutrals) with the player's position relative to the cleared camp. Without camp-death timestamps, the positional correlation cannot be bounded.
- F5 (farm efficiency by source) can partially use `CurrencyChanged` source=Neutrals=3 to count jungle income, but cannot attribute it to specific camps or determine camp-steal vs. own-camp farm.
**To fix:** Add neutral camp CREATE/DELETE handling in `on_entity` (similar to `BossTracker` but for neutrals). Key data to capture: entity index, position at creation (maps to camp zone), team, spawn time, death time.
**Priority:** HIGH -- blocks F10 entirely; F5 partial workaround exists via message source filtering.

---

### Gap 2: CItemXP (Soul Orb) -- Spawn/Despawn Not Tracked
**Missing:** CREATE (which kill spawned this orb, face value), DELETE (claimed vs. denied vs. timeout), association with spawning unit.
**What we have:** Per-second position of all soul orbs. Cannot tell claimed vs. denied vs. floating.
**Why it matters:**
- F1 (souls per second) uses `CurrencyChanged` messages, which fire when souls are collected -- but the orb entity data would allow calculating *denied* souls (orbs that spawned and then despawned without a corresponding CurrencyChanged event for either team).
- Deny detection: `EGoldSource.Denies=7` in `CurrencyChanged` fires when a player denies a creep, but deny tracking for hero kills or neutral kills via orb entity lifecycle is not possible without tracking orb creation and destruction.
- The `current-options.md` explicitly flags this as deferred high-value work.
**To fix:** Track CItemXP CREATE (capture position, timestamp, nearby killed entity to infer orb source) and DELETE (note if a CurrencyChanged event precedes delete -- claimed; if not -- denied/timed-out).
**Priority:** MEDIUM-HIGH -- enriches F1, enables deny analytics. F1 base functionality works without it via message subscription.

---

### Gap 3: CNPC_MidBoss -- Health Not Tracked; No Lifecycle in BossTracker
**Missing:** `BossTracker.is_boss_entity()` returns false for `CNPC_MIDBOSS_ENTITY`. Health timeline not sampled. Spawn-to-death duration not explicitly recorded (MidBossSpawned message provides spawn tick; BossKilled message provides death tick -- but entity-level health progress during the fight is missing).
**What we have:** Positions polled. Spawn and death events available via messages (ID 349, 347). Entity exists in `should_track_position` whitelist.
**Why it matters:**
- F6 (fight classification) -- mid boss fights are typically 5+ player team fights. Knowing the mid boss health timeline would allow classifying how many players were attacking it at each moment (via damage events), not just that a fight happened.
- Game phase detection uses the MidBoss message events adequately for phase timing, but entity-level health during the fight (how long did it take, was it contested) is richer.
**To fix:** Add `CNPC_MIDBOSS_ENTITY` to `BossTracker.is_boss_entity()`. Register CREATE/DELETE in `on_entity`. Record health samples on each damage event to mid boss (same pattern as other bosses).
**Priority:** MEDIUM -- game phase features work without it via messages; entity tracking adds fight-quality depth for F6. Effort is low (one-line change to `is_boss_entity` plus verifying field names via haste-inspector).

---

### Gap 4: CCitadel_PunchablePowerup (Bridge Buffs) -- Not Subscribed
**Missing:** Entity hash constant, CREATE/DELETE tracking, claim detection, variant identification (Casting / Gun / Movement / Survival), claim-team/player attribution.
**What we have:** Nothing. The class is confirmed present in the SendTable (`[probe-classes]`), but it is not in `parser/src/entities/constants.rs`, not in `should_track_position`, and not handled in any tracker.
**Why it matters:**
- **Phase detection:** The first bridge buff spawns at match clock 5:00 -- the earliest neutral-objective timestamp in the match. This is a cleaner laning-to-midgame boundary signal than "heroes leave lane" heuristics.
- **F6 (fight classification):** Bridge-buff contests are a predictable engagement source every 5 minutes. Labeling "pickup fight" vs "ambush" vs "random skirmish" needs buff-spawn times + claim events.
- **F8 (archetype metrics):** The four buff variants (Casting / Gun / Movement / Survival) each nudge a player toward a different playstyle; who claims which and how often is an archetype signal.
- **F11 / F12 (pressure / tempo):** The buff lasts 160s after claim; correlating claim events with subsequent objective damage exposes "buff snowball" patterns where one team converts a buff into a guardian kill.
**To fix:** (1) Add `CCITADEL_PUNCHABLEPOWERUP_ENTITY` hash to `parser/src/entities/constants.rs`. (2) Register CREATE/DELETE in `on_entity`; treat CREATE as "buff is claimable", DELETE as "claimed or expired". (3) On DELETE, check proximity of hero pawns to attribute claim to a player (or to `m_hOwner` / `m_hVacuumTarget` if those resolve at DELETE time -- verify via haste-inspector). (4) Read `m_sPickupName` to tag the variant. (5) Emit a new `PowerupClaimEvent` timeline for downstream consumption.
**Priority:** MEDIUM-HIGH -- enables phase detection and enriches multiple F-features with low effort. The class is already confirmed present; only the subscription and tracker wiring are missing.

---

### Gap 5: Rejuv / Idol / Breakable Pickups -- Not Subscribed
**Missing:** `CCitadelItemPickupRejuv`, `CCitadelItemPickupIdol`, `CCitadelItemPunchableNeutralGold`, and the `CCitadel_BreakableProp{Modifier,Health,Gold}Pickup` family are all confirmed in the SendTable but not subscribed.
**Why it matters:** Rejuv and Idol are late-game objective events (F9, F6). The breakable / punchable gold family contributes to F5 farm-completeness accounting.
**To fix:** Same shape as Gap 4 -- add hash constants, register lifecycle, attribute claim to nearby hero. Can be tackled as one shard with Gap 4 since all follow the same subscription pattern.
**Priority:** MEDIUM -- Rejuv is a single decisive late-game event; Idol and prop pickups are supporting signals.
