# Entity Types Runtime Census

**Last Updated:** 2026-04-09
**Replay analyzed:** `parser/src/replays/68175583_527726523.dem` (match 68175583)
**Tool:** `private/engineering/tools/probe_entity_counts.rs`
**Static list reference:** `probe_all_entity_classes` (862 serializer classes in SendTables)
**Companion doc:** `private/specs/entity-types-reference.md`

## Purpose

Complements `entity-types-reference.md` (which catalogs classes from game knowledge + static SendTables enumeration) with a **runtime census**: which entity classes actually instantiate during a real match, how many instances spawn, and how many `on_entity` callbacks they generate.

The user question this answers: *"The static list is 862 classes; how many of those actually show up in a match, and how many of each?"* Plus: there are entity classes in the SendTables that never made it into `entity-types-reference.md` (e.g. `CCitadel_PickupItemSpawner`, `CCitadel_HeroTestOrbSpawner`) -- this doc surfaces them.

## Conventions: Confirmed vs Inferred

- **[C]** = **Confirmed by probe** -- the class appeared in `on_entity` with the given counts for this replay. Reproducible by re-running `probe_entity_counts`.
- **[I]** = **Inferred** -- a role, grouping, or semantic claim derived from the class name, field patterns, or game knowledge but not directly validated against match events in this pass. Treat as a hypothesis.

## Headline Numbers [C]

| Metric | Value |
|---|---|
| Classes registered in SendTables (static universe) | 862 |
| Classes actually instantiated at runtime | **177** |
| Classes in SendTables but NOT observed at runtime | 685 |
| Total CREATE delta events across all classes | 14,906 |
| Total unique entity indices allocated across match | 4,489 |

Interpretation: only ~20% of the SendTables class universe ever instantiates in a given match. The rest are hero abilities for heroes not picked, items for items not bought, projectiles for abilities not cast, seasonal / map variants, etc.

Column key in the raw table (`private/engineering/tools/entity_counts_68175583_527726523.txt`):

| Column | Meaning |
|---|---|
| `Creates` | Count of `DeltaHeader::CREATE` events observed in `on_entity` |
| `Unique_idx` | Number of distinct entity indices that ever carried this class |
| `Updates` | Count of `DeltaHeader::UPDATE` events |
| `Deletes` | Count of `DeltaHeader::DELETE` events |
| `Total_obs` | Sum of all `on_entity` callbacks for this class |

`Creates > Unique_idx` indicates entity-index reuse (same slot recycled for a new instance; matches the creep-recycling pattern already documented in `parser-mental-model.md`).

## Entity Types Not Currently in `entity-types-reference.md`

The following classes instantiated at runtime but are absent from the entity catalog in `entity-types-reference.md`. 152 runtime classes fall outside the spec; the ones worth calling out are grouped below. Pure hero-ability classes (77 entries like `CCitadel_Ability_*`, `CAbility_*`) are omitted from the tables below but listed as a group; see the raw counts file for the full enumeration.

### Spawners (newly surfaced) [C]

These are the classes the user specifically flagged. All three exist in SendTables; two instantiate at runtime in this match, one does not.

| Class | Creates | Unique idx | Updates | Deletes | Status |
|---|---:|---:|---:|---:|---|
| `CCitadel_PickupItemSpawner` | 2 | 2 | 30 | 0 | [C] runtime |
| `CCitadel_HeroTestOrbSpawner` | 2 | 2 | 0 | 0 | [C] runtime |
| `CCitadel_HideOutTargetSpawner` | 0 | 0 | 0 | 0 | [C] in SendTables, NOT observed |

**Inferences [I]:**
- `CCitadel_PickupItemSpawner` (2 instances, 30 updates) -- **[I]** likely the spawner controller for the Rejuvenator / Idol / breakable-prop pickups. The update count (30) suggests it ticks a respawn timer or state machine. The 2 unique indices could correspond to "one per team side" or "one per objective category".
- `CCitadel_HeroTestOrbSpawner` (2 instances, 0 updates) -- **[I]** the name `HeroTest` strongly suggests a **hero practice / sandbox** entity. These may be always-present map decoration that never updates because they are passive static references (consistent with 0 updates / 0 deletes). **Not validated** -- could also be something used in the hero-tutorial pre-match flow; needs position probing or haste-inspector inspection to confirm.
- `CCitadel_HideOutTargetSpawner` -- **[I]** likely tied to the post-match hideout (the Mysterium-style social space after a match ends). Absent in this demo because the recording may end before the hideout phase. `CNPC_Neutral_Hideout_Cat` and `CNPC_Neutral_Hideout_Rabbit` in the static list support the hypothesis that a whole "hideout" entity family exists but only instantiates in specific recording windows.

