# Citadel Messages Supplemental
**Last Updated:** 2026-03-17
**Last Verified:** 2026-04-09 (via `probe_all_messages` against 3 replays: `55423930_379917638`, `55841493_649180947`, `68175583_527726523`)
**Purpose:** Background-context message catalogs -- low product alignment, not load-bearing for current dashJump features. Agents working on active features should load `citadel-messages-reference.md` instead. Load this file only when investigating lower-level engine messaging or ballistics/visual subsystems.

---

## Verification Status

Each entry below carries a ✅/❌ status inline in its table row, reflecting whether `on_packet` ever saw the ID in the 3 verification replays.

**Supplemental CitadelUserMessageIds summary:**

| Status | IDs |
|---|---|
| ✅ VERIFIED | 303, 308, 312, 314, 317, 332, 338, 341 |
| ❌ NOT SEEN | 304, 306, 311, 315, 321, 322, 325, 326, 327, 329, 330, 331, 336, 342, 343, 344, 354, 357, 358, 359, 362, 364 |

**ECitadelGameEvents summary (IDs 450-466):**

| Status | IDs |
|---|---|
| ✅ VERIFIED | 450 `GE_FireBullets` (148,236 events -- by far the highest-volume non-sound message), 466 `GE_RemoveBullet` (57 events) |
| ❌ NOT SEEN | 451, 458, 459, 461, 462, 463, 464, 465 |

Notable: `GE_BulletImpact` (461) is documented as the richer alternative to `CCitadelUserMessage_BulletHit` (323) but **neither fires** in these replays. Bullet-level analytics currently has no verified signal. `GE_FireBullets` (450) fires but only carries projectile simulation inputs, not hit results.

**Previously undocumented IDs (now identified):** IDs 400 and 423 are `ETEProtobufIds` -- Source 2 temporary-entity (visual effect) messages from `te.proto`. They live in their own namespace, separate from `CitadelUserMessageIds` (300-366) and `ECitadelGameEvents` (450-466), and are not present in `deadlock-api/valveprotos-rs` (which omits `te.proto`); they were located in the `SteamDatabase/GameTracking-Deadlock` mirror. See the "Temporary Entities" section at the bottom of this file.

---

## Low-Alignment UserMessages (citadel_usermessages.proto)

**Source:** `protos/deadlock/citadel_usermessages.proto`
**Commit analyzed:** `9625c0784beca10634442ef11ede5f022ab186da`

Messages from `CitadelUserMessageIds` (IDs 300-366) with no current product alignment. Documented here for completeness. For aligned messages see `citadel-messages-reference.md`.

| Status | ID  | Message | Reason not in reference |
|:---:|-----|---------|------------------------|
| ❌ | 325 | `ModifierApplied` | Only fields are caster/parent entity indices + serial_number -- no modifier type ID. Too sparse to derive analytics value without a separate modifier registry. NOT SEEN in 2026-04-09 probe. |
| ❌ | 327 | `AuraModifierApplied` | Has `modifier_type_id` + start/end times, but modifier type IDs have no documented lookup table; would require additional reverse engineering to decode. NOT SEEN in 2026-04-09 probe. |
| ✅ | 338 | `AbilityNotify` | Victim, attacker, ability_id, and `status_impact` uint32 -- but `status_impact` enum values are not defined in the proto and carry unknown semantics. VERIFIED in 2026-04-09 probe (fires -- could be decoded now that the ID is confirmed live). |
| ❌ | 357 | `SeasonalKill` | Seasonal content mechanic -- killer + victim entity handles only. Not relevant to standard match analytics. NOT SEEN in 2026-04-09 probe. |
| ❌ | 362 | `StreetBrawlScoring` | Street Brawl game mode only (`sapphire_score`, `amber_score` per team). Does not fire in standard matches. NOT SEEN in 2026-04-09 probe (consistent with replays being standard mode). |
| ❌ | 304 | `TeamRewards` | Team-level XP and gold at match end (`xp`, `gold`, `winner`) -- no team number in the message itself; timing comes from `ctx.tick()`. NOT SEEN in 2026-04-09 probe -- the end-of-match summary category is entirely absent. |

---

## Game Events: ECitadelGameEvents

**Source:** `protos/deadlock/citadel_gameevents.proto`
**Commit analyzed:** `9625c0784beca10634442ef11ede5f022ab186da`

