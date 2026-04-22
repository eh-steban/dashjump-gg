# Learnings

Cross-project discoveries and patterns that affect multiple services or prevent repeated learning cycles.

**How to use:** Check `private/learnings-index.md` first to find relevant learnings by topic, service, or problem type.

---

## S3 Storage Solves JSONB Bottleneck Better Than Differential Encoding

**Date discovered:** December 2025, Backend architecture spike
**Impact:** Backend (storage strategy), Parser (output format), Frontend (data transfer)
**Status:** active

PostgreSQL JSONB hits performance limits at 15-18MB per match. Differential encoding adds complexity without solving the core problem. S3 storage with PostgreSQL metadata achieves query performance, storage efficiency, and easier integration simultaneously.

**Key Takeaway:**
For large per-match data: Store raw + transformed JSON in S3, keep metadata in PostgreSQL.

**Related Docs:**
- Architecture details: [backend-mental-model.md](../.claude/rules/backend/backend-mental-model.md)
- Implementation specs: private/specs/ (TBD — S3 migration spec)

**When to Reference:**
- Evaluating storage strategies for new features
- Understanding why we use S3 instead of JSONB
- Sizing infrastructure costs
- Explaining architecture decisions to coaches/partners

**Prevention:**
✓ New large-data features default to S3 storage pattern
✓ JSONB reserved for small metadata only
✓ Specs document storage assumptions upfront

---

## Wave Priority Tracking > Raw Kill Data (Coach Feedback Pattern)

**Date discovered:** January 2026, coach interviews ([redacted coach] + [redacted coach])
**Impact:** Strategic (roadmap priority), Backend (data focus), Frontend (visualization priority)
**Status:** validated

Multiple coaches independently mentioned wave/lane priority as valuable for decision-making. None emphasized kill data as primary need (though useful context). Wave tracking (creep position, lane pressure, rotation opportunities) has higher perceived value than damage metrics.

**Key Takeaway:**
Prioritize wave/lane analytics over detailed damage breakdowns. Coach decisions depend on positioning more than kills.

**Related Docs:**
- Experiment and spec not yet created. Create via `/new-experiment` when pursuing this feature.

**When to Reference:**
- Prioritizing features against coach feedback
- Explaining why we build X before Y
- Validating if new feature idea aligns with coach needs
- Making roadmap decisions

**Prevention:**
✓ Features default to coach-validated needs (not guesses)
✓ Every spec references validation source
✓ Monthly check-in with coaches to re-validate priorities

---

## Deadlock Entity Field Enums Are Not in Protobufs

**Date discovered:** March 2026, Parser debugging session (rust-parser agent)
**Impact:** Parser (entity field debugging), any future service that introspects Deadlock entity state
**Status:** active
**Graduated to:** `.claude/rules/parser/parser-mental-model.md` — "Entity Field Lookup Tools" section

When debugging Deadlock entity fields (e.g., `m_NPCState` on `CNPC_Trooper`), the instinct is to search `valveprotos-rs` proto files. This will fail. Protobufs only cover network messages (game events, netmessages). Game-engine field types and enums are embedded in the demo's SendTables, not in any `.proto` file.

**Key Takeaway:**
Never search proto files for game-engine entity field enums. Use the `uniquetypes` tool or `haste-inspector` against an actual `.dem` file.

**Related Docs:**
- Full tool lookup guide: `.claude/rules/parser/parser-mental-model.md` — "Entity Field Lookup Tools" section

**When to Reference:**
- Debugging unexpected integer values in entity fields (`m_NPCState`, `m_lifeState`, etc.)
- Adding tracking for a new entity type whose field values are unknown
- Wondering why proto search returns no results for a Deadlock entity field

**Prevention:**
✓ Never search protos for entity field enums — go to SendTables tooling directly
✓ Check parser-mental-model.md "Entity Field Lookup Tools" before starting any new entity tracker

---

## Cargo Is Container-Only; Worktrees Are Not Auto-Mounted

**Date discovered:** March 2026, rust-parser agent working from a git worktree
**Impact:** All agents doing parser work from a non-main worktree; any future worktree-based Rust task
**Status:** active

Rust/Cargo is not installed on the WSL host -- it exists only inside Docker containers. When a rust-parser agent works from a git worktree (e.g. `dashjump-gg-haste-migration/`), neither the devcontainer nor the running parser container has the worktree's `parser/` directory mounted. Both containers fix their bind-mounts at the main repo root. Running `cargo build` or `cargo test` from the worktree directory on the host will fail with `cargo: command not found`.

The correct approach is to use the worktree stack: `scripts/wt start <name>` spins up a parser container with the worktree's `parser/` directory mounted, after which `docker compose exec dashjump-parser cargo test` works as expected. The `dashjump-gg-cargo-cache` named volume is shared across all worktrees so no recompilation is required. See `.claude/rules/git.md` -- "Worktree Workflow / Running Tests" for the canonical commands.