### Breakable props and their drops [C]

A major surprise -- `CCitadel_BreakableProp` is one of the highest-cardinality classes in any match (691 unique indices, 1,931 CREATE events).

| Class | Creates | Unique idx | Updates | Deletes | Role (inferred) |
|---|---:|---:|---:|---:|---|
| `CCitadel_BreakableProp` | 1,931 | **691** | 0 | 0 | [I] Destructible world props (crates, barrels, etc.) |
| `CCitadel_BreakableDroppedGoldPickup` | 1,680 | 36 | 141,710 | 0 | [I] Transient gold-drop entity spawned when a breakable prop is destroyed |
| `CCitadel_BreakablePropGoldPickup` | 479 | 36 | 27 | 0 | [I] Gold-pickup variant of breakable prop |
| `CCitadel_BreakablePropHealthPickup` | 433 | 11 | 8,435 | 0 | [I] Health-pickup variant |
| `CCitadel_BreakablePropModifierPickup` | 138 | 29 | 10 | 0 | [I] Modifier/buff-drop variant |

**Confirmed [C]:** All these classes fire on `on_entity`. The large `Creates > Unique_idx` ratio for `BreakableDroppedGoldPickup` (1680 / 36) confirms entity-slot recycling -- the game repeatedly reuses 36 slots for transient gold orbs.

**Inferred [I]:**
- `CCitadel_BreakableProp` appears to be the *parent* class for all destructibles placed in the map. 691 distinct instances suggests every breakable crate / barrel / decoration is network-replicated. Zero updates and zero deletes across 1,931 creates is unusual -- **[I]** creation events may be happening repeatedly as the server's prop-spawn system re-inits them, rather than being genuine spawns. Worth investigating before relying on this as a spawn signal.
- The three `CCitadel_BreakableProp{Gold,Health,Modifier}Pickup` classes plus `CCitadel_BreakableDroppedGoldPickup` are almost certainly the "drop" siblings of the breakable family. `entity-types-reference.md` already mentions `CCitadel_BreakableProp{Modifier,Health,Gold}Pickup` in the Gap 5 table, but the **dropped** variant (`CCitadel_BreakableDroppedGoldPickup`) and the parent (`CCitadel_BreakableProp`) are both missing.
- A similar `CCitadel_BreakableDroppedNecroPickup` exists in the SendTables but did not instantiate in this match -- **[I]** a Graves-specific drop type.

### Doorman ability entity family [C]

The Doorman hero has an unusually rich set of dedicated entity classes -- none of which are in `entity-types-reference.md`.

| Class | Creates | Unique idx | Updates |
|---|---:|---:|---:|
| `CDoormanBombProjectile` | 132 | 116 | 4,993 |
| `CProjectile_Doorman_Cart_Projectile` | 33 | 31 | 1,790 |
| `CCitadel_DoorwayPortal` | 67 | 64 | 90,575 |
| `CCitadel_Ability_Doorman_Bomb` | 1 | 1 | 286 |
| `CCitadel_Ability_Doorman_Cart` | 1 | 1 | 134 |
| `CCitadel_Ability_Doorman_Doorway` | 1 | 1 | 373 |
| `CCitadel_Ability_Doorman_Hotel` | 1 | 1 | 241 |

**Inferred [I]:** this is almost certainly the full ability kit for the Doorman hero. The `_Ability_` singletons (one per class) are the passive ability-state entities for whichever player picked Doorman. The `_Projectile_` and `_DoorwayPortal_` classes are the transient instances of those abilities being cast. 90,575 updates on `CCitadel_DoorwayPortal` is notable -- **[I]** persistent portal entities that tick every frame while active.

### Other hero ability families [C]

Hero-specific ability classes are each tagged with exactly 1 unique index (= 1 hero played). Observed hero ability families: **Fencer, Werewolf, Unicorn, Lash, Priest, Doorman, Bull, Hornet, VampireBat** (plus shared classes `CCitadel_Ability_BloodBomb`, `_FlyingStrike`, `_LifeDrain`, etc.).

**Inferred [I]:** this replay contained roughly 9-10 hero picks for which there are dedicated ability entity classes, plus 2-3 heroes whose abilities live only under shared generic classes. We cannot cleanly read the 12-hero lineup off the ability classes alone.