### Subscription Mechanism

Game events use **the same `on_packet` callback** as Citadel user messages. The `ECitadelGameEvents` enum assigns IDs in the 450–466 range, which is a separate numeric namespace from `CitadelUserMessageIds` (300–366) but arrives through the same dispatch path.

Match `packet_type` against the `ECitadelGameEvents` variant cast to `u32`:

```rust
// ECitadelGameEvents::GE_BulletImpact = 461
if packet_type == 461u32 {
    let msg = CMsgBulletImpact::decode(data)?;
}
```

There is **no separate `on_game_event` callback** in haste. All unrecognized sub-message types fall through to `visitor.on_packet()` via the catch-all branch in `parser.rs`.

### ECitadelGameEvents Enum

ECitadelGameEvents is NOT present in the generated Rust enum at the current valveprotos-rs rev (`9625c07`); the IDs below are verified by manual proto inspection + the 2026-04-09 `probe_all_messages` run.

| Status | ID  | Enum Variant                    | Probe count (3 replays) | Product Alignment |
|:---:|-----|---------------------------------|-------------------------|-------------------|
| ✅ | 450 | GE_FireBullets                  | ~148k                   | None              |
| ❌ | 451 | GE_PlayerAnimEvent              | 0                       | None              |
| ❌ | 458 | GE_ParticleSystemManager        | 0                       | None              |
| ❌ | 459 | GE_ScreenTextPretty             | 0                       | None (debug only) |
| ❌ | 461 | GE_BulletImpact                 | 0                       | Low (spec only)   |
| ❌ | 462 | GE_EnableSatVolumesEvent        | 0                       | None              |
| ❌ | 463 | GE_PlaceSatVolumeEvent          | 0                       | None              |
| ❌ | 464 | GE_DisableSatVolumesEvent       | 0                       | None              |
| ❌ | 465 | GE_RemoveSatVolumeEvent         | 0                       | None              |
| ✅ | 466 | GE_RemoveBullet                 | 57                      | None              |

**Key finding:** `GE_BulletImpact` (461) is documented upstream as the richer bullet-hit signal vs. `CCitadelUserMessage_BulletHit` (323) -- but **neither fires** in the 2026-04-09 probe. If a bullet-level feature is ever needed, first confirm at least one of {323, 461} fires in newer replays.

### Message Detail: CMsgBulletImpact (ID: 461)

**Category:** GameEvent
**Fields:**
- `trace_start: CMsgVector` -- bullet origin position
- `impact_origin: CMsgVector` -- world position of impact
- `surface_normal: CMsgVector` -- surface normal at impact point
- `damage: uint32` -- damage dealt at this impact
- `surface_type: uint32` -- surface material type
- `ability_ehandle: uint32` -- ability entity handle (default: 16777215 = invalid)
- `impacted_ehandle: uint32` -- entity that was hit (default: 16777215 = invalid)
- `impacted_bone_index: uint32` -- bone index on impacted entity
- `weapon_subclass_id: uint32`
- `shooter_ehandle: uint32` -- entity that fired (default: 16777215 = invalid)
- `bullet_radius_override: float`

**Product Alignment:** Low. Carries bullet-level damage with positions and entity handles, but `CCitadelUserMessage_Damage` (ID 300) is strictly superior -- it includes ability ID, damage type, victim health deltas, and pre/post mitigation values. `CMsgBulletImpact` fires per-projectile and would require significant volume handling for marginal gain.
**Data Richness:** Medium raw -- damage + positions + entity handles -- but superseded by ID 300.

### Messages with No Product Alignment

- **CMsgFireBullets (450):** Bullet simulation inputs -- origin, angles, spread, seed, shooter entity. Useful for ballistics reconstruction, not analytics.
- **CMsgPlayerAnimEvent (451):** Animation system event (entity handle + event enum + data int). No analytics value.
- **CMsgParticleSystemManager (458):** Visual particle system lifecycle management (create/destroy/update). No analytics value.
- **CMsgScreenTextPretty (459):** Debug HUD text overlay with position, color, duration, font. Fragile as a game state signal.
- **CMsgEnableSatVolumesEvent (462), CMsgPlaceSatVolumeEvent (463), CMsgDisableSatVolumesEvent (464), CMsgRemoveSatVolumeEvent (465):** Saturation volume visual effects for screen-space post-processing. No analytics value.
- **CMsgRemoveBullet (466):** Bullet cleanup signal (shooter, shot_id, bullet_index). No analytics value.

