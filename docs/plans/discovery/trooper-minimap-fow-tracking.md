# Trooper Minimap FOW Tracking Discovery

> **File location:** `private/plans/discovery/`

## Context

Lane troopers are currently tracked per-NPC: the parser subscribes to every `CNPC_Trooper` entity, reads `m_iLane` / `m_iTeamNum` / `CBodyComponent.m_{cellX,cellY}` / `m_NPCState` / `m_lifeState` / `m_iHealth`, then layers on a cage-entity filter, a ghost-creep whitelist, and custom wave-reassignment to handle Deadlock's entity-slot recycling (`parser/src/tracking/creep_tracker.rs:40-411`). A single match produces ~1,744 CREATE events across ~234 recycled slots.

The SendTables expose what looks like a server-side aggregate on the game-rules singleton:

- `CCitadelGameRulesProxy.m_pGameRules.m_hTrooperMinimap: CHandle<CCitadelTrooperMinimap>` -- a handle to a singleton minimap controller
- `CCitadelTrooperMinimap.m_vecFOWEntities: CUtlVectorEmbeddedNetworkVar<STrooperFOWEntity>` with capacity 192 -- an embedded-netvar vector of per-trooper records, where "FOW" plausibly means per-team fog-of-war visibility

Pre-research (two parallel agents + `entity-types-runtime-census.md:134`) confirmed: the singleton is present at runtime (1 CREATE with 1 unique index in match 68175583 -- the census table does not track Deletes for this class, so full-match presence is a To-Validate assumption), `CUtlVectorEmbeddedNetworkVar` maps to haste's `DynamicSerializerArray` so per-slot per-subfield keys work correctly (it is NOT subject to the `FixedArray`-of-primitives collapse bug in `reference_haste_fixed_array_collapse.md`), and replays carry the server-authoritative stream so any FOW bits would be ground truth rather than client-filtered data. The schema of `STrooperFOWEntity` itself is NOT documented in any local proto or spec and must be dumped from SendTables before any design decision.

### Contract impact

Any Decision-Tree branch except `Stop` changes the parser → backend contract (`private/specs/contracts/parser-api.md`). Call-outs for the implementation plan that graduates from this discovery:

- **Replace branch:** `lane_creep_data.creeps` is keyed today by `CNPC_Trooper` entity_index. If we switch to the minimap entity, the implementation plan must either (a) preserve those keys by joining through the `CHandle<CNPC_Trooper>` field (Q5) or (b) define a new key scheme (e.g. `minimap_slot_id`) and update the contract + downstream backend + frontend consumers.
- **Augment branch:** any FOW / per-team visibility fields require a new contract type (suggested name: `TrooperFOWData` or a `fow_visibility` sub-field on `LaneCreepData`) specified in `parser-api.md` before implementation begins.
- **Cage-phase overlap:** if the minimap entity covers cage/zipline troopers (Q8), the `is_cage` field on `CreepSnapshot` may be replaceable or redundant -- the implementation plan needs to decide.

None of this needs to be resolved in this discovery -- these are headers the implementation plan's contract-update section must fill in.

**Goals:** What decisions will this discovery inform?

- Whether to **replace** `CNPC_Trooper` subscription with the minimap entity, **augment** the existing tracker with minimap-derived fields, or **reject** the minimap approach and document the dead end
- Whether per-team FOW visibility data is exposed on this entity (would unlock new analytics: vision denial, lane awareness, "team was blind to the push")
- What to add to `private/specs/entity-fields-reference.md` and the next entity-types runtime census so this class is no longer a blind spot

---

## Open Questions

Specific enough that a data citation closes each one.

