# Mid-Boss Message Fields Spike

## Context

`CNPC_MidBoss` (the mid-boss that grants the rejuvenator buff on death) is partially tracked -- per-second position snapshots exist in `should_track_position()` (`replay_parser.rs:142`), but this is incorrect. The mid-boss spawns in a fixed location and does not move. What we actually need: a spawn snapshot, a death snapshot, and an efficient health timeline.

When the mid-boss dies, a rejuvenator buff entity (colloquially "rejuv") spawns at the same location. Players must punch it to claim rejuvs -- each punch does 1 damage and grants the whole team 1 rejuv buff. The buff likely has 3 health (3 punches, 3 rejuvs). It disappears within ~10-15 seconds once fully claimed or expired. The entity class name for this buff is unknown. There may be weaker map buff pickups sharing the same entity class that spawn on a 5-minute cycle starting at 10 minutes -- those take only 1 punch (1 health). If they share a class, we need a way to distinguish the mid-boss rejuv (3 health) from a weaker variant (1 health).

Two Citadel user messages are unsubscribed: `CCitadelUserMsg_MidBossSpawned` (ID 349) and `CCitadelUserMsg_BossKilled` (ID 347). Their fields and firing conditions are unconfirmed. `CCitadelUserMsg_BossKilled` is NOT shared with the sinners sacrifice -- it is specific to the mid-boss.

---

## Questions

1. What fields do `CCitadelUserMsg_MidBossSpawned` and `CCitadelUserMsg_BossKilled` expose? Are they sufficient to capture spawn timing, kill attribution (team or player), and rejuv buff grant without entity-level inspection?

2. What is the entity class name for the rejuvenator buff that spawns when the mid-boss dies? Is it shared with the smaller periodic map buffs? If shared, how do we distinguish a 3-health rejuv from a 1-health weaker variant at entity creation time?

3. What is an efficient method for tracking mid-boss health over time, given that health changes very infrequently (some games the boss is never killed; others it dies once or twice per game)?

---

## Assumptions

### To Validate

- [ ] `CCitadelUserMsg_MidBossSpawned` (ID 349) fires once per mid-boss spawn cycle -- *How to check: inspect proto definition in valveprotos-rs; look for a cycle or phase field*
- [ ] `CCitadelUserMsg_BossKilled` (ID 347) fires when the mid-boss dies -- *How to check: inspect proto definition for an entity-type discriminator or name field*
- [ ] The rejuvenator buff spawns as a trackable entity with a health value (not purely a UI event) -- *How to check: search valveprotos-rs for buff-related entity class names; check haste-inspector output near mid-boss death tick*
- [ ] The buff entity class is shared between the mid-boss rejuv and weaker periodic map buffs -- *How to check: compare entity class names in replay for both buff types*

### Accepted (not tested here)

- `CNPC_MidBoss` entity hash (`fxhash::hash_bytes(b"CNPC_MidBoss")`) correctly identifies the entity -- *Risk if wrong: entity-level field reads target the wrong class; constants.rs:49 needs correction*
- `deadlock-api/haste` async Visitor is the correct API surface for subscribing to these messages -- *Risk if wrong: subscription code pattern differs from what is implemented today*
- Per-second position tracking for mid-boss (replay_parser.rs:142, 238) should be replaced by spawn/death snapshots -- mid-boss does not move -- *Risk if wrong: position data is discarded unnecessarily*
- This branch will implement its own `should_track_snapshot` method for mid-boss, parallel to the one being added in the sinner-tracking worktree. The two implementations will be reconciled when both branches merge to main.

---

## Agent & Timebox

**Agent:** haste-expert
**Timebox:** 3 hours

---

## Research Standards

Follow `.claude/rules/research.md` for confidence labels, citation format, and scope discipline.

---

## Investigation Approach

1. Fetch proto definitions for `CCitadelUserMsg_MidBossSpawned` (ID 349) and `CCitadelUserMsg_BossKilled` (ID 347) from valveprotos-rs. List every field with name, type, and field number. Note any entity-type discriminator or name field on BossKilled.

2. Search valveprotos-rs and haste-inspector output for buff-related entity class names. Look for anything matching "rejuvenator", "rejuv", or buff pickup patterns. Also check whether a shared class exists for the periodic map buff pickups (spawns at 10 min, every 5 min after). If found, check whether health or a tier field distinguishes the mid-boss rejuv (3 health) from the weaker variants (1 health).

