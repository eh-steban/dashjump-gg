# Entity Fields Supplemental
**Last Updated:** 2026-03-17
**Purpose:** Background-context entity fields — informative but not load-bearing for current dashjump features. Agents working on active features should load `entity-fields-reference.md` instead. Load this file only when researching low-level engine behavior or platform/physics mechanics.

---

## Source Verification Key

| Label | URL |
|-------|-----|
| `[deadlock-CBaseEntity]` | https://raw.githubusercontent.com/SteamDatabase/GameTracking-Deadlock/master/DumpSource2/schemas/server/CBaseEntity.h |
| `[deadlock-EntityPlatformTypes_t]` | https://raw.githubusercontent.com/SteamDatabase/GameTracking-Deadlock/master/DumpSource2/schemas/client/EntityPlatformTypes_t.h |
| `[deadlock-MoveType_t]` | https://raw.githubusercontent.com/SteamDatabase/GameTracking-Deadlock/master/DumpSource2/schemas/client/MoveType_t.h |
| `[deadlock-api-gcmessages-common]` | https://raw.githubusercontent.com/deadlock-api/valveprotos-rs/9625c0784beca10634442ef11ede5f022ab186da/protos/deadlock/citadel_gcmessages_common.proto |

---

## m_nPlatformType: uint8

**Source:** `[deadlock-CBaseEntity]` (field declaration with `MNetworkEnable`); `[deadlock-EntityPlatformTypes_t]` (enum definition, underlying type `uint8_t`)
**Applies to:** `CBaseEntity` -- present on all entities in the game world, including `CCitadelPlayerPawn`, `CNPC_Trooper`, objectives, and world geometry entities.

Describes whether an entity acts as a moving platform that other entities stand on. When non-zero, the engine makes passengers follow the platform's movement. In Deadlock this is relevant for ziplines, moving platforms, and any surface that carries riders.

### Values / Semantics

| Value | Constant | Meaning |
|-------|----------|---------|
| `0` | `ENTITY_NOT_PLATFORM` | Normal entity, does not carry passengers |
| `1` | `ENTITY_PLATFORM_PLAYER_FOLLOWS_YAW` | Moving platform; passengers rotate with the platform's yaw |
| `2` | `ENTITY_PLATFORM_PLAYER_IGNORES_YAW` | Moving platform; passengers translate but do not rotate with yaw |

### Gotchas

**Not useful for creep or hero state features.** `m_nPlatformType` is a property of the platform entity itself, not of the entity riding it. A hero standing on a zipline has `m_nPlatformType = 0` (they are not a platform). To detect that a hero is currently being carried by a platform, you would need to track the platform entity and check whether the hero's position matches it -- there is no rider-side flag.

**Nearly always 0 in entity snapshots.** The overwhelming majority of entities in a Deadlock match carry value `0`. Do not iterate this field looking for interesting state changes -- it is set at map load for static platform entities and does not change during play.

**uint8 underlying type, read as `u8`.** Although `CBaseEntity` fields often decode as `u64` in haste (the fallback for unrecognized type names), the declared underlying type is `uint8_t`. If `entity.get_value::<u64>(&key)` returns an unexpectedly large value, fall back to `u8` read.

### Correct Usage Pattern

```rust
// Read only if explicitly needed (e.g. detecting zipline platform entities at map load)
const PLATFORM_TYPE_KEY: u64 = fkey_from_path(&["m_nPlatformType"]);
let platform_type: u8 = entity.get_value(&PLATFORM_TYPE_KEY).unwrap_or(0);
// 0 = not a platform, 1/2 = platform entity
```

---

## m_MoveType: uint8

**Source:** `[deadlock-CBaseEntity]` (field declaration with `MNetworkEnable`, also declares `m_nActualMoveType` without `MNetworkEnable`); `[deadlock-MoveType_t]` (enum definition, underlying type `uint8_t`)
**Applies to:** `CBaseEntity` -- present on all entities. Particularly meaningful on `CCitadelPlayerPawn` (live movement mode changes). For `CNPC_Trooper`, confirmed always 9 (`MOVETYPE_STEP`) across all lifecycle phases.

The networked movement physics mode of an entity. Controls which movement subsystem the engine uses for this entity's physics simulation. For hero pawns the value transitions as heroes enter/exit different states (walking, flying on ziplines, noclip in spectator, etc.).

### Confirmed CNPC_Trooper Behavior (Validated in Haste-Inspector)

**`m_MoveType` is always 9 (`MOVETYPE_STEP`) for `CNPC_Trooper` across ALL lifecycle phases, including cage/zipline travel.**

This was validated empirically by manually inspecting `m_MoveType` for many `CNPC_Trooper` entities across many ticks in two different Deadlock demos using haste-inspector. The field never returned 0 (`MOVETYPE_NONE`) at any point -- not before cage launch, not during zipline travel, and not during in-lane walking. The value was consistently 9 across all observations.

**Consequence:** `m_MoveType` cannot be used to discriminate cage-phase creeps from in-lane walking creeps. The hypothesis that `MOVETYPE_NONE (0)` would appear during cage travel is disproven. See `entity-fields-reference.md` -- `CNPC_Trooper Zipline/Cage Phase vs. In-Lane Phase Discrimination` for the current state of the investigation and recommended alternative approaches.

> **Open question:** See `CMsgMatchPlayerPathsData.EMoveType` section below. The GC post-match message uses a different, higher-level movement classification (GroundDash, Ziplining, InAir, etc.) under the same field name. It is not yet confirmed whether the GC message values are a server-side translation of `m_MoveType` entity values, or a separate classification. To verify: sample `m_MoveType` from a replay entity at a known zipline timestamp and compare to the `move_type` field value in `CMsgMatchPlayerPathsData.Path` for that same player and time.