1. **What fields does `STrooperFOWEntity` expose?** Full name + C++ type list from SendTables, ordered by field index.
2. **Does `STrooperFOWEntity` carry a lane identifier** (`m_iLane` or equivalent)? If yes, what value range and does it match the `CNPC_Trooper.m_iLane` convention (0 = pre-match, 1--4 = lanes)?
3. **Does it carry position, and at what precision?** Full world-unit encoding (cell + vec pair) or a lower-precision 2D minimap coordinate? Do the values agree with `CNPC_Trooper` position within ≤50 world units?
4. **Does it carry health**, and at what fidelity? Absolute `int32`, bucketed (e.g. 0/25/50/75/100 percent), or just an alive/dead flag?
5. **Does it carry a `CHandle<CNPC_Trooper>` (or other entity handle)** that links each slot back to the underlying NPC entity? This is the make-or-break field for a hybrid replace+augment strategy.
6. **Does "FOW" mean per-team visibility bits are exposed?** E.g. `visible_to_team0`, `visible_to_team1`, `last_seen_tick_team0`, or a packed bitmask? Are they populated with differing values between teams in the same slot (proving per-team semantics)?
7. **What is the slot lifecycle?** Does the vector length shrink / compact on trooper death (slot indices shift), or are slots stable with an alive-flag / zeroed-fields approach?
8. **Does the minimap entity track the cage / zipline phase** (before lane touchdown), or only post-drop live troopers? If the former, does it overlap with our current cage filter? (If yes, note whether slot count during cage phase matches the current cage-entity count of 4 per wave -- determines whether the cage filter can be replaced entirely.)
9. **What is the entity scope**? Lane troopers only, or also flex-guards, neutral camps, and other AI actors that the current tracker excludes or handles separately?
10. **Cost comparison:** across a full match, how many per-tick update events does the minimap entity generate vs the existing `CNPC_Trooper` subscription path?
11. **Position fidelity over time:** for seconds where both sources are populated, what are the p50 / p95 / max world-unit deltas between paired minimap slot positions and `CNPC_Trooper.CBodyComponent` positions?

---

## Assumptions

### To Validate

- [ ] `STrooperFOWEntity` sub-fields are per-slot decodable in `deadlock-api/haste` (i.e. `DynamicSerializerArray` keygen works; not subject to the `FixedArray` slot-collapse bug). -- *How to check: Phase 2 probe reads two different slot indices simultaneously and observes distinct values rather than collapsed last-write values.*
- [ ] `CCitadelTrooperMinimap` is present for the full match (not gated on first wave spawn or a specific match phase). -- *How to check: Phase 2 probe logs CREATE tick and confirms the entity is observed from the earliest sampled tick through to match end.*
- [ ] Any FOW fields on `STrooperFOWEntity` carry server-authoritative truth for all teams (not filtered to a single recording team). -- *How to check: Phase 3 analysis compares FOW field values across both team bits / team slots for the same underlying trooper and confirms at least one tick where values differ.*
- [ ] Position values in `STrooperFOWEntity` agree with `CNPC_Trooper` positions within ≤50 world units after nearest-neighbor pairing. -- *How to check: Phase 3 analysis reports p50 / p95 / max deltas.*
- [ ] The `fxhash::hash_bytes(b"CCitadelTrooperMinimap")` subscription key resolves to this singleton in our `deadlock-api/haste` build. -- *How to check: Phase 1 output confirms exactly 1 entity matches this hash in the runtime class census; Phase 2 on-entity callback fires and populates at least the vector-length field.*

### Accepted (not tested here)

- Canonical haste fork is `deadlock-api/haste` -- *Risk if wrong: the field decode path differs. Mitigation: the runtime census already observed `CCitadelTrooperMinimap` through our current parser, so the fork can at least see the class.*
- 1-second probe granularity is sufficient to characterize slot lifecycle; sub-second compaction events (if any) will be missed. -- *Risk if wrong: we falsely conclude slots are stable when they actually compact between samples. Mitigation: Phase 2 also logs raw update-event counts per slot alongside sampled values, surfacing churn that implies between-sample mutation.*
- Match 68175583 is representative (single-replay discovery). -- *Risk if wrong: a field we deem "always zero" might be populated in other replays. Mitigation: call out unpopulated fields in Findings with "unpopulated in 68175583" so a later spike can re-check against another replay.*
- "Trooper" in the class name refers to lane creeps specifically (`CNPC_Trooper`), not neutrals or flex-guards. -- *Risk if wrong: entity doesn't cover everything we want; reduces the replace case. Phase 3 will reveal this by comparing populated minimap slot counts to the set of alive `CNPC_Trooper` entities per second.*
- No Valve patch has changed this entity's shape between the match-68175583 demo and today's replay schema. -- *Risk if wrong: stale field list. Mitigation: cross-check field names against `deadlock.wiki` patch changelogs if anything looks off.*