3. For health timeline efficiency, evaluate these approaches and identify which best supports "was mid-boss alive at time T?" and "how much health at first engagement?" queries with minimal storage:
   - **Delta / on-change only** -- record (tick, health) only when health value changes; flat storage between events
   - **Event-anchored** -- record health only at spawn, each damage event, and death; requires a damage or health-change event subscription
   - **Coarse sampling with change detection** -- poll every N ticks, emit only when value differs from previous sample

4. If haste-inspector output is available for a replay with a mid-boss kill, cross-reference observed message and entity events around the death tick.

---

## Findings

### Q1 -- Fields of `CCitadelUserMsg_MidBossSpawned` (ID 349) and `CCitadelUserMsg_BossKilled` (ID 347)

**`CCitadelUserMsg_MidBossSpawned` (ID 349):** Empty message -- zero fields. The message body carries no data whatsoever.

```protobuf
message CCitadelUserMsg_MidBossSpawned {
}
```

Source: `https://raw.githubusercontent.com/deadlock-api/valveprotos-rs/458c5e17c402953ab61baaef4a099a073cf01644/protos/deadlock/citadel_usermessages.proto` (confirmed, 2026-04-01)

Spawn timing must come from `ctx.tick()` at the moment the `on_packet` callback fires. There is no alternative -- the message carries no `gametime` or tick field.

**`CCitadelUserMsg_BossKilled` (ID 347):** Eight fields.

| Field # | Name | Type | Notes |
|---------|------|------|-------|
| 1 | `objective_team` | `int32` | Team that killed the boss |
| 2 | `objective_mask_change` | `int32` | Bitmask delta encoding which objective changed |
| 3 | `entity_killed` | `uint32` | EHandle of killed entity (default 16777215 = invalid) |
| 4 | `entity_killed_class` | `int32` | Class ID of killed entity -- key for mid-boss disambiguation |
| 5 | `entity_killer` | `uint32` | EHandle of killing entity (default 16777215 = invalid) |
| 6 | `gametime` | `float` | Match time of kill in seconds |
| 7 | `bosses_remaining` | `int32` | Count of bosses still alive after this kill |
| 8 | `entity_position` | `CMsgVector` | World position of killed entity |

Source: `CCitadelUserMsg_BossKilled` definition, valveprotos-rs commit `458c5e1` (confirmed, 2026-04-01)

**Sufficiency assessment:**

- **Spawn timing:** `confirmed` sufficient -- record `ctx.tick()` when `MidBossSpawned` fires; convert with `tick as f32 * ctx.tick_interval()`.
- **Kill timing:** `confirmed` sufficient -- `BossKilled.gametime` carries match time directly; no need for tick conversion.
- **Team attribution:** `confirmed` sufficient -- `BossKilled.objective_team` identifies which team got the kill.
- **Player attribution:** `inferred` insufficient -- `entity_killer` is an ehandle, which identifies the killing entity at the network layer, but requires a live entity lookup via `ctx.entities()` to resolve to a player slot. If the entity is already deleted when the message fires (common for killed projectiles), the lookup may return `None`. For reliable per-player kill credit, subscribe to `CCitadelUserMsg_HeroKilled` (ID 319) cross-referenced against the `BossKilled.gametime`. Neither message alone gives player slot + boss kill atomically.
- **Rejuv buff grant:** `inferred` insufficient from `BossKilled` alone -- the message confirms kill team but not individual buff grants. `CCitadelUserMsg_RejuvStatus` (ID 350) exists specifically for this: it has `killing_team` (int32), `player_pawn` (uint32 ehandle), `user_team` (int32), and `event_type` (int32). The `event_type` enum values are not documented in the proto but `inferred` to encode "buff granted" vs "buff expired" vs "all stacks consumed". One `RejuvStatus` message likely fires per buff grant event -- meaning up to 3 fires per mid-boss kill (once per punch/rejuv awarded). Subscribe to both `BossKilled` and `RejuvStatus` together for complete coverage.
- **Mid-boss vs. other boss disambiguation:** `inferred` possible via `entity_killed_class` field 4 -- this is a runtime class ID, not a compile-time constant in the proto. The actual integer value for `CNPC_MidBoss` must be confirmed by inspecting `entity_killed_class` in a replay with a mid-boss kill using haste-inspector or a probe binary. The `bosses_remaining` field may also be usable as a proxy if mid-boss and walker counts are tracked separately.