---

## CitadelUserMessageIds -- No/Low Alignment (IDs 303-364)

**Source:** `protos/deadlock/citadel_usermessages.proto` from `deadlock-api/valveprotos-rs` commit `8dd8ab2216c217a14030adb3f367d9b0762e6d27`, fetched 2026-04-01

Messages cataloged below have no current product alignment. For aligned messages see `citadel-messages-reference.md`.

### Audio / Sound System (IDs 341, 342, 343, 344, 358)

All four `Participant*SoundEvent*` messages share the same structure: one sound-system sub-message + `repeated int32 player_slots`. No analytics value. Note: 340 (`ParticipantStartSoundEvent`, the companion message for 341) lives in `citadel-messages-reference.md` because it was originally cataloged there as a high-frequency sound event; all Participant* sound messages share the same low-alignment profile.

| Status | ID  | Message | Fields |
|:---:|-----|---------|--------|
| ✅ | 341 | `CCitadelUserMsg_ParticipantStopSoundEvent` | `event: CMsgSosStopSoundEvent`, `player_slots: repeated int32` |
| ❌ | 342 | `CCitadelUserMsg_ParticipantStopSoundEventHash` | `event: CMsgSosStopSoundEventHash`, `player_slots: repeated int32` |
| ❌ | 343 | `CCitadelUserMsg_ParticipantSetSoundEventParams` | `event: CMsgSosSetSoundEventParams`, `player_slots: repeated int32` |
| ❌ | 344 | `CCitadelUserMsg_ParticipantSetLibraryStackFields` | `event: CMsgSosSetLibraryStackFields`, `player_slots: repeated int32` |
| ❌ | 358 | `CCitadelUserMsg_MusicQueue` | `music_state: int32`, `override: bool` |

### Camera / Client Visual (IDs 308, 321, 326, 332, 359)

| Status | ID  | Message | Fields | Note |
|:---:|-----|---------|--------|------|
| ✅ | 308 | `CCitadelUserMsg_TriggerDamageFlash` | `entindex_flash_victim: int32`, `entindex_flash_attacker: int32`, `entindex_flash_hitgroup: int32`, `flash_value: uint32`, `flash_type: uint32`, `flash_flags: uint32`, `flash_position: CMsgVector` | Client screen-flash on damage hit; superseded by Damage (300) for analytics. Fires heavily. |
| ❌ | 321 | `CCitadelUserMsg_SetClientCameraAngles` | `player_slot: int32` (default -1), `camera_angles: CMsgQAngle` | Spectator/replay camera control. Likely only fires for live spectators, not in recorded demos. |
| ❌ | 326 | `CCitadelUserMsg_CameraController` | `action: CameraAction`, `operation: CameraOperation`, `param: CameraParam`, `param_mode: CameraParamMode`, `delay: float`, `relative_values: bool`, `context_symbol_id: uint32`, `priority: uint32` (default 1), plus nested sub-messages: `maintain`, `approach`, `spring`, `lerp`, `lag` | Camera animation system |
| ✅ | 332 | `CCitadelUserMsg_PostProcessingAnim` | `entindex_owner: int32` (default -1), `clear_all_states: bool`, `state: PostProcessingGameStates` (default `PostProcState_Killed`), `delay: float`, `fade_in_time: float`, `hold_time: float`, `fade_out_time: float`, `scale: float` | Full-screen post-processing visual effect (kill-flash bloom etc.). Verified; correlates with kills. |
| ❌ | 359 | `CCitadelUserMsg_AG2ParamTrigger` | `param_id: string`, `param_value: string` | Valve Animation Graph 2 client-side animation trigger |

### Communication / Pings (IDs 303, 312, 314, 315, 331, 364)