**Key Takeaway:**
Cargo is not on the WSL host -- use `scripts/wt start <name>` to get a parser container with the worktree mounted, then run cargo commands via `docker compose exec`.

**Related Docs:**
- Worktree workflow: `.claude/rules/git.md` -- "Worktree Workflow" section
- Parser container setup: `parser/Dockerfile`

**When to Reference:**
- Starting parser work from a worktree directory
- `cargo: command not found` errors from a non-main repo directory
- Setting up any worktree that needs Rust build or test commands
- Advising a rust-parser agent on how to test worktree changes

**Prevention:**
✓ Use `scripts/wt start <name>` before any `cargo` work in a worktree -- this mounts the right `parser/` directory
✓ Use `docker compose exec dashjump-parser cargo test` (not `cargo test` directly on the host)
✓ Check `.claude/rules/git.md` "Running tests" section for the canonical worktree test commands

---

## parsedmatch Cache Must Be Invalidated on Parser Schema Changes

**Date discovered:** March 2026, adding m_nDeducedLane to parser output
**Impact:** Backend (cache invalidation), Parser (output schema changes), development workflow
**Status:** active

`parsedmatch` stores the full transformed match blob keyed by `(match_id, schema_version)`. When the parser output schema changes -- new fields, type fixes, renamed keys -- previously cached rows carry stale data that silently mismatches the new schema. This produces confusing bugs where code reading new fields always sees zero or None despite the parser emitting correct values.

In development: delete the affected row directly from the database. In production: bump `schema_version` in `backend/app/api/match.py`, which forces a cache miss and re-parse on next request. For Alembic migrations that alter the parsedmatch shape, add `op.execute("TRUNCATE parsedmatch")` at the top of `downgrade()` to prevent stale blobs from being deserialized by older code.

**Key Takeaway:**
Every parser output schema change requires a matching cache invalidation -- bump `schema_version` in production, delete rows in dev.

**Related Docs:**
- Cache key implementation: `backend/app/api/match.py`
- Backend mental model: `.claude/rules/backend/backend-mental-model.md`

**When to Reference:**
- Adding new fields to parser output JSON
- Changing field types in parser domain structs (e.g., `i32` to `f32`, adding new keys)
- Debugging "new field is always zero/None" after a parser change
- Writing Alembic migrations that touch parsedmatch schema
- Onboarding a new agent to the parser + backend integration

**Prevention:**
✓ Every parser schema change has a paired `schema_version` bump or dev-side row deletion
✓ Alembic `downgrade()` includes `TRUNCATE parsedmatch` for schema-affecting migrations
✓ Check for stale cache rows first when new parser fields read as zero/None

---

## Boss Objective Health: Parser Must Emit health=0 at Entity Delete

**Date discovered:** March 2026, lane pressure debug on match 68182475 (backend + parser agents)
**Impact:** Parser (boss_tracker.rs), Backend (lane_pressure_service.py), any future objective tracker
**Status:** active

When a Deadlock boss objective (guardian, walker, shrine) dies, the parser's `handle_boss_delete` must push a `(time, 0)` entry to `health_samples` at the deletion tick. Without this, carry-forward logic propagates the last pre-death health value indefinitely. Backend services that check `health > 0` to determine liveness then treat the objective as alive, targeting it for pressure calculations and producing 0% lane pressure for creep waves that have already advanced past the destroyed objective.

The reliable liveness pattern is two-signal: `death_time_s` as primary gate (set at deletion tick, more reliable) and `health_timeline` as belt-and-suspenders. Either signal alone can be absent or delayed by demo recording artifacts; checking both makes the system tolerant of edge cases.

**Key Takeaway:**
Objective lifecycle completeness is a parser responsibility -- the backend must not work around missing terminal health samples.

**Related Docs:**
- Parser implementation: `parser/src/tracking/boss_tracker.rs` -- `handle_boss_delete`
- Backend consumer: `backend/app/services/lane_pressure_service.py` -- `_current_target`, `_own_frontline_objective`
- Parser mental model: `.claude/rules/parser/parser-mental-model.md`

**When to Reference:**
- Implementing any new entity tracker with health or state timelines
- Debugging lane pressure or objective control calculations that look stale post-objective-death
- Reviewing carry-forward logic in any timeline-based computation
- Adding a new boss-type entity (walker variants, shrines) to the boss tracker

**Prevention:**
✓ Every `handle_*_delete` in an entity tracker pushes a terminal state sample (health=0, state=DEAD) at deletion time
✓ Backend liveness checks use `death_time_s` as primary gate, timeline as secondary
✓ Test objective lifecycle across a match that includes objective destruction

---

## Coach Interview Research

Detailed notes from coach interviews -- validated features, coach workflows, pain points, and follow-up actions -- are held in a private strategy doc and not published in this snapshot.

---

## Maintenance Notes

- **Last reviewed:** March 2026
- **Next quarterly review:** June 2026
- **Deprecated entries:** None yet
- **Total active learnings:** 6