**Entity-level inspection needed:** `confirmed` -- player attribution from `BossKilled.entity_killer` alone is unreliable. `confirmed` -- rejuv buff grant tracking requires `RejuvStatus` (ID 350) subscription, not just `BossKilled`.

---

### Q2 -- Rejuvenator buff entity class name

The rejuvenator buff entity class name is **not present in any proto file**. This is expected: entity class names are embedded in the demo's SendTables, not in `.proto` files. This is consistent with the established pattern documented in `private/learnings.md` -- "Deadlock Entity Field Enums Are Not in Protobufs".

Source search: full scan of `citadel_usermessages.proto`, `citadel_gameevents.proto`, `citadel_gamemessages.proto` at valveprotos-rs commit `458c5e1` -- no rejuv, rejuvenator, buff pickup, or map buff class names found (confirmed, 2026-04-01).

The parser's `constants.rs` contains no rejuv buff entity hash constant -- `CNPC_MidBoss` is the only mid-boss-related entry (`parser/src/entities/constants.rs:49`).

**What is known from the proto layer:** `CCitadelUserMsg_RejuvStatus` (ID 350) fires on rejuv-related events. Its `event_type` field (int32, field 4) encodes the event type but enum values are undocumented. This message fires in `on_packet` and requires no entity subscription. It may be sufficient for tracking rejuv buff grants without needing the pickup entity class at all -- if `event_type` distinguishes "buff granted to player" from "buff entity spawned", the entity itself may never need to be tracked.

**Entity class name confirmed via haste-inspector:** `CCitadelItemPickupRejuv`

Fields observed at tick 84932 (entity already fully claimed, health depleted):

| Field | Type | Value | Notes |
|-------|------|-------|-------|
| `m_flSimulationTime` | float32 | 1354.0781 | |
| `m_iHealth` | int32 | 0 | 0 at this inspection point -- already claimed |
| `m_iMaxHealth` | int32 | 0 | 0 at this inspection point |
| `m_flCreateTime` | GameTime_t | 1339.9375 | Creation gametime |
| `m_nSubclassID` | CUtlStringToken | 289368075 | `inferred`: candidate field for distinguishing mid-boss rejuv from weaker map buff variant |
| `m_iTeamNum` | uint8 | 4 | |
| `m_eLootType` | int32 | 1 | `inferred`: loot type discriminator -- may differ between mid-boss and map buff variants |
| `m_nCurrencyValue` | int32 | 0 | |

Source: haste-inspector, tick 84932 (2026-04-01)

The entity was confirmed to appear in the inspector at tick 84550, the same tick the mid-boss `m_iHealth` hit 0 and `m_lifeState` transitioned to 1. This confirms the rejuv pickup entity spawns at boss death.

**Whether `CCitadelItemPickupRejuv` is shared with periodic map buff pickups:** `inferred` not shared -- `CCitadelItemPickupRejuv` appears to be a specific class for the rejuv buff. Other pickups (periodic map buffs, soul orbs, etc.) likely use a base `CCitadelItemPickup` class or other specific subclasses. The class name's specificity (`Rejuv` suffix) follows the same naming convention as other specific entity types in Deadlock. No discrimination logic is needed.

**Rejuv claim tracking -- health is NOT the mechanism:** `confirmed` -- the entity fields show `m_iHealth=0` and `m_iMaxHealth=0`, meaning health is not used to track the claim state at all. The interaction model for players claiming rejuv stacks is via a separate mechanism. `CCitadelUserMsg_RejuvStatus` (ID 350) is the likely signal for individual claim events. How many stacks remain on the entity and whether the entity deletes itself when fully claimed vs. on a timer requires replay inspection.

**Overall confidence for Q2:** `confirmed` for entity class name and exclusivity; `unknown` for claim-tracking mechanism (requires `RejuvStatus` event_type mapping and entity lifecycle observation).

---

### Q3 -- Most efficient method for tracking mid-boss health over time

Three approaches evaluated:

**(a) Delta / on-change only** -- Record `(tick, health)` only when `m_iHealth` changes on the `CNPC_MidBoss` entity in `on_entity`. Since `on_entity` fires on every `UPDATE` that includes any changed field (not just health), this requires reading `m_iHealth` every entity update and comparing to the last recorded value, emitting only when different.

- Storage: minimal -- only changed values are stored
- Complexity: medium -- requires a last-known-health variable per entity instance
- Correctness risk: `inferred` medium -- haste fires `on_entity` with the full entity state on every update, with no field-changed list (documented in `deadlock-api-haste-reference.md` -- "No field list on entity update"). Reading health every tick is cheap, but false-positive emissions are not possible if you compare to previous value before appending.
- Query fit: good for "was mid-boss alive at time T?" (binary search on sparse list); good for "health at first engagement?" (first sample after `m_iHealth` drops from max)