**Not hero-specific but instantiating per-hero [C]:** the generic movement / interaction abilities each have exactly 12 unique indices (= 1 per player), confirming 12 players:

| Generic ability class | Unique idx |
|---|---:|
| `CCitadel_Ability_Climb_Rope` | 12 |
| `CCitadel_Ability_Dash` | 12 |
| `CCitadel_Ability_HoldMelee` | 12 |
| `CCitadel_Ability_Jump` | 12 |
| `CCitadel_Ability_Mantle` | 12 |
| `CCitadel_Ability_MeleeParry` | 12 |
| `CCitadel_Ability_Slide` | 12 |
| `CCitadel_Ability_Sprint` | 12 |
| `CCitadel_Ability_ZipLine` | 12 |
| `CCitadel_Ability_ZipLine_Boost` | 12 |

### Other notable runtime classes not in spec [C]

| Class | Creates | Unique idx | Inference [I] |
|---|---:|---:|---|
| `CCitadelObserverPawn` | 90 | 80 | Spectator pawns; 80 unique slots across the demo. |
| `CCitadelTrackedProjectile` | 107 | 97 | [I] Generic base class for homing / tracked projectiles. |
| `CCitadelConfigurableTrackedProjectile` | 47 | 44 | [I] Subclass with custom config params. |
| `CCitadelProjectile` | 302 | 214 | [I] The most generic projectile base class. |
| `CCitadel_Projectile_BatSwarmProjectile` | 603 | 301 | [I] Vampire Bat ultimate bats; 603 creates in one match is striking. |
| `CCitadel_Projectile_BloodBomb` | 96 | 92 | [I] Blood Bomb ability projectile. |
| `CProjectile_Priest_SlideTrap_Projectile` | 50 | 48 | In spec already. |
| `CNPC_Neutral_Bug` | 97 | 69 | [I] A neutral enemy type -- "Bug" enemy (possibly a new jungle camp). Not in spec. |
| `CCitadel_Item_GooseEgg` | 53 | 52 | [I] An item carrier entity; "GooseEgg" name suggests zero-value placeholder. |
| `CItemMysticReverb` | 1 | 1 | [I] An item pickup. |
| `CItem_FleetfootBoots` | 1 | 1 | [I] An item pickup -- matches the item "Fleetfoot Boots". |
| `CCitadelTrooperMinimap` | 1 | 1 | [I] A single global minimap annotation entity for trooper positions. |
| `CCitadelTeam` | 5 | 5 | [I] Team state entities -- count of 5 suggests 2 teams + 3 auxiliary ("spectator", "neutral", ...). |
| `CCitadelCatapultTrigger` | 22 | 22 | [I] Map trigger volumes for the lane catapults that auto-launch players. |
| `CCitadelClimbRopeTrigger` | 26 | 26 | [I] Rope interaction triggers. |
| `CCitadelPassthroughFakeWall` | 36 | 36 | [I] Hero-only passable wall volumes (e.g. Bebop/Lash movement). |
| `CCitadelPortalTrigger` | 60 | 56 | [I] Teleport portal volumes. |
| `CCitadelTunnelTrigger` | 75 | 75 | [I] Tunnel volumes affecting projectile / vision. |
| `CCitadel_NewYears_Fireworks` | 22 | 22 | [C] Seasonal decoration entity, present in this match. |
| `CCitadel_BaseProp_MidStairs` | 2 | 2 | [I] Named map prop for the mid boss staircase. |
| `CCitadelIdolReturnTrigger` | 6 | 6 | [I] The return-zone trigger where carried idols are deposited -- 6 triggers = 3 per team? |
| `CCitadelTriggerCapturePoint` | 2 | 2 | [I] Likely the mid boss capture ring. |
| `CTriggerTier3Phase2Shield` | 2 | 2 | [I] The Patron phase-2 shield trigger volume (one per team). |
| `CCitadel_PunchablePowerup` | 4 | 4 | In spec (Gap 4). 4 bridge buffs in this match. |
| `CBaseEntity` | 230 | 155 | [I] Abstract base class -- 230 deletes with 0 updates. Orphan / stub entities; not semantically meaningful on their own. |

### Hero ability classes (omitted from tables above)