---

## Agent Assignments

| Question(s) | Agent | Approach |
|-------------|-------|----------|
| Q1, Q2, Q5, Q6, Q8, Q9 | haste-expert → rust-parser | Phase 1: run `probe_all_entity_classes` with filter, enumerate every field + type, annotate each with a confidence label (`confirmed` field existence, `inferred` / `hypothesis` semantics) |
| Q3, Q4, Q7, Q10 | rust-parser | Phase 2: author and run `probe_trooper_minimap.rs` at 1s granularity per `feedback_probe_granularity.md`; dump populated values and raw update-event counts |
| Q11 | rust-parser | Phase 3: analyze paired samples from the Phase 2 output, produce p50 / p95 / max position-delta table |

All agents must follow the probe-commit-before-cleanup rule (`feedback_probe_commit_before_cleanup.md`): persist `probe_trooper_minimap.rs` to `private/engineering/tools/` BEFORE any `parser/src/bin/` cleanup.

---

## Research Standards

Follow `.claude/docs/shared/research.md` for confidence labels (`confirmed` / `inferred` / `hypothesis`), citation format, and scope discipline.

---

## Investigation Approach

Target replay: `/parser/src/replays/68175583_527726523.dem` (same replay as `entity-types-runtime-census.md`, so `CCitadelTrooperMinimap` presence is already confirmed for this file).

### Phases

**Phase 1 -- Field enumeration (blocks Phase 2 + Phase 3)**