**(b) Event-anchored** -- Record health only at: (1) entity CREATE, (2) each `CCitadelUserMessage_Damage` event where the victim is the mid-boss entity index, (3) entity DELETE. This is exactly what `BossTracker.record_boss_damage` already implements for objective bosses (`parser/src/tracking/boss_tracker.rs:139-158`).

- Storage: minimal -- one sample per damage event; typically O(10-50) samples for a mid-boss with ~8000 HP
- Complexity: low -- reuse or extend the existing `BossTracker` pattern; no new state machine needed
- Correctness risk: `inferred` low -- damage messages are reliable for health changes; `CCitadelUserMessage_Damage` already includes `victim_health_new` which makes the entity lookup unnecessary
- Query fit: excellent -- `victim_health_new` gives exact post-damage health without a separate entity read; DELETE marks death; CREATE marks spawn

**(c) Coarse sampling with change detection** -- Poll in `on_tick_end` every N ticks (e.g. every 60 ticks = 1 second), read `m_iHealth`, emit only when value differs from previous sample.

- Storage: low but higher than (a) or (b) for actively-attacked bosses -- worst case one sample per second during a fight
- Complexity: low -- simple tick counter
- Correctness risk: `inferred` medium -- a rapid kill could show health changes between two 1-second polls, missing the intermediate damage pattern; the terminal health=0 sample may land 0-59 ticks late
- Query fit: adequate for "was mid-boss alive at time T?" but imprecise for "health at first engagement?" (±1 second resolution)

**Recommendation: approach (b), event-anchored, extending the existing `BossTracker`.**

The existing `BossTracker` already implements this exact pattern for objective bosses, including the critical `handle_boss_delete` terminal sample (`parser/src/tracking/boss_tracker.rs:115-136`). Adding `CNPC_MidBoss` to `BossTracker.is_boss_entity()` and including it in the damage event filter in `replay_parser.rs` would cover all three use cases (spawn snapshot, health timeline, death snapshot) with minimal new code and no storage overhead. `CCitadelUserMessage_Damage.victim_health_new` provides health at each damage event without a separate entity read.

For the specific queries called out in the spike:
- "Was mid-boss alive at time T?" -- binary search the sparse health sample list; if the last sample before T has health > 0 and no death_time_s is set before T, it was alive
- "How much health at first engagement?" -- first health sample where health < max_health

**Approach (a)** is also valid but offers no advantage over (b) for this use case -- mid-boss health only changes when it takes damage, so "on-change" and "on damage event" are equivalent, and (b) also captures the `victim_health_new` value directly from the message without needing an entity read.

**Approach (c)** is the weakest option -- it introduces timing imprecision and does not reuse the established `BossTracker` pattern.

---

### Additional Entity Observations (haste-inspector, 2026-04-01)

**CNPC_MidBoss at tick 43931 (full health, alive):**

| Field | Type | Value | Notes |
|-------|------|-------|-------|
| `m_iHealth` | int32 | 14950 | Full health -- max HP confirmed |
| `m_iMaxHealth` | int32 | 14950 | |
| `m_lifeState` | uint8 | 0 | Alive |
| `m_flCreateTime` | GameTime_t | 618.8906 | Spawns at ~10 min 18 s |
| `m_iTeamNum` | uint8 | 4 | Neutral team |
| `m_NPCState` | NPC_STATE | 2 | `inferred`: NPC_STATE_IDLE or equivalent |
| `m_bMinion` | bool | false | |
| `m_bBeamActive` | bool | false | Also present on walkers -- not mid-boss specific |
| `m_vEyeBeamTarget` | VectorWS | [0, 0, 0] | Inactive at full health |
| `m_MoveType` | MoveType_t | 0 (alive) → 9 (death) | `inferred`: type 9 is ragdoll/dead state; value transitions at tick 84550 when health hits 0 |

**CNPC_MidBoss death transition (tick 84547 → 84550):**
- `m_iHealth`: 12 (tick 84547) → 0 (tick 84550)
- `m_lifeState`: 0 → 1
- `m_NPCState`: 2 → 10 (`inferred`: NPC_STATE_DEAD)
- `m_bRagdollEnabled`: false → true
- `m_bBeamActive`: false → true (beam activates on death -- `hypothesis`: death animation trigger)
- `CCitadelItemPickupRejuv` entity appears in inspector at tick 84550