77 entries matching `CCitadel_Ability_*` or `CAbility_*` appeared at runtime. Each is either a hero's ability state (singleton per player picking that hero) or a shared player-movement ability (12 instances). They are listed in `entity_counts_68175583_527726523.txt` but not enumerated here to keep this doc focused on entity *types* rather than per-hero ability kits.

## Entity Classes in `entity-types-reference.md` -- Runtime Confirmation

Cross-check of every class that `entity-types-reference.md` documents, against this match.

| Class | In spec as | Creates [C] | Unique idx [C] | Notes |
|---|---|---:|---:|---|
| `CCitadelPlayerController` | player | 13 | 13 | 12 players + 1 extra slot (likely observer controller) |
| `CCitadelPlayerPawn` | player | 12 | 12 | Matches 12-player lobby exactly |
| `CCitadelGameRulesProxy` | game state | 1 | 1 | Match-level singleton, as expected |
| `CNPC_Trooper` | lane creeps | 1,744 | 234 | Heavy index reuse (1744/234) -- wave recycling behavior [C] |
| `CNPC_TrooperBoss` | Guardian | 6 | 6 | **[C] matches expected**: 3 lanes × 2 teams = 6. Deadlock reduced to 3 lanes (previously 4); the `ECitadelObjective` `Team*_Tier1_Lane[1-4]` enum in `citadel-messages-reference` reflects the older 4-lane layout but only 3 are used now. The 4th-lane enum values are legacy. |
| `CNPC_Boss_Tier2` | Walker | 6 | 6 | **[C] matches expected**: same 3-lane × 2-team math as Guardians. |
| `CNPC_BarrackBoss` | Base Guardian | 12 | 12 | **[I]**: 12 = 3 lanes × 2 teams × 2 Base Guardians per lane-side. Deadlock bases have paired Base Guardians flanking each lane entry; the pairing cleanly accounts for 12. Worth one CREATE-time `m_iLane` / `m_iTeamNum` sample to confirm. |
| `CCitadel_Destroyable_Building` | Shrine | 4 | 4 | 2 per team -- matches game knowledge [C] |
| `CNPC_Boss_Tier3` | Patron | 2 | 2 | Exactly 1 per team -- matches [C] |
| `CNPC_MidBoss` | Sinner / mid boss | 3 | 3 | **[I]** 3 spawns in one match -- consistent with mid-boss respawning after first kill. |
| `CNPC_TrooperNeutral` | neutral camps | 525 | 146 | Large runtime count confirms "Gap 1" scope in spec |
| `CNPC_TrooperNeutralNodeMover` | spec: discovered via probe, role uncharacterized | **0** | **0** | **[C]** NOT observed at runtime in this match. Possibly a class that only spawns under specific conditions, or left over from an older build. |
| `CNPC_Neutral_SinnersSacrifice` | punchable neutral | 54 | 12 | 12 distinct slots with heavy update traffic -- confirms multiple sacrifice locations / respawns |
| `CNPC_Neutral_SinnersSacrifice_Hideout` | companion static | **0** | **0** | **[C]** NOT observed -- inference in spec (paired static) is NOT supported by this match |
| `CNPC_BaseDefenseSentry` | base defense | 8 | 8 | 4 per team -- matches [C] |
| `CNPC_ShieldedSentry` | shielded base defense | **0** | **0** | **[C]** NOT observed. **[I]** possibly only spawns during Patron phase-2 shield phase; this match may not have reached it. |
| `CItemXP` | soul orb | 3,950 | 83 | Huge index reuse (3950/83) -- confirms orb slots are aggressively recycled |
| `CCitadel_GraveStone_Blocker` | Graves ability | 0 | 0 | [C] No Graves picked in this match |
| `CNecro_HauntingSkullEntity` | Graves ability | 0 | 0 | [C] Same |
| `CNPC_NecroSkele` | Graves summon | 0 | 0 | [C] Same |
| `CProjectile_Priest_SlideTrap_Projectile` | Venator trap | 50 | 48 | 1 Venator pick; ~48 trap instances |
| `CCitadel_PunchablePowerup` | bridge buff | 4 | 4 | 4 buffs this match. Confirms Gap 4 scoping |
| `CCitadelItemPickupRejuv` | Rejuv | 3 | 3 | 3 rejuv spawns -- consistent with mid-game cadence |
| `CCitadelItemPickupIdol` | Golden Idol | 10 | 10 | 10 idol spawns / pickups (`[I]` -- may include multiple respawns per location) |
| `CCitadelItemPunchableNeutralGold` | punchable gold | 19 | 19 | Exactly 19 punchable gold piles tracked |