| Status | ID  | Message | Fields | Note |
|:---:|-----|---------|--------|------|
| ✅ | 303 | `CCitadelUserMsg_MapPing` | `ping_data: PingCommonData` (ping_message_id, ping_location, entity_index, sender_player_slot, speech_concept, response_chosen, cooldown_time), `event_type: uint32`, `ping_marker_and_sound_info: ChatMsgPingMarkerInfo`, `pinged_enemy_entity: bool`, `pinged_entity_class: uint32`, `is_minimap_ping: bool`, `pinged_hero_name: string`, `is_blind_ping: bool` | Map ping with sender slot and location; low priority unless communication-behavior feature is planned |
| ✅ | 312 | `CCitadelUserMsg_ChatWheel` | `chat_message_id: uint32`, `player_slot: int32`, `pawn_entindex: int32`, `account_id: uint32`, `hero_id: uint32`, `param_1: string`, `lane_color: CMsgLaneColor` | Chat wheel selection with lane and hero context |
| ✅ | 314 | `CCitadelUserMsg_ChatMsg` | `player_slot: int32`, `text: string`, `all_chat: bool`, `lane_color: CMsgLaneColor` | Raw in-game chat text. **PII-sensitive** -- see error-handling rules before logging. |
| ❌ | 315 | `CCitadelUserMsg_QuickResponse` | `ping_data: PingCommonData`, `responding_to_ping_message_id: uint32`, `responding_to_player_slot: int32`, `lane_color: CMsgLaneColor` | Ping acknowledgement |
| ❌ | 331 | `CCitadelUserMsg_AbilityPing` | `ping_data: PingCommonData`, `ability_id: uint32`, `ability_cooldown: float`, `ping_marker_and_sound_info: ChatMsgPingMarkerInfo` | Ability ping with cooldown remaining at ping time |
| ❌ | 364 | `CCitadelUserMsg_ItemDraftReaction` | `ping_data: PingCommonData`, `rare: bool`, `legendary: bool` | Reaction ping to a rare/legendary item drop |

### Spectator / UI (IDs 311, 317, 322, 330, 336, 354)

| Status | ID  | Message | Fields | Note |
|:---:|-----|---------|--------|------|
| ❌ | 311 | `CCitadelUserMsg_SpectatorTeamChanged` | `teamnumber: int32` | Spectator team switch; no replay analytics value |
| ✅ | 317 | `CCitadelUserMsg_ChatEvent` | `type: ECitadelChatMessage`, `values: repeated uint32`, `player_slots: repeated int32` | Pause/unpause system events; `PREGAME_COUNTDOWN=11` could anchor game-start tick precisely. **Verified -- actionable for precise match-start detection.** |
| ❌ | 322 | `CCitadelUserMsg_MapLine` | `sender_player_slot: int32` (default -1), `mapline: CMsgMapLine` | In-game map drawing tool |
| ❌ | 330 | `CCitadelUserMsg_AbilityLateFailure` | `entindex_caster: int32` (default -1), `entindex_ability: int32` (default -1), `failure_type: uint32` | Cast started but failed mid-execution; `failure_type` values not enumerated in proto |
| ❌ | 336 | `CCitadelUserMsg_ForceShopClosed` | (empty message) | Server signal to close shop UI |
| ❌ | 354 | `CCitadelUserMsg_CallCheaterVote` | `player_slot: int32` (default -1) | In-game cheater report vote |

### Empty / Dead Messages (IDs 306, 329)

| Status | ID  | Message | Note |
|:---:|-----|---------|------|
| ❌ | 306 | (no proto definition) | Enum slot `k_EUserMsg_AbilityFailed = 306` exists but has no corresponding message definition in the proto. Cannot be subscribed to or decoded. NOT SEEN in probe -- consistent with it being a dead enum slot. |
| ❌ | 329 | `CCitadelUserMsg_ObstructedShotFired` | Empty message body -- no fields. Likely a client audio/visual cue for a shot blocked by terrain. NOT SEEN in 2026-04-09 probe. |

---

## Temporary Entities (te.proto -- ETEProtobufIds)

**Source:** `Protobufs/te.proto` from `SteamDatabase/GameTracking-Deadlock` (mirror of decompiled Deadlock binaries). **Not present in `deadlock-api/valveprotos-rs`** -- that fork omits `te.proto` entirely, which is why `ETEProtobufIds` does not appear in the generated Rust enums under `parser/target/.../deadlock.rs` and these IDs surfaced as "unknown" in earlier probes.

**Namespace layout:** `ETEProtobufIds` occupies the 400-449 range, sandwiched between `CitadelUserMessageIds` (300-366) and `ECitadelGameEvents` (450-466). All three namespaces dispatch through the same `on_packet` callback in haste.