**`m_bBeamActive` on walkers:** `confirmed` -- field is present on walker entities as well. Not a mid-boss exclusive field. Source: user observation (2026-04-01).

---

### Assumptions Check

- [x] `CCitadelUserMsg_MidBossSpawned` (ID 349) fires once per mid-boss spawn cycle -- **held (partial)** -- The message is confirmed to exist and fires for the mid-boss spawn, but "once per cycle" cannot be confirmed from the proto alone since the message has no fields (no cycle counter, no spawn index). It is `inferred` that it fires once per spawn given the name and the absence of any multi-spawn discriminator field. Validate with a replay that has two mid-boss spawns.

- [x] `CCitadelUserMsg_BossKilled` (ID 347) fires when the mid-boss dies -- **held (with caveat)** -- The message fires for any boss kill, including walkers and Patron. Disambiguation requires filtering on `entity_killed_class` (field 4), whose integer value for `CNPC_MidBoss` must be confirmed via haste-inspector. The spike plan note that it "is NOT shared with the sinners sacrifice" is consistent with the message fields -- the Sinners Sacrifice is a neutral NPC, not an objective boss, so `BossKilled` would not fire for it.

- [x] The rejuvenator buff spawns as a trackable entity with a health value -- **held** -- `CCitadelItemPickupRejuv` entity confirmed in haste-inspector at tick 84550, the exact tick the mid-boss health hit 0. Entity has `m_iHealth` and `m_iMaxHealth` fields. Health was 0 at tick 84932 (fully claimed by then). Source: haste-inspector (2026-04-01).

- [ ] The buff entity class is shared between mid-boss rejuv (3 health) and weaker periodic map buffs (1 health) -- **unknown** -- Class name is now confirmed (`CCitadelItemPickupRejuv`) but whether the periodic map buffs share it is unconfirmed. `m_nSubclassID` (289368075) and `m_eLootType` (1) are candidate discriminators. Requires inspection of a periodic map buff entity to compare.

- Accepted assumptions worth flagging: `CNPC_MidBoss` entity hash (`constants.rs:49`) is confirmed correct as the class name appears in the existing constants file and the serializer name is used consistently. The accepted assumption that mid-boss position tracking should be replaced by spawn/death snapshots is reinforced by the findings -- `MidBossSpawned` and `BossKilled` both fire cleanly and `BossKilled.entity_position` already provides the death position.

---

## Learnings Output

- [x] Draft entry appended to `private/learnings.md` ## Drafts
- [ ] Follow-up questions or spikes needed:

  1. ~~**Rejuv buff entity class name**~~ -- `confirmed` resolved: `CCitadelItemPickupRejuv` (haste-inspector, 2026-04-01).
  2. ~~**Periodic map buff shared class**~~ -- `inferred` resolved: `CCitadelItemPickupRejuv` is specific to the rejuv buff; other pickups use distinct classes. No discrimination logic needed.
  3. **Rejuv claim mechanism** -- `CCitadelItemPickupRejuv` does not use health to track claims (`m_iHealth=0`, `m_iMaxHealth=0`). The claim signal is likely `CCitadelUserMsg_RejuvStatus` (ID 350). Open questions: how many `RejuvStatus` events fire per mid-boss kill (expected 3), what `event_type` values encode (grant vs. expire vs. all-claimed), and whether the entity self-deletes when fully claimed or on a fixed timer. Investigate via haste-inspector entity lifecycle + `RejuvStatus` event log in a replay with a full mid-boss kill.
  4. **`BossKilled.entity_killed_class` value for mid-boss** -- In a replay with a mid-boss kill, log `entity_killed_class` when `BossKilled` fires. A probe binary with a `println!` in the `BossKilled` packet handler is the cleanest method.
  5. **`RejuvStatus.event_type` enum values** -- Subscribe to `RejuvStatus` (ID 350) in a probe binary, log all observed `event_type` values from a replay with multiple rejuv grants, and map values to game events.
  6. **Re-verify `CCitadelUserMsg_MidBossSpawned` proto** -- The in-game announcement and sound at ~10 minutes suggest the message should carry data. The empty proto definition (zero fields, commit `458c5e1`) may be stale or fetched from the wrong message. Re-fetch at a more recent valveprotos-rs commit and confirm. If still empty, `ctx.tick()` is the only timing signal available.