---

## Drafts

Service agents append raw findings here. Only `spec-writer` may promote entries above this section or edit/remove drafts. Use `/consolidate-learnings` to process pending drafts.

**Format for new drafts:**
```
### [Draft] [Topic] — [agent: agent-name, date: YYYY-MM-DD]
[What was discovered and why it matters across services]
```

### [Draft] Session-as-Event-Log Pattern: Structural Plumbing Confirmed, Model Coherence Deferred -- [agent: backend-python, date: 2026-04-13]

A coaching conversation (5 turns, 3 tool calls) modeled as an append-only JSONL with events `{user_message, tool_call, tool_result, assistant_message}` survived replay in a fresh subprocess with no in-memory carryover. Tool results from the original run were substituted by ordinal index -- adapters were not re-executed. The log also survived a schema bump (new field added to fixture, schema_version 1->2) without migration. Evidence: `private/engineering/tools/coaching_session_spike/runs/live_20260413_164013.jsonl` + replay + schema_evo outputs.

**Pydantic-to-tool-string friction points (specific to this backend):**
- `TransformedMatchData.per_player_data` positions and damage arrays are enormous (one entry per second x 12 players). A real tool adapter needs explicit field projection -- raw flattening is not viable. A dedicated `get_player_positions(match_id, player_id, window_s)` tool is warranted.
- `LanePressureData.pressure` wave_id keys (e.g., `"1_0_0"`) are opaque without cross-referencing `LaneCreepData.wave_meta`. Flatten helpers must embed this semantic mapping or the model cannot interpret keys.
- `SteamPlayer` has no Deadlock-specific fields (no rank, no hero pool). Useful only for identity resolution; a coaching session will need at least one additional player-context tool.
- `account_id` in `MatchSummary` is 32-bit; `SteamPlayer.steamid` is 64-bit. Identity bridging across tools requires explicit handling or model confusion is likely.
- `None` entries in `LanePressureData.pressure` lists (no alive wave that second) carry coaching signal but are lost in average-only flattening. Include a null_seconds count or pass the raw list.