**What temporary entities are:** Source 2's "TE" messages are short-lived, fire-and-forget engine events used for visual / audio / physics effects (impacts, decals, sparks, particle dispatches, breakable debris). They carry no persistent state and are not analytics-grade signals on their own -- they trigger client-side rendering. None of these have product alignment for current dashJump features; they are documented here so the IDs are no longer "unknown" in probe output.

### ETEProtobufIds (full enum)

Full enum from `te.proto` for context. The two IDs verified live in our 2026-04-09 probe (400, 423) are marked; the rest are listed for namespace completeness but have not been observed in our replay set.

| Status | ID  | Enum Variant         | Probe count (3 replays) |
|:---:|-----|----------------------|-------------------------|
| ✅ | 400 | `TE_EffectDispatchId` | ~1,450 (~480/match) |
| ❓ | 401 | `TE_ArmorRicochetId`  | not seen / not probed |
| ❓ | 402 | `TE_BeamEntPointId`   | not seen / not probed |
| ❓ | 403 | `TE_BeamEntsId`       | not seen / not probed |
| ❓ | 404 | `TE_BeamPointsId`     | not seen / not probed |
| ❓ | 405 | `TE_BeamRingId`       | not seen / not probed |
| ❓ | 408 | `TE_BubblesId`        | not seen / not probed |
| ❓ | 409 | `TE_BubbleTrailId`    | not seen / not probed |
| ❓ | 410 | `TE_DecalId`          | not seen / not probed |
| ❓ | 411 | `TE_WorldDecalId`     | not seen / not probed |
| ❓ | 412 | `TE_EnergySplashId`   | not seen / not probed |
| ❓ | 413 | `TE_FizzId`           | not seen / not probed |
| ❓ | 414 | `TE_ShatterSurfaceId` | not seen / not probed |
| ❓ | 415 | `TE_GlowSpriteId`     | not seen / not probed |
| ❓ | 416 | `TE_ImpactId`         | not seen / not probed |
| ❓ | 417 | `TE_MuzzleFlashId`    | not seen / not probed |
| ❓ | 418 | `TE_BloodStreamId`    | not seen / not probed |
| ❓ | 419 | `TE_ExplosionId`      | not seen / not probed |
| ❓ | 420 | `TE_DustId`           | not seen / not probed |
| ❓ | 421 | `TE_LargeFunnelId`    | not seen / not probed |
| ❓ | 422 | `TE_SparksId`         | not seen / not probed |
| ✅ | 423 | `TE_PhysicsPropId`    | 1 (in 1 of 3 replays) |
| ❓ | 426 | `TE_SmokeId`          | not seen / not probed |

The probe only logs IDs it actually observes via `on_packet`, so the ❓ rows are "not seen in 3 replays" but were not specifically targeted. A higher-volume probe across more replays would be needed to confirm which other TE messages fire in normal Deadlock matches.

### Message Detail: CMsgTEEffectDispatch (ID 400)

**Category:** TempEntity (visual / audio dispatch)
**Wire ID:** 400 (`TE_EffectDispatchId`)
**Volume:** Highest-volume TE message observed -- ~480/match, roughly one every ~5 match seconds. Steady cadence throughout the match consistent with a generic "fire a particle/sound effect at this location" broadcast, not a sporadic event.

**Fields:**
- `effectdata: CMsgEffectData` -- single nested message carrying the effect parameters (see below).