1. Copy `private/engineering/tools/probe_all_entity_classes.rs` → `parser/src/bin/probe_all_entity_classes.rs` (per that probe's own header comment -- it is NOT part of the normal build).
2. Ensure the parser container is up: `docker compose up -d dashjump-parser`.
3. Run with a `CCitadelTrooperMinimap` filter:
   ```
   docker compose exec dashjump-parser cargo run --bin probe_all_entity_classes -- \
     /parser/src/replays/68175583_527726523.dem CCitadelTrooperMinimap
   ```
4. Re-run with filter `TrooperFOW` (catches `STrooperFOWEntity` if it appears as a distinct serializer):
   ```
   docker compose exec dashjump-parser cargo run --bin probe_all_entity_classes -- \
     /parser/src/replays/68175583_527726523.dem TrooperFOW
   ```
5. If `STrooperFOWEntity` does NOT appear as its own serializer in step 4, inspect the full field dump for `CCitadelTrooperMinimap`: the embedded struct's fields may be flattened into the parent serializer or listed via a `field_serializer_name` reference. If neither, fall back to inspecting `deadlock-api/haste`'s `FlattenedSerializers::parse` output for the embedded-struct path.
6. Pipe output to `private/engineering/samples/trooper_minimap_68175583_phase1.log` (gitignored or small enough to commit -- follow the pattern in `private/engineering/samples/`).
7. Delete `parser/src/bin/probe_all_entity_classes.rs` (do NOT commit the copy -- per the probe header).

**Exit criteria:** Every field on `CCitadelTrooperMinimap` and `STrooperFOWEntity` captured with name and C++ type. Answers Q1 (full schema), Q2 (lane field presence), Q5 (handle presence), Q6 (FOW field presence), partial Q8--Q9 (field names hint at entity scope).

**Phase 2 -- 1s sampler probe (depends on Phase 1 field list)**

1. Author `private/engineering/tools/probe_trooper_minimap.rs`. **Commit this path first**, before copying into the parser crate -- see `feedback_probe_commit_before_cleanup.md`. Model the file header after `probe_all_entity_classes.rs` (copy-to-run-then-delete workflow, reference to this plan).
2. The probe subscribes to:
   - `CCitadelTrooperMinimap` (serializer hash via `fxhash::hash_bytes(b"CCitadelTrooperMinimap")`)
   - `CNPC_Trooper` (for cross-check, using the existing constant in `parser/src/entities/constants.rs:27`)
3. On `on_tick_end`, gated by `tick % 60 == 0` (1s granularity per `feedback_probe_granularity.md`):
   - Emit the current minimap-vector length
   - For each occupied slot 0..length, emit every field from the Phase 1 list (raw value, no interpretation)
   - Emit the full `CNPC_Trooper` census at the same tick: entity_index, `m_iLane`, `m_iTeamNum`, `CBodyComponent.m_{cellX,cellY}` → (x,y), `m_iHealth`, `m_lifeState`, `m_NPCState`
4. Between ticks, maintain per-slot update-event counters (incremented on every field update, not just sampled ticks). Emit these counters at each 1s sample so churn can be reconstructed post-run.
5. Output to `private/engineering/samples/trooper_minimap_68175583.jsonl` (one JSON object per sampled tick; format: `{tick, matchtime_s, minimap_slots: [...], cnpc_troopers: [...], update_counts: {...}}`). Plain text is acceptable if adding a JSON dep is heavier than the analysis needs -- the format must be machine-parsable for Phase 3.
6. Run inside the container with `exec` (not `run`), capturing the first ~15 minutes of match time (covers 3--4 lane waves plus Guardian engagements):
   ```
   docker compose exec dashjump-parser cargo run --release --bin probe_trooper_minimap -- \
     /parser/src/replays/68175583_527726523.dem
   ```
7. Delete `parser/src/bin/probe_trooper_minimap.rs` after the run completes.

**Exit criteria:** Machine-parsable sample artifact covers match tick 0 → ≥ 15 minutes of match time with per-second minimap + CNPC_Trooper snapshots, AND includes per-slot update counts (not just sampled field values -- counters maintained between sampled ticks). Answers Q3 (position values), Q4 (health values), Q7 (lifecycle: inspect vector length trajectory + update counts), Q10 (cost: sum update counts vs CNPC_Trooper tracker event volume).

**Phase 3 -- Cross-check analysis (depends on Phase 2 artifact)**

Run an analysis pass against `trooper_minimap_68175583.jsonl`. A small Python script is appropriate (the backend container already has Python / pandas); do NOT add analysis code to `parser/src/bin/` or to `backend/app/`. Keep it in `private/engineering/tools/` as `analyze_trooper_minimap.py` alongside the probe.

For each sampled second:

- **Coverage:** `len(minimap_slots)` vs `count(alive CNPC_Trooper with lane != 0 and m_iHealth > 0)`. Flag ticks where the two counts diverge by more than 2.
- **Pairing:** nearest-neighbor match each minimap slot to the closest alive `CNPC_Trooper` by Euclidean distance. Record the distance.
- **Position fidelity:** aggregate paired distances across all sampled ticks. Report p50, p95, max in world units.
- **Lane accuracy:** if `STrooperFOWEntity` carries a lane field, compare to `m_iLane` on the paired `CNPC_Trooper`. Report agreement rate.
- **Lifecycle:** detect vector-length decreases (slot compaction) vs stable-length with alive-flag toggle. Also detect slot-index reuse within the minimap entity.
- **FOW semantics:** if FOW fields exist, for each sampled tick find a slot where the per-team fields differ -- confirms per-team semantics. If they never differ, FOW is just a naming convention.
- **Cost:** sum per-slot update counts across the match; compare to an estimate of `CNPC_Trooper` updates derived from CREATE events × average sampled-field update rate (or instrument it directly in the probe).

Write the result as a Markdown table into `private/engineering/samples/trooper_minimap_crosscheck.md` with each row answering one of Q7, Q10, Q11.

**Exit criteria:** Q10 and Q11 answered with numerical evidence. Q7 answered with a concrete lifecycle model (stable / compacting / hybrid). Q6 FOW semantics reported (populated vs unpopulated; per-team divergence observed or not). Discovery Checkpoint can be filled in.

---

## Decision tree

| Field completeness (lane + position + team + health/alive) | Slot lifecycle | FOW fields present & per-team | Path forward |
|---|---|---|---|
| All present, position fidelity ≤50 world units | Stable slots | Yes, populated | **Implementation plan** -- replace `CNPC_Trooper` subscription with minimap entity + add FOW-visibility feature (new metric in lane-pressure service) |
| All present, position fidelity ≤50 world units | Stable slots | No / single-truth | **Implementation plan** -- replace `CNPC_Trooper` subscription for cost + lifecycle wins; no FOW feature |
| All present, position fidelity > 50 world units | Stable slots | -- | **Implementation plan (augment)** -- keep `CNPC_Trooper` as position source, adopt minimap for team/lane/alive (authoritative lifecycle) and any FOW bits |
| Handle-back-to-CNPC_Trooper present, other fields partial | Stable slots | -- | **Implementation plan (augment)** -- minimap becomes a lightweight secondary index; keep existing reader for the fields it does carry; use handle for join |
| No lane field AND no handle, other fields (position + team + FOW) present | Stable slots | Yes | **Implementation plan (narrow augment)** -- use minimap for FOW analytics feature only; keep `CNPC_Trooper` as the sole source of truth for all lifecycle / lane / position tracking |
| Slot compaction on death (indices shift between ticks) | -- | -- | **Another spike** (timebox ≤ 1 day) -- investigate whether a handle field makes slot identity recoverable; if not, **reject** minimap as primary source and stop |
| Entity unpopulated or all fields always zero in the sampled replay | -- | -- | **Stop** -- document the dead end in `entity-fields-reference.md` with a one-line "present but unused in server stream"; no further action |

---

## Probe / Query Artifacts *(agents fill in)*

- `private/engineering/tools/probe_trooper_minimap.rs` -- Phase 2 sampler; rust-parser persists here BEFORE copying into the parser crate for execution
- `private/engineering/tools/analyze_trooper_minimap.py` -- Phase 3 analysis script; reads the JSONL artifact and emits the cross-check table
- `private/engineering/samples/trooper_minimap_68175583_phase1.log` -- raw field dump from Phase 1
- `private/engineering/samples/trooper_minimap_68175583.jsonl` -- Phase 2 per-second samples (minimap + CNPC_Trooper)
- `private/engineering/samples/trooper_minimap_crosscheck.md` -- Phase 3 analysis output

---

## Discovery Checkpoint *(agents fill in)*

**Status:** `[x] Complete -- 2026-04-19`

### Results

- [x] **Q1 (full field list):** `CCitadelTrooperMinimap` has 2 fields: `m_timeLastUpdate: GameTime_t` and `m_vecFOWEntities: CUtlVectorEmbeddedNetworkVar<STrooperFOWEntity>`. `STrooperFOWEntity` has exactly 3 fields: `m_nPositionXY: uint16`, `m_nEntIndex: CEntityIndex`, `m_nTeam: int8`. -- **confirmed** -- `private/engineering/samples/trooper_minimap_68175583_phase1.log:5488-5495`
- [x] **Q2 (lane field):** **Absent.** `STrooperFOWEntity` has no lane field. (`STeamFOWEntity` on the sibling entity `CCitadelTeam` does carry `m_iLane: int32` -- see Q6.) -- **confirmed** -- `trooper_minimap_68175583_phase1.log:5492-5495`
- [x] **Q3 (position precision):** `m_nPositionXY` is a packed uint16 with `hi-byte = y_cell` and `lo-byte = x_cell`, both unsigned 8-bit. World coordinate = `(byte - 128) * ~84`. Empirical linear-regression fit: `world_x = lo*83.912 - 10647.0`, `world_y = hi*83.973 - 10659.4`, r^2 = 0.9950 / 0.9931 on 75,164 live pairs. Effective resolution: step 84 world units/cell over [-10752, +10752]. **Agreement with `CNPC_Trooper`: p50 = 42 world units, p95 = 80, max = 24024 (outliers during death/respawn).** -- **confirmed** -- `private/engineering/samples/trooper_minimap_crosscheck.md:11,47-60` and `private/engineering/tools/analyze_trooper_minimap.py:104-108`
- [x] **Q4 (health fidelity):** **No health field.** Only "slot occupied" vs "slot parked" (sentinel `m_nEntIndex <= 1`) is observable. (`STeamFOWEntity` carries `m_nHealthPercent: uint8` -- a health-at-percent-resolution field lives on the sibling schema, not this one.) -- **confirmed** -- `trooper_minimap_68175583_phase1.log:5492-5495`
- [x] **Q5 (handle back to `CNPC_Trooper`):** **Present.** `m_nEntIndex` is emitted as `(CNPC_Trooper.entity_index << 1)` -- low bit is always 0 across 345,879 occupied samples; right-shift by 1 yields the raw entity index. Join rate: 54.8% to a live `CNPC_Trooper`, 16.9% to a recently-dead one, 28.3% parked (`ent_idx <= 1`). The 24% "not live in census" rows are slots that haven't yet been re-assigned after their trooper died -- not a join-field failure. -- **confirmed** -- `trooper_minimap_crosscheck.md:11,32-41`
- [x] **Q6 (FOW per-team fields):** **Absent on this entity.** `STrooperFOWEntity` carries only `m_nTeam` (the owning team of the slotted trooper) -- no visibility bitmask, no per-team-visible-to-me fields, no `m_nTickHidden`. However, Phase 1 discovered a different entity, `STeamFOWEntity` (12 fields), living in `CCitadelTeam.m_vecFOWEntities` which does carry `m_bVisibleOnMap: bool`, `m_nTickHidden: GameTick_t`, `m_iLane: int32`, `m_nHealthPercent: uint8`, `m_eClass: Class_T`, and `m_eHeight: EMinimapHeight`. That is the correct source of per-team FOW truth but it is out of scope for this discovery. -- **confirmed** -- `trooper_minimap_68175583_phase1.log:5497-5508` and `trooper_minimap_crosscheck.md:69-73`
- [x] **Q7 (slot lifecycle):** **Stable vector, heavy slot recycling, no compaction.** The `m_vecFOWEntities` vector is pre-allocated to 192 slots on tick 1 and remains at 192 for the entire match (no length shrink on death). Slots are reused: 114/192 slots saw ≥2 `m_nEntIndex` re-assignments; 77/192 saw ≥10; the heaviest slot churned 167 times. Team is NOT fixed per slot -- max 31 `m_nTeam` changes on a single slot (slots are used for whichever team's trooper died first and re-used by either team). First 192 team assignments are initial setup; the remaining 1,019 are recycling-driven re-assignments. -- **confirmed** -- `trooper_minimap_crosscheck.md:17-30`
- [x] **Q8 (cage / zipline coverage):** **Minimap is populated before laning but the schema cannot distinguish cage/zipline from in-lane troopers.** The vector has 192 occupied slots from tick 1 (matchtime_s = -18.0 pre-match), well before any wave touches down. 28.3% of sampled slots remain in sentinel state (`ent_idx <= 1`) throughout the match, suggesting this pool also covers the pre-drop cage phase. With no `m_nState` / `m_bInCage` / `m_nHeight` field, the minimap cannot replace the existing cage-entity filter. -- **inferred** (scope: the "sentinel 28%" interpretation is based on post-match samples; confirming the cage-phase mapping would need a secondary probe correlating minimap parked slots against cage-entity indices) -- `trooper_minimap_crosscheck.md:32-41`
- [x] **Q9 (entity scope vs `CNPC_Trooper`):** **Lane troopers only -- no other AI classes observed.** Of non-sentinel (`ent_idx > 1`) samples, 100% had `(ent_idx >> 1)` matching either a live or recently-dead `CNPC_Trooper` in the census. No neutral camps, flex-guards, or other NPC classes were observed via the join. The 192-slot capacity (= 6 waves × 32 max concurrent troopers, or 4 lanes × 48, etc.) also points to a purpose-built lane-trooper pool. -- **confirmed** -- `trooper_minimap_crosscheck.md:32-41`
- [x] **Q10 (update-volume comparison):** Across the full 40-minute match: **53,762 `on_entity` callbacks for `CCitadelTrooperMinimap`** vs **4,539,276 for `CNPC_Trooper`** -- a **84.4x** reduction. Field-delta breakdown on the minimap side: pos_xy = 457,090 (98.3% of all deltas), ent_idx = 6,481 (1.4%), team = 1,211 (0.3%). Per-second rates: minimap 22.3/s, CNPC_Trooper 1,885.9/s. -- **confirmed** -- `trooper_minimap_crosscheck.md:75-83`
- [x] **Q11 (position fidelity p50/p95/max):** Using the derived decoder, paired against live `CNPC_Trooper` positions (76,635 pairs post-match): **p50 = 41.9, p95 = 79.8, max = 24,024.3 world units.** Per-lane breakdown: lane 1 p50 = 51.3, lane 4 p50 = 34.8, lane 6 p50 = 41.3. The p95 being double the p50 and the max tail ~24k world units both stem from the minimap lagging 1-2 ticks behind CNPC_Trooper during rapid movement (respawns, teleports). The plan's "≤50 world units" threshold in Q3 is met at the median but NOT at p95. -- **confirmed** -- `trooper_minimap_crosscheck.md:47-67`
- [x] **Learnings appended to `private/learnings.md`** (2 drafts: STrooperFOWEntity schema + STeamFOWEntity sibling discovery + encoding reference; minimap lifecycle vs CNPC_Trooper recycling)

### Assumptions check

- [x] **Per-slot decodability (DynamicSerializerArray) -- held.** 192 distinct slots observed with distinct `pos_xy` values in the same tick; no slot collapse. Low bit of `ent_idx` being always 0 is a real encoding quirk, not decoder error (confirmed by perfect 2:1 ratio vs CNPC_Trooper indices). -- `trooper_minimap_68175583.jsonl` (any sampled tick with 192 varied slots)
- [x] **Full-match entity presence -- held.** `first_minimap_create_tick = 1`, `last_seen_tick = 154044`, `occupied_slots_end_of_match = 192`. Entity was present every sampled second from pre-match through post-match. -- `trooper_minimap_crosscheck.md:5` (summary counters)
- [x] **Server-authoritative FOW fields -- n/a (fields absent).** `STrooperFOWEntity` does not expose FOW visibility bits; the assumption is moot for this entity. Defer to a follow-on `STeamFOWEntity` spike to validate this against the richer per-team schema.
- [x] **Position agreement ≤50 world units -- partially held.** p50 = 42 (held), p95 = 80 (invalidated at p95 threshold). The 80-unit p95 is acceptable for minimap-grade analytics but NOT for replacing `CNPC_Trooper.CBodyComponent` for precision features. -- `trooper_minimap_crosscheck.md:49`
- [x] **fxhash-based subscription key -- held.** Phase 2 probe subscribed via `hash_bytes(b"CCitadelTrooperMinimap")` and recorded 53,762 `on_entity` callbacks on a singleton; no missed or duplicated matches. -- probe source: `private/engineering/tools/probe_trooper_minimap.rs`
- **Accepted assumptions worth flagging based on findings:** The "Trooper in class name = lane creeps only" assumption held empirically (Q9). The "1-second granularity is sufficient" assumption held for slot-lifecycle characterization; however, the pos_xy field updates far more frequently than 1 Hz (457k deltas / 2406s ≈ 190 Hz per match, or ~1 Hz per slot), so 1s sampling is at the Nyquist limit for per-slot motion -- higher-frequency sampling would be needed to audit position accuracy frame-by-frame.

### Evidence

Phase 1 field dump (annotated summary, line 5488+):
```
CCitadelTrooperMinimap (2 fields):
  m_timeLastUpdate: GameTime_t
  m_vecFOWEntities: CUtlVectorEmbeddedNetworkVar< STrooperFOWEntity >

STrooperFOWEntity (3 fields, trooper-specific slim schema):
  m_nPositionXY: uint16       # packed 2D minimap coord
  m_nEntIndex: CEntityIndex   # handle back to underlying CNPC_Trooper
  m_nTeam: int8

STeamFOWEntity (12 fields, lives inside CCitadelTeam.m_vecFOWEntities -- DIFFERENT container):
  m_nPositionX: uint8, m_nPositionY: uint8
  m_nEntIndex: CEntityIndex
  m_nTeam: int32
  m_eClass: Class_T
  m_iLane: int32
  m_eHeight: EMinimapHeight
  m_bVisibleOnMap: bool
  m_bBackdoorProtectionActive: bool
  m_nTickHidden: GameTick_t
  m_strEntityName: CUtlString
  m_nHealthPercent: uint8
```

Phase 2 JSONL final summary row:
```json
{"kind":"summary","match_start_time_s":18.9375,"tick_interval":0.015625,
 "last_seen_tick":154044,"first_minimap_create_tick":1,
 "minimap_on_entity_calls":53762,"cnpc_trooper_on_entity_calls":4539276,
 "minimap_slot_field_deltas_total":464782,
 "delta_breakdown":{"pos_xy":457090,"ent_idx":6481,"team":1211},
 "max_slot_seen":191,"occupied_slots_end_of_match":192}
```

Phase 3 paired row (tick 16440, matchtime_s = 237.9):
```
slot  43 pos_xy=0xC5A7 -> ( +3366, +5883)  vs trooper idx=2871 lane=6 team=3 world=( +3376, +5865)  d=21 world units
slot  45 pos_xy=0x586F -> ( -1333, -3270)  vs trooper idx=2821 lane=4 team=2 world=( -1384, -3307)  d=64 world units
slot  48 pos_xy=0x5A71 -> ( -1165, -3102)  vs trooper idx=2772 lane=4 team=2 world=( -1208, -3077)  d=49 world units
```

### Deferred questions

- **Cage/zipline overlap (Q8, partial):** the plan asked whether slot count during the cage phase matches the current cage-entity count of 4 per wave. Phase 2 does not sample cage-entity census, so the positive claim "minimap can replace the cage filter" is unanswerable from this data alone. Would be resolved by a narrow follow-on probe that also reads the cage-entity census. Not blocking the decision-tree landing.
- **Slot-ordering stability (Q7, adjacent):** the plan did not ask about slot ordering vs wave spawn order, but the heavy slot churn (up to 167 ent_idx changes on slot 125) suggests that the vector is NOT a per-wave FIFO. Resolving this is only interesting if a future feature needs slot-stable identifiers -- kept as an open item for implementation-plan scoping.
- **STeamFOWEntity per-team semantics (Q6 follow-on):** whether `STeamFOWEntity.m_bVisibleOnMap` differs between the two `CCitadelTeam` instances for the same underlying entity (proving per-team FOW) is unanswered. Requires a separate discovery/spike on `CCitadelTeam.m_vecFOWEntities`.

---

**STOP. Present the following to the user before doing anything else:**

1. Answers to every open question with confidence labels
2. Data model or approach recommendation based on findings (which branch of the Decision tree the findings land on)
3. Any new analytics / enrichment opportunities flagged during investigation (especially FOW-derived features)
4. Unresolved questions and what would resolve them

Await user decision. If approved, create an implementation plan (or fix / spike / stop) before writing any production parser or backend code.