**Mocked-model caveat:** both "model" variants in the spike used scripted responses. Real semantic coherence (assumption #3 in the plan) is unvalidated. A follow-up spike using live `anthropic.messages.create()` with two real model IDs and the same JSONL replay pathway is required before this pattern can be called production-ready.

### [Draft] CMsgMatchMetaDataContentsPatched -- direct decode, no intermediate envelope -- [agent: rust-parser, date: 2026-04-10]
The `CCitadelUserMsg_PostMatchDetails.match_details` bytes decode directly as `CMsgMatchMetaDataContentsPatched` -- there is no intermediate `CMsgMatchMetaData` envelope step despite spec documentation suggesting otherwise. The `CMsgMatchMetaData` struct (version + match_details + match_id wrapper) exists in generated code but is not on the wire. Confirmed by `probe_post_match_details` against 3 replays. If other probes or agents add an extra decode step they will get a prost decode error. Use one step: `CMsgMatchMetaDataContentsPatched::decode(outer.match_details)`.

### [Draft] CMsgMatchPlayerPathsData absent from patched replay blob -- [agent: rust-parser, date: 2026-04-10]
`CMsgMatchPlayerPathsData` (player movement paths with combat_type and move_type) decoded as `None` in all 3 verification replays tested against `probe_post_match_details`. This sub-message may be stripped from the patched variant, version-gated to a newer replay format, or only present in private/GC API responses. Solo time tracking features that planned to use this field need a fallback. Do not block feature work on this data until presence is confirmed in newer replays.

### [Draft] assigned_lane uses CMsgLaneColor encoding, not sequential -- [agent: rust-parser, date: 2026-04-10]
`Players.assigned_lane` values observed in 3 replays are `{1, 4, 6}` -- exactly the `CMsgLaneColor` enum values (Yellow=1, Blue=4, Purple=6). Not sequential 0-3 or 1-4. The spec previously had this as "hypothesis: sequential 0-indexed" which is incorrect. Any code mapping lane numbers to lane colors must account for the non-contiguous encoding (no value 2, 3, or 5 for lanes; missing green=3 not yet observed but expected in replays with green-lane action).

### [Draft] Deadlock Sinner Death Signal: health==1 Not health==0 -- [agent: rust-parser, date: 2026-04-08]
When a CNPC_Neutral_SinnersSacrifice entity dies, the final health value observed via UPDATE is 1, not 0. Delta compression skips the zero-health packet when the entity slot is recycled. Using health==0 as a death signal misses every kill. The correct death signal is `health == 1 && prev_health > 1`. Confirmed across all 34 kills in replay 68182475_4609034.dem -- zero false negatives, zero missed kills.

### [Draft] should_track_snapshot() vs should_track_position() -- [agent: rust-parser, date: 2026-04-08]
`should_track_position()` emits one entry per second for every tracked entity, producing O(match_duration) entries. `should_track_snapshot()` identifies stationary objectives that need a one-time position capture at CREATE -- no per-second tracking needed. Sinners belong in the snapshot category: they never move and per-second position entries would add ~12 identical rows per second from minute 7 onwards. The abstraction generalizes to any future stationary spawn (rune camps, etc.). Adding an entity to `should_track_position()` when it should be in `should_track_snapshot()` silently bloats the positions array without adding value.

### [Draft] CCitadelUserMsg_BossKilled Does Not Fire for Sinners -- [agent: rust-parser, date: 2026-04-08]
Sinner Sacrifice kills do not produce a `CCitadelUserMsg_BossKilled` event. Sinner entity indices fall in the range 3335-3379; none of the 39 `BossKilled` events in replay 55841493 had `entity_killed_class` values matching sinners. Kill detection must be done via entity health UPDATE events using the health==1 death signal. Boss kill message listeners cannot be reused for sinner kill attribution.

### [Draft] Entity Index Recycling on Respawn -- Last-Snapshot-Wins Lookup Pattern -- [agent: rust-parser, date: 2026-04-08]
Deadlock entity slots are recycled on sinner respawn: the same entity index is reused for a new life. This means `entity_index` is not unique across snapshots in a match. When finding the "current" snapshot for an entity index, always iterate `snapshots` in reverse and take the first match (`last-snapshot-wins`). Stale per-entity state (last_health, last_attacker) must be cleared on every CREATE event. Failing to clear on CREATE causes attacker state from a prior life to produce wrong kill attribution on the first death of the new life.

### [Draft] Parser pass-through fields need no service layer -- [agent: backend-python, date: 2026-04-08]
When the parser emits a new top-level field that requires no domain transformation, the backend wiring is two schema additions: add the field to `ParsedMatchResponse` (with a safe default of `[]` or `{}`) and add it to `TransformedMatchData`, then thread it through `MatchDataService.transform()` as a direct assignment. No new service, use case, or mapper is needed. Pydantic/SQLModel deserializes the parser JSON directly into the domain type. The only type-mapping gotcha is Rust `HashMap<u32, K>` -- serde serializes integer keys as JSON strings, so the Python side must use `dict[str, V]`, not `dict[int, V]`.

### [Draft] @deadlock-api/ui-react: integration profile and adoption boundaries -- [agent: frontend-react, date: 2026-04-12]

`@deadlock-api/ui-react@1.2.0` installs and renders cleanly in React 19 + Vite 7 + Tailwind v4. Only 5 of 9 listed components have React bindings (DlItemCard, DlItemGrid, DlItemTooltip, DlProvider, DlShopPanel); the remaining 4 (HeroMinimapIcon, HeroCard, BuildPanel, AbilityOrder) are WIP with no docs or React wrappers. Components self-fetch item data and images from `assets.deadlock-api.com` -- 130+ network requests on a page with two shop panels. Surface colors are overridable via `--dl-bg-*` / `--dl-text-*` CSS vars on `:root`; game-specific colors (tier badges, category accents) and the "Retail Demo" font are hardcoded intentionally. TypeScript 4.9.5 cannot type-check StencilJS types (requires TS 5.x) but Vite builds via esbuild are unaffected. Adoption path: use ready components for item-related features (shop reference, build analysis); skip WIP components and continue building hero/minimap components ourselves.

---

**See [`.claude/knowledge-management.md`](../.claude/knowledge-management.md) for full knowledge management rules, format specifications, and anti-patterns.**

### [Draft] Mid-Boss Citadel Messages: Empty Spawn, Rich Kill, Undocumented Rejuv -- [agent: haste-expert, date: 2026-04-01; updated with inspector data: 2026-04-01]

`CCitadelUserMsg_MidBossSpawned` (ID 349) carries zero fields per valveprotos-rs commit `458c5e1` -- spawn timing must come from `ctx.tick()` at the callback site. Note: the in-game announcement and sound at ~10 minutes suggests this message should carry data; the proto may be stale and should be re-verified at a later commit before implementing.

`CCitadelUserMsg_BossKilled` (ID 347) fires for all boss kills (walkers, patron, mid-boss) and includes `entity_killed_class` (int32) for disambiguation, `objective_team` for team attribution, `gametime` (float), `bosses_remaining`, and `entity_position`. It does not carry player-level kill credit -- `entity_killer` is an ehandle that requires a live entity lookup and may resolve to `None` if the entity is already deleted.

For mid-boss health tracking, the existing `BossTracker` event-anchored pattern (record on `CCitadelUserMessage_Damage` using `victim_health_new`, plus terminal health=0 on entity DELETE) is the correct approach -- extend `BossTracker.is_boss_entity()` to include `CNPC_MidBoss` rather than building a new tracker.

The rejuv buff pickup entity class is `CCitadelItemPickupRejuv` (confirmed via haste-inspector at the tick CNPC_MidBoss health hit 0). This class is specific to the rejuv buff -- other pickups use distinct classes, so no discrimination logic is needed. Fields include `m_nSubclassID` (CUtlStringToken 289368075), `m_eLootType` (1), and `m_nCurrencyValue`. Notably, `m_iHealth` and `m_iMaxHealth` are both 0 -- health is NOT the mechanism for tracking rejuv claims. Individual claim events are signalled via `CCitadelUserMsg_RejuvStatus` (ID 350). On death, CNPC_MidBoss transitions: `m_lifeState` 0 → 1, `m_NPCState` 2 → 10, `m_MoveType` 0 → 9, `m_bRagdollEnabled` false → true, `m_bBeamActive` false → true. `m_bBeamActive` is also present on walker entities -- not mid-boss specific. CNPC_MidBoss base health is 14950 (confirmed at tick 43931).

`CCitadelUserMsg_RejuvStatus` (ID 350) exists for rejuv buff grant events (`killing_team`, `player_pawn`, `user_team`, `event_type`), but `event_type` enum values are undocumented and must be reverse-engineered from a replay. Remaining open tasks: (1) map `RejuvStatus.event_type` values and confirm how many fire per mid-boss kill, (2) confirm `entity_killed_class` integer value for `CNPC_MidBoss` via probe binary, (3) re-verify `MidBossSpawned` proto at latest valveprotos-rs commit (in-game announcement at 10 min suggests fields may exist).

### [Draft] MID_BOSS_CLASS_ID = 8, RejuvStatus enum confirmed, tick rate is 64 not 60 -- [agent: rust-parser, date: 2026-04-08]

**`MID_BOSS_CLASS_ID = 8`** (`confirmed`): `BossKilled.entity_killed_class` is 8 in every mid-boss kill across 3 replays (8 total kills), identified by position (0.0, 0.0, -768.0) = the underground pit, team=4 (neutral), and always followed by `RejuvStatus` events. Add `pub const MID_BOSS_CLASS_ID: i32 = 8;` to `parser/src/entities/constants.rs`. Source: `probe_midboss_runtime`, replays `68175583_527726523.dem`, `68182475_4609034.dem`, `55423930_379917638.dem`, 2026-04-08.

**`RejuvStatus.event_type` enum** (`confirmed`): Three values observed from `CCitadelUserMsg_RejuvStatus` (ID 350) around mid-boss kills:
- 6 = buff granted (fires within ~6s of kill, once per rejuv stack awarded to a player; `killing_team` = the killing team; filter on this for "claim" tracking)
- 7 = buff consumed (player who held a rejuv died and was revived; `killing_team = -1`; fires minutes after the kill)
- 8 = buff expired or last stack gone (`killing_team = -1`; fires at same tick as type=7 or standalone)
Grant count per kill is 2-3, not always exactly 3 (depends on how many players claim the crystal).

**Deadlock tick rate is 64 tps, NOT 60** (`confirmed`): All replay files tested confirm `tick_interval = 0.015625` (1/64). The project memory note "60 ticks/sec" is wrong. Respawn timer probe cross-validates this: 26881 ticks / 64 = 420.0s = exactly 7 minutes. At 60 tps the same delta is 448s, which is inconsistent with the wiki formula. Always use `ctx.tick_interval()` at runtime rather than any hardcoded tick rate constant.

**50% health roar has no Citadel message** (`confirmed`): `CCitadelUserMsgHudGameAnnouncement` (ID 363) fired zero times across 3 replays covering 8 mid-boss kills. No `Roar`, `HealthThreshold`, or `MidBossHealth` proto type exists in `deadlock.rs`. The 50% crossing must be derived from the `health_samples` timeline in the parser output: find the first `HealthSample` where `health / max_health <= 0.5`.

**Wiki respawn timers 7 min / 6 min confirmed exact** (`confirmed`): Validated by `kill_tick -> next_MidBossSpawned_tick` delta across 5 observations: always 26881 ticks (420.0s = 7.00 min) for first death, 23041 ticks (360.0s = 6.00 min) for second death. Third+ death (5 min) is wiki-only, not yet replay-observed.

**Haste `34a3a49` uses async Visitor API** (`confirmed`): `probe_currency_changed.rs` in this worktree is stale -- written against the old sync API, does not compile. The correct template is `replay_parser.rs` or `parse_local.rs`. Key differences from old API: `async fn on_entity/on_packet/on_tick_end/on_cmd`, `type Error = YourError` associated type required, `use prost::Message` (not `use haste::valveprotos::prost::Message`), `ctx.tick()` returns `i32` (not `u32`), `parser.run_to_end().await?` (async, needs `#[tokio::main]`).

### [Draft] CurrencyChanged API Surface: Three Assumptions Invalidated -- [agent: haste-expert, date: 2026-04-01]

`CCitadelUserMessage_CurrencyChanged` (ID 345) confirmed field list from generated types (`parser/target/debug/build/valveprotos-0c71e7db1e5291c4/out/deadlock.rs`): `entindex_hero_pawn: Option<i32>` (tag 1), `currency_type: Option<i32>` (tag 2), `currency_source: Option<i32>` (tag 3), `delta: Option<i32>` (tag 4), `notification: Option<bool>` (tag 5), `entindex_victim: Option<i32>` (tag 6), `victim_pos: Option<CMsgVector>` (tag 7), `playsound: Option<i32>` (tag 8), `ability_id: Option<u32>` (tag 9). `new_value` does NOT exist -- soul balance must be accumulated from `delta`. `userid` does NOT exist -- player identity is `entindex_hero_pawn`. `CCitadelUserMsg_AbilitiesChanged` (ID 309) uses `entindex_purchaser: Option<i32>` (entity index), NOT `purchaser_player_slot`. The probe binary written against `blukai/haste` is incompatible with `deadlock-api/haste` on two counts: (1) sync Visitor must become async with `type Error = anyhow::Error`, (2) player name must be read as `Box<[u8]>` then converted via `String::from_utf8_lossy`. `currency_source` directly feeds the active "lane priority tracking" initiative at zero extra implementation cost -- per-source grouping (LaneCreeps=2, Denies=7) is a filter on events already being iterated.

### [Draft] Serializer Hash Constants Are Stable; the Mid-Boss Panic Is a Missing Match Arm, Not Hash Drift -- [agent: rust-parser, date: 2026-04-14]

Spike `private/plans/spikes/boss-serializer-hash-drift.md` investigated whether `fxhash::hash_bytes(b"...")` constants in `parser/src/entities/constants.rs` can drift away from haste's runtime `entity.serializer().serializer_name.hash` across replays, motivated by the panic at `parser/src/replay_parser.rs:277` on replay 55423930 with hash `16112031173533486177`. **They cannot drift**, and the panic is not a hash problem.

**(i) Hash-by-constant is sustainable, no runtime registry needed** (`confirmed`): `haste::fxhash::hash_bytes` is a `const fn` (`haste_core/src/fxhash.rs:66`), so `const CNPC_MIDBOSS_ENTITY: u64 = fxhash::hash_bytes(b"CNPC_MidBoss")` and a runtime call with the same bytes are byte-identical by construction. Empirically, the extended `probe_all_entity_classes` emitted `CNPC_MidBoss\t16112031173533486177` on all three test replays (55423930, 68175583, 68182475), and all 20 tracked entity-class constants appear byte-identically in every replay's `CDemoSendTables.symbols`. The runtime probe `probe_entity_runtime_hashes` (now saved at `private/engineering/tools/`) logged every distinct `entity.serializer().serializer_name.hash` from CREATE events in all three replays -- 190, 177, and 185 distinct runtime hashes, 100% join rate against the static fxhash table in every case (`190/190`, `177/177`, `185/185`). No exceptions, no divergence. Output files: `private/engineering/tools/class_symbols_{55423930,68175583,68182475}.txt` and `class_runtime_hashes_{55423930,68175583,68182475}.txt`.

**(ii) The panicking entity is the mid-boss itself** (`confirmed`): `16112031173533486177` is literally the fxhash of `CNPC_MidBoss`. Not a neighbour, not a new NPC class, not a rejuv crystal. Reverse lookup against all three replays' symbol tables returns `CNPC_MidBoss` as the only match. The original spike Context claimed the mid-boss tracker stores `boss_name_hash = 11298616958347856125`; that number is actually a fabricated placeholder hardcoded in `frontend/src/domain/matchAnalysis.ts:99` and duplicated through eight frontend/backend fixture files -- it has never matched any real runtime hash and was mistaken for an observed tracker value when the spike was written.

**(iii) Hand-off: parser-internal implementation plan** (no cross-service contract drift): the fix is adding a dispatch arm for `CNPC_MIDBOSS_ENTITY` somewhere in the `get_custom_id` / `get_damage_entity_id` path at `parser/src/replay_parser.rs:213-300`. Mid-boss damage routes through `get_damage_entity_id` (`replay_parser.rs:286`), which short-circuits for `CCitadelPlayerPawn`, `is_boss_entity` (walkers, shrines, patron, guardians), and the Priest slide-trap projectile, then falls through to `get_custom_id` -- whose NPC match arm at `replay_parser.rs:261-282` does not list mid-boss and panics at `replay_parser.rs:277`. Either route mid-boss through the `is_boss_entity` branch (return entity index directly, like other objective bosses) or add a fixed-ID arm to `get_custom_id`. This is an implementation plan, not a discovery plan -- parser JSON output schema is unchanged, so `backend-api.md`, `frontend/src/domain/midBoss.ts`, and the backend transform layer need no contract work. The `11298616958347856125` magic number in fixtures is pre-existing cruft that should be revisited during the implementation plan's test pass but is out of scope for the spike.

**(iv) Secondary recommendation**: when a new tracker introduces a new entity class constant, every dispatch site that matches on `serializer_name.hash` must be updated in the same change. The mid-boss tracker added `CNPC_MIDBOSS_ENTITY` and the `on_entity` routing but did not add any arm to `get_custom_id` / `get_damage_entity_id`, and no test exercises mid-boss damage routing, so the gap was invisible until a replay with mid-boss damage events reached the parser. Consider a static analysis helper or a compile-time macro that forces exhaustive dispatch for a central `TrackedEntity` enum, or at minimum add a regression test that asserts `get_custom_id` returns successfully for every constant in `entities::constants`. Source: `private/plans/spikes/boss-serializer-hash-drift.md`, probe outputs in `private/engineering/tools/`, 2026-04-14.

### [Draft] STrooperFOWEntity schema + pos_xy / ent_idx encoding -- [agent: rust-parser, date: 2026-04-19]

The server-authoritative lane-trooper minimap vector `CCitadelTrooperMinimap.m_vecFOWEntities` holds 192 `STrooperFOWEntity` slots with only three fields, NOT the richer FOW schema the name might suggest. Full schema (Phase 1 dump, `trooper_minimap_68175583_phase1.log:5488-5495`):

```
CCitadelTrooperMinimap:
  m_timeLastUpdate: GameTime_t
  m_vecFOWEntities: CUtlVectorEmbeddedNetworkVar< STrooperFOWEntity >   # capacity 192
STrooperFOWEntity:
  m_nPositionXY: uint16
  m_nEntIndex: CEntityIndex
  m_nTeam: int8
```

**Two non-obvious encodings verified empirically on replay 68175583 (Phase 2/3):**

1. `m_nEntIndex` is emitted as `(CNPC_Trooper.entity_index << 1)`. Low bit is always 0 across 345,879 occupied samples. To join a minimap slot to its underlying entity, right-shift by 1: `cnpc_idx = minimap_slot.ent_idx >> 1`. A naive direct lookup fails at 0% join rate and the join-hit-rate jumps to 72% after the shift.

2. `m_nPositionXY` is a packed uint16 where `hi-byte = y_cell`, `lo-byte = x_cell`, both unsigned uint8. World coord = `(byte - 128) * 84`. Linear regression on 75,164 live pairs against `CNPC_Trooper.CBodyComponent` positions: slope 83.91 / 83.97, r^2 0.9950 / 0.9931. Covers an effective range of [-10752, +10752] -- wider than the [-8960, +8960] range hardcoded in haste's deadlock coord helpers, so scaling by `17920/256 = 70` gives a 1200-unit-off decoder (rejected). Position fidelity: p50 = 42 world units, p95 = 80, both well under half a lane width.

**Slot lifecycle:** vector is pre-allocated to 192 slots on tick 1 and never compacts. Slots recycle heavily as troopers die -- up to 167 ent_idx changes on a single slot in a 40-min match -- and `m_nTeam` is also re-assigned on recycle (not a stable per-slot attribute). `m_timeLastUpdate` updates rarely enough to be ignored unless a probe explicitly watches it.

**Why this matters:** if a future feature needs lane-creep snapshots without paying the per-trooper subscription cost (84x fewer `on_entity` callbacks than `CNPC_Trooper`), this entity gives position + team + handle for free -- but not lane, health, or visibility bits. Anyone building from this should: (a) right-shift the ent_idx, (b) use the `(byte - 128) * 84` decoder, (c) join through `CNPC_Trooper` for any missing fields. Artifacts: `private/engineering/tools/probe_trooper_minimap.rs`, `private/engineering/tools/analyze_trooper_minimap.py`, `private/engineering/samples/trooper_minimap_crosscheck.md`.

### [Draft] `STeamFOWEntity` is the real per-team FOW source, not `STrooperFOWEntity` -- [agent: rust-parser, date: 2026-04-19]

Deadlock's replay exposes TWO FOW-related embedded-vector entities that share naming but live on different parents with different schemas (Phase 1 dump, `trooper_minimap_68175583_phase1.log:5497-5508`):

- `STrooperFOWEntity` (3 fields: pos, ent_idx, team) -- child of the singleton `CCitadelTrooperMinimap`. Lane-troopers only. Has NO visibility bits, NO lane, NO health, NO class discriminator.
- `STeamFOWEntity` (12 fields: `m_nPositionX/Y`, `m_nEntIndex`, `m_nTeam`, `m_eClass`, `m_iLane`, `m_eHeight: EMinimapHeight`, `m_bVisibleOnMap: bool`, `m_bBackdoorProtectionActive: bool`, `m_nTickHidden: GameTick_t`, `m_strEntityName: CUtlString`, `m_nHealthPercent: uint8`) -- child of `CCitadelTeam.m_vecFOWEntities`. Per-team vector (one per team). This is the entity that carries the "what can team X see on the minimap right now?" truth.

Any feature that asked for per-team lane/vision/awareness ("team was blind to the push", "which lane did team X commit to", "fog of war rolling window") should target `STeamFOWEntity`, NOT `STrooperFOWEntity`. The `CCitadelTeam` parent means you get one independent 12-field vector per team, so comparing `m_bVisibleOnMap` for the same `m_nEntIndex` across the two team entities directly yields per-team visibility divergence.

**Not yet validated** (deferred from the trooper-minimap discovery): (a) whether `m_bVisibleOnMap` actually differs between the two `CCitadelTeam` instances for the same underlying entity, (b) which entity classes the pool covers (lane troopers? heroes? neutrals? objectives?), (c) update volume vs the `CNPC_*` per-entity subscription path. A dedicated discovery spike on `CCitadelTeam.m_vecFOWEntities` is warranted before any per-team FOW feature work. Source: `private/plans/discovery/trooper-minimap-fow-tracking.md` Phase 1 side-finding, 2026-04-19.

---

## u64 Identifiers Crossing JSON to JavaScript Must Ship as Strings

**Date discovered:** 2026-04-14, boss_name_hash JS precision-loss fix
**Impact:** Parser (serde wire format), Backend (Pydantic typing), Frontend (TS interface + lookup maps)
**Status:** validated | pattern-identified

`BossSnapshot.boss_name_hash` was declared `pub boss_name_hash: u64` in `parser/src/domain/boss.rs` and serialized via `#[derive(Serialize)]`, so serde emitted a bare JSON number. Every real `fxhash::hash_bytes(class_name)` value is an 18-20 digit u64 -- always above JavaScript's `Number.MAX_SAFE_INTEGER` (`2^53 - 1 = 9007199254740991`). The hash was silently truncated to the nearest IEEE 754 double on `JSON.parse`, the trailing digits collapsed to `000`, and `frontend/src/domain/boss.ts`'s `BOSS_NAME_HASH_MAP` lookup missed for every boss type. Boss tooltips in the UI degraded to `Boss #<truncated-number>` and Sankey diagrams collapsed distinct classes into the same node. The bug went undetected for weeks because the TypeScript interface typed the field as `number` and the lookup fallback was silent.

**Root cause:** wire-format mismatch -- a value type whose realistic range exceeds 53 bits cannot survive a JSON number transport when one of the consumers is JavaScript. The fix is a one-character change in the wire format (`u64` -> `String`) plus a typing cascade (`int` -> `str` on the Python side, `number` -> `string` on the TS side, dict-key types updated). No arithmetic is performed on these hashes anywhere in the codebase, so the type change is behavior-preserving aside from the equality semantics (which are identical).

**The in-codebase precedent already existed.** `parser/src/domain/mid_boss.rs:8` had been declaring `pub boss_name_hash: String` since the mid-boss tracker landed -- string-on-wire was the established convention for this exact field, and `BossSnapshot` simply hadn't been updated to match. Failing to propagate the convention across sibling structs is a recurring pattern worth watching for.

**Pattern -- candidates to audit when introducing a new u64 ID at a service boundary:**

| Field type | Safe as JSON number? | Reasoning |
|---|---|---|
| `entity_index` (u16) | yes | Capped at 16 bits |
| `tick` count (u32) | yes | Realistic match never exceeds 2^32 |
| `match_id` (u64) | **no** | Steam IDs are 17-digit, must be string-on-wire |
| `steam_id_64` | **no** | Same as above |
| `*_name_hash` (fxhash u64) | **no** | Always 18-20 digits |
| `*_id` from any external system | **case-by-case** | Default to string on the wire if the upstream type is u64 |

**Key Takeaway:**
Default to string-on-wire for any u64 ID at a JSON service boundary that has a JavaScript consumer. The TypeScript type system makes the truncation invisible at compile time, and the lookup-miss fallback can swallow the bug at runtime. If you must ship a number, prove the field cannot exceed `2^53 - 1` and document the bound.

**Related Docs:**
- `private/plans/fixes/boss-name-hash-js-precision-loss.md` -- the fix plan
- `private/plans/spikes/boss-serializer-hash-drift.md` -- the spike that surfaced this bug as a side-finding while investigating the mid-boss panic
- `parser/src/domain/mid_boss.rs:8` -- the pre-existing string-on-wire precedent

**When to Reference:**
- Adding a new `*_hash` or `*_id` field to a parser struct that flows to the frontend
- Reviewing a contract spec change that introduces a new u64
- Any `Number.MAX_SAFE_INTEGER` discussion
- Auditing the parser-output / backend-api contracts for similar fields

**Prevention:**
- Frontend types for any cross-service ID must be `string`, never `number`, unless the bound is provably under 2^53
- Contract specs should record the wire type explicitly: `u64 as decimal string`, not bare `int`
- When introducing a new sibling struct, check whether any existing struct in the same domain already converted to string-on-wire and follow that precedent