**Nested type: `CMsgEffectData`** (the only field, but very wide -- this is Source 2's universal effect-dispatch payload):

| Field             | Type         | Default     | Notes                                                       |
|-------------------|--------------|-------------|-------------------------------------------------------------|
| `origin`          | `CMsgVector` | --          | World position the effect is dispatched at                  |
| `start`           | `CMsgVector` | --          | Start position (e.g. for beam/trail effects)                |
| `normal`          | `CMsgVector` | --          | Surface normal at impact point                              |
| `angles`          | `CMsgQAngle` | --          | Orientation of the effect                                   |
| `entity`          | `fixed32`    | `16777215`  | Primary entity handle (invalid = no entity)                 |
| `otherentity`     | `fixed32`    | `16777215`  | Secondary entity handle (e.g. beam target)                  |
| `scale`           | `float`      | --          | Effect scale multiplier                                     |
| `magnitude`       | `float`      | --          | Effect intensity / magnitude                                |
| `radius`          | `float`      | --          | Effect radius                                               |
| `surfaceprop`     | `fixed32`    | --          | Surface property hash (material lookup)                     |
| `effectindex`     | `fixed64`    | --          | Effect resource index                                       |
| `damagetype`      | `uint32`     | --          | Damage type bitfield (engine-level, not Citadel damage)     |
| `material`        | `uint32`     | --          | Material id                                                 |
| `hitbox`          | `uint32`     | --          | Hitbox index on `entity`                                    |
| `color`           | `uint32`     | --          | Packed RGBA tint                                            |
| `flags`           | `uint32`     | --          | Effect flags bitfield                                       |
| `attachmentindex` | `int32`      | --          | Attachment point index on `entity`                          |
| `effectname`      | `uint32`     | --          | Effect name hash                                            |
| `attachmentname`  | `uint32`     | `0`         | Attachment name hash                                        |

**Product Alignment:** None. This is Source 2's generic "play an effect here" pipeline -- it carries no game-state semantics on its own. The effect identity is hashed (`effectname`, `effectindex`, `surfaceprop`) and not directly decodable without a parallel asset registry. Even if the hashes were resolvable, the same data is reachable via richer Citadel-namespace messages (`Damage` 300, `BulletHit` 323, etc.) when relevant. **Do not subscribe** unless reverse-engineering Source 2 effect dispatch is explicitly the goal.

### Message Detail: CMsgTEPhysicsProp (ID 423)

**Category:** TempEntity (physics / breakable debris)
**Wire ID:** 423 (`TE_PhysicsPropId`)
**Volume:** Extremely rare -- 1 event across 3 verification replays. Fires when a destructible/breakable prop is spawned as physics debris (e.g. shattered crate or breakable cover).

**Fields:**

| Field                            | Type         | Default | Notes                                                     |
|----------------------------------|--------------|---------|-----------------------------------------------------------|
| `origin`                         | `CMsgVector` | --      | Spawn position of the physics prop                        |
| `velocity`                       | `CMsgVector` | --      | Initial linear velocity                                   |
| `angles`                         | `CMsgQAngle` | --      | Initial orientation                                       |
| `skin`                           | `fixed32`    | `0`     | Model skin index                                          |
| `flags`                          | `uint32`     | --      | Spawn flags bitfield                                      |
| `effects`                        | `uint32`     | --      | Effects bitfield (e.g. EF_NODRAW, EF_NOSHADOW)            |
| `color`                          | `fixed32`    | --      | Packed RGBA tint                                          |
| `modelindex`                     | `fixed64`    | --      | Model resource index                                      |
| `unused_breakmodelsnottomake`    | `uint32`     | --      | Legacy / unused field carried from Source 1               |
| `scale`                          | `float`      | --      | Model scale                                               |
| `dmgpos`                         | `CMsgVector` | --      | Position of the damage that caused the spawn              |
| `dmgdir`                         | `CMsgVector` | --      | Direction of the damage that caused the spawn             |
| `dmgtype`                        | `int32`      | --      | Damage type that caused the spawn                         |

**Product Alignment:** None. Breakable-prop debris is a visual artifact, not a game-state event. There is no entity index for the source breakable, so this can't be joined back to map objectives. Skip unless investigating environment-destruction telemetry as a future feature.

### Other unknowns seen but already identified

For completeness, the 2026-04-09 probe also saw:
- IDs 205, 207, 208, 209, 210, 212 -- all `EBaseGameEvents` base engine sound/legacy events. Not in the Citadel namespace; documented upstream in `netmessages.proto`.
- ID 450 -- `ECitadelGameEvents::GE_FireBullets` (see table above).
- ID 466 -- `ECitadelGameEvents::GE_RemoveBullet` (see table above).

### Note on `te.proto` upstream availability

`te.proto` is **not in `deadlock-api/valveprotos-rs`** as of 2026-04-09 (rev `9625c07` and master). It is in the `SteamDatabase/GameTracking-Deadlock` mirror under `Protobufs/te.proto`. If TE messages ever become product-relevant, the cleanest path is to vendor `te.proto` into a local `protos/` directory and add it to the parser's `prost-build` invocation, rather than waiting for upstream to add it.