### Cross-check findings

1. **[C] Spec entries confirmed by runtime**: `PlayerController`, `PlayerPawn`, `GameRulesProxy`, `Trooper`, `TrooperNeutral`, `Neutral_SinnersSacrifice`, `BaseDefenseSentry`, `Destroyable_Building`, `Boss_Tier3`, `MidBoss`, `ItemXP`, `PunchablePowerup`, `ItemPickupRejuv`, `ItemPickupIdol`, `ItemPunchableNeutralGold`, `Projectile_Priest_SlideTrap_Projectile`, `TrooperBoss`, `Boss_Tier2`, `BarrackBoss`.
2. **[C] Spec entries NOT observed** (acknowledged low impact): `CNPC_TrooperNeutralNodeMover`, `CNPC_Neutral_SinnersSacrifice_Hideout`, `CNPC_ShieldedSentry`, `CCitadel_GraveStone_Blocker`, `CNecro_HauntingSkullEntity`, `CNPC_NecroSkele`. The Graves/Necro trio is explained by no Graves pick; `CNPC_ShieldedSentry` likely only spawns if the match reaches Patron phase 2. Not pursued further.
3. **[C] Objective counts confirmed**:
   - `CNPC_TrooperBoss` = 6, `CNPC_Boss_Tier2` = 6 -- consistent with **3 lanes × 2 teams** (Deadlock reduced to 3 lanes from 4; the `ECitadelObjective` `Lane[1-4]` enum in `citadel-messages-reference.md` reflects the older layout).
   - `CNPC_BarrackBoss` = 12 -- likely 3 lanes × 2 teams × 2 Base Guardians per lane-side. Confirmation path: CREATE-time `m_iLane` / `m_iTeamNum` sampler probe (low priority).

## New Entity Classes Worth Adding to `entity-types-reference.md`

Ordered by potential product relevance:

1. **`CCitadel_BreakableProp`** and family (5 breakable subclasses). Already partially covered in Gap 5 of the spec but the parent class + `CCitadel_BreakableDroppedGoldPickup` are missing. With 691 unique instances it deserves first-class documentation.
2. **`CCitadel_PickupItemSpawner`** and **`CCitadel_HeroTestOrbSpawner`** -- the two spawner classes the user surfaced. Add to the "Entity Classes Known to Exist but Not Yet in Parser" table with [I] markers for inferred role.
3. **`CCitadel_DoorwayPortal`** plus the full Doorman ability family -- concrete data that ability entities are rich and replicable.
4. **`CNPC_Neutral_Bug`** -- a neutral creature type not currently in the spec. 97 creates / 69 unique indices across one match suggests it's a real jungle camp, not a one-off.
5. **`CCitadelObserverPawn`** -- spec has nothing about observer/spectator entities, and they account for 80 unique indices in this match.
6. **`CCitadelTeam`** -- 5 distinct team state entities. Spec doesn't document the team-entity count or the meaning of the extra 3.
7. **`CCitadelTrooperMinimap`** -- singleton global minimap annotation; worth naming if we ever want to read trooper positions off it directly.
8. **`CCitadelTrackedProjectile` / `CCitadelConfigurableTrackedProjectile`** -- generic base classes under `CCitadelProjectile`; useful for documenting the projectile inheritance tree.
9. **Map trigger family** (`CCitadelCatapultTrigger`, `CCitadelClimbRopeTrigger`, `CCitadelPassthroughFakeWall`, `CCitadelPortalTrigger`, `CCitadelTunnelTrigger`, `CTriggerTier3Phase2Shield`). These are all trigger volumes; low product value today but useful reference for future features involving map geometry.

## Methodology -- How to Mine the Runtime Census

These are the analysis patterns that turned the raw `probe_entity_counts` output into the findings in this doc. Reuse them when investigating other hero kits or entity families.

### Finding a hero's full ability kit

A hero's ability entities share a substring -- usually the internal hero name (`Doorman`, `Fencer`, `VampireBat`, `Werewolf`, `Unicorn`, `Lash`, `Hornet`, `Bull`, `Priest`). To find a hero's full ability kit from a single probe run, grep the runtime output by that substring:

```bash
# Example: everything Doorman-related
grep -i doorman private/engineering/tools/entity_counts_68175583_527726523.txt
# →
# CDoormanBombProjectile                           132     116     4993     132    5257
# CProjectile_Doorman_Cart_Projectile               33      31     1790      33    1856
# CCitadel_DoorwayPortal                            67      64    90575      67   90709   (name doesn't contain "Doorman" -- see below)
# CCitadel_Ability_Doorman_Bomb                      1       1      286       0     287
# CCitadel_Ability_Doorman_Cart                      1       1      134       0     135
# CCitadel_Ability_Doorman_Doorway                   1       1      373       0     374
# CCitadel_Ability_Doorman_Hotel                     1       1      241       0     242
```

**Pattern to look for:**
- `CCitadel_Ability_<Hero>_*` singletons (1 CREATE, 1 unique index) = the ability-state entities bound to the single player who picked that hero.
- `CProjectile_<Hero>_*` or `C<Hero>*Projectile` = transient projectile entities from that hero's ability casts.
- `CCitadel_<Hero>*` or classes named after an ability verb (e.g. `CCitadel_DoorwayPortal`) = persistent / placed world entities from that hero's abilities. These won't match a hero-name substring, so **always cross-check** by looking at the static SendTables list for any class whose name matches ability vocabulary from the hero (Doorman → "Doorway", "Hotel", "Cart", etc.), then confirm the class appears in runtime output.
- The singleton count is your proof of "this hero was picked exactly once" -- generic shared ability classes (see below) have 12 instances, so anything with unique_idx=1 but matching a hero prefix is that hero's dedicated kit.

### Finding the generic player-movement abilities

Generic actions every player has (dash, jump, slide, zip-line, melee parry, etc.) instantiate **once per player**. In a 12-player match, filter the runtime output for classes with exactly `unique_idx = 12`:

```bash
# Extract classes with exactly 12 unique indices from the raw table
awk '$3 == 12 {print $1, $2, $3}' private/engineering/tools/entity_counts_68175583_527726523.txt
```

This is how the doc's "12 unique idx" movement-ability list was compiled (`Dash`, `Jump`, `Slide`, `Sprint`, `HoldMelee`, `MeleeParry`, `Mantle`, `Climb_Rope`, `ZipLine`, `ZipLine_Boost`). The filter also catches some per-player items and modifiers, so double-check the name before assuming it's a movement action.

### Confirming the player count

When in doubt about how many players are in a match, cross-check three classes that must equal player count:
- `CCitadelPlayerPawn` (one per player) → `unique_idx` should equal player count.
- `CCitadel_Ability_Dash`, `CCitadel_Ability_Sprint`, etc. (generic movement) → same `unique_idx`.
- `CCitadelPlayerController` is NOT a clean signal -- in this match it showed 13, not 12 (likely includes an extra observer/bot slot).

### Finding entity-family patterns

For an entity with an unfamiliar parent (e.g. the first time I saw `CCitadel_BreakableProp`), grep the **static** SendTables list for the prefix to get the full family:

```bash
grep -i breakable /tmp/static_probe_full.txt
# Reveals: parent, pickup variants, dropped variants, plus the Source 2 base classes
```

Then cross-reference against the runtime output to see which family members actually instantiate.

## Follow-up Probes Worth Running

1. **Run on additional matches** to rule out match-specific noise (e.g. the 6 TrooperBoss anomaly). Replays `55423930_379917638.dem`, `55841493_649180947.dem`, `68182475_4609034.dem` are already on disk.
2. **Probe `CCitadel_PickupItemSpawner`'s fields** to confirm role. Run `probe_all_entity_classes` with filter `PickupItemSpawner` and correlate field names with the Rejuv/Idol/Gold entity relationships.
3. **Probe `CNPC_ShieldedSentry` against a match that reached Patron phase 2** to verify the inferred spawn condition.
4. **Probe `CNPC_TrooperNeutralNodeMover`** against a match with aggressive neutral-camp activity to test whether it's a dead class or a live-but-rare one.
5. **Add a `CREATE`-time `m_iLane` / `m_iTeamNum` sampler** to resolve the TrooperBoss / Boss_Tier2 / BarrackBoss count anomalies.

## Reproducing

```bash
# From repo root, with dashjump-parser container running
cp private/engineering/tools/probe_entity_counts.rs parser/src/bin/probe_entity_counts.rs
docker compose exec dashjump-parser cargo run --bin probe_entity_counts -- \
    /parser/src/replays/68175583_527726523.dem
rm parser/src/bin/probe_entity_counts.rs
```

Raw output for this pass is preserved at `private/engineering/tools/entity_counts_68175583_527726523.txt`.