### Values / Semantics — entity network field

| Value | Constant | Meaning |
|-------|----------|---------|
| `0` | `MOVETYPE_NONE` | No movement simulation; entity is static |
| `1` | `MOVETYPE_OBSOLETE` | Legacy value; should not appear in live entities |
| `2` | `MOVETYPE_WALK` | Standard ground movement with gravity |
| `3` | `MOVETYPE_FLY` | Fly without gravity |
| `4` | `MOVETYPE_FLYGRAVITY` | Fly with gravity (e.g. projectiles) |
| `5` | `MOVETYPE_VPHYSICS` | Physics simulation via VPhysics (ragdolls, physics props) |
| `6` | `MOVETYPE_PUSH` | Engine-pushed entity (doors, platforms) |
| `7` | `MOVETYPE_NOCLIP` | No collision, free movement (spectator/dev) |
| `8` | `MOVETYPE_OBSERVER` | Spectator follow mode |
| `9` | `MOVETYPE_STEP` | Step-based NPC movement |
| `10` | `MOVETYPE_SYNC` | Movement synchronized to another entity |
| `11` | `MOVETYPE_CUSTOM` | Game-specific custom movement |
| `12` | `MOVETYPE_LAST` / `MOVETYPE_INVALID` | Sentinel / invalid |

Source: `[deadlock-MoveType_t]` — `GameTracking-Deadlock/DumpSource2/schemas/client/MoveType_t.h`. These are the values readable from `entity.get_value::<u8>(&MOVE_TYPE_KEY)` in replay parsing.

### Values / Semantics — CMsgMatchPlayerPathsData.EMoveType (GC post-match message)

A separate, higher-level movement classification used in the `CMsgMatchPlayerPathsData` GC message. This message contains per-player path data sampled at `interval_s` intervals and includes `move_type` alongside `x_pos`, `y_pos`, and `health` per sample.

Source: `[deadlock-api-gcmessages-common]` — `deadlock-api/valveprotos-rs @ 9625c07`, `protos/deadlock/citadel_gcmessages_common.proto`, line 364.

| Value | Constant | Meaning |
|-------|----------|---------|
| `0` | `k_eMoveType_Normal` | On-foot, normal movement |
| `1` | `k_eMoveType_Ability` | Movement altered by an ability (buff) |
| `2` | `k_eMoveType_AbilityDebuff` | Movement altered by an ability (debuff) |
| `3` | `k_eMoveType_GroundDash` | Ground dash |
| `4` | `k_eMoveType_Slide` | Slide |
| `5` | `k_eMoveType_RopeClimbing` | Climbing a rope |
| `6` | `k_eMoveType_Ziplining` | On a zipline |
| `7` | `k_eMoveType_InAir` | Airborne (jump, falling) |
| `8` | `k_eMoveType_AirDash` | Air dash |

These values are almost certainly a server-side translation of the low-level `m_MoveType` entity field into gameplay-meaningful categories. For example, `MOVETYPE_SYNC` (10) on a hero entity likely corresponds to `k_eMoveType_Ziplining` (6) in the GC message. Confirmation requires cross-referencing a demo replay against the matching `CMsgMatchPlayerPathsData` for a known zipline event.

### Gotchas

**`m_nActualMoveType` is not networked.** `CBaseEntity` declares both `m_MoveType` (with `MNetworkEnable`) and `m_nActualMoveType` (without). Demos only contain `m_MoveType`. `m_nActualMoveType` reflects the engine's internal effective move type after modifiers; it is not accessible from replay files.

**Creep death transition to VPHYSICS -- hypothesis unvalidated.** The ragdoll-to-VPHYSICS transition on death was a theoretical concern. Given that `m_MoveType` is confirmed always 9 for `CNPC_Trooper` at all observed points, this transition may not actually occur in demos (the engine may delete the entity before any VPHYSICS update is networked). The `DELETE` event remains the only reliable death signal for lane creeps. Do not use `m_MoveType` for alive/dead discrimination on creeps.

**Hero on zipline.** Heroes riding a zipline transition from `MOVETYPE_WALK` to `MOVETYPE_SYNC` (value 10) while attached. This is the current best signal for zipline detection via entity fields for `CCitadelPlayerPawn`, pending confirmation that it maps to `k_eMoveType_Ziplining` in the GC message. This behavior is for hero entities only and does not apply to `CNPC_Trooper`.

### Correct Usage Pattern

For detecting hero zipline state via move type (CCitadelPlayerPawn only):

```rust
const MOVE_TYPE_KEY: u64 = fkey_from_path(&["m_MoveType"]);
const MOVETYPE_WALK: u8 = 2;
const MOVETYPE_SYNC: u8 = 10;  // suspected: on zipline -- unconfirmed against GC message

// in on_entity for CCitadelPlayerPawn:
let move_type: u8 = entity.get_value(&MOVE_TYPE_KEY).unwrap_or(0);
let on_zipline = move_type == MOVETYPE_SYNC;
```

For CNPC_Trooper: m_MoveType is always 9 (MOVETYPE_STEP). Do not use this field to discriminate creep lifecycle phases:

```rust
// CNPC_Trooper: m_MoveType == 9 at all times (cage travel, in-lane, near death).
// Use DELETE event for death detection.
// Use position-based heuristics for cage-phase discrimination (see entity-fields-reference.md).
```
