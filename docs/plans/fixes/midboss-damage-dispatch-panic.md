# Mid-Boss Damage Dispatch Panic Fix

> **File location:** `private/plans/fixes/`

## Context

Parsing a replay that contains mid-boss damage events panics the parser at `parser/src/replay_parser.rs:277` with `Unknown entity - Index: N, Hash: 16112031173533486177`. The mid-boss entity class `CNPC_MidBoss` is tracked by `mid_boss_tracker.rs` but was never wired into the damage dispatch path, so the first real replay to exercise mid-boss damage crashes.

**Discovered via:** spike `private/plans/spikes/boss-serializer-hash-drift.md` (2026-04-14). The spike confirmed the panicking hash `16112031173533486177` is the true `fxhash(b"CNPC_MidBoss")` from `entities/constants.rs`, not a drifted or unknown hash.
**Related:** `private/plans/spikes/boss-serializer-hash-drift.md`
**Overall confidence:** `confirmed` -- spike's 3-replay probe (`private/engineering/tools/class_symbols_*.txt`) proved the panicking hash is byte-identical to `fxhash(b"CNPC_MidBoss")` and that all 20 tracked entity-class constants are stable across replays. Fix mirrors an existing in-codebase short-circuit (priest slide-trap), so the implementation pattern is also validated. Nothing speculative.

---

## Symptom

Parser panics partway through parsing any replay that records damage dealt to or from the mid-boss. On replay `55423930` the parser logs:

```
thread 'tokio-runtime-worker' panicked at parser/src/replay_parser.rs:277:
Unknown entity - Index: <index>, Hash: 16112031173533486177
```

`16112031173533486177` is `fxhash::hash_bytes(b"CNPC_MidBoss")`, verified against the `CDemoSendTables` symbol tables of replays 55423930, 68175583, and 68182475 during the spike (`private/engineering/tools/class_symbols_*.txt`).

**Reproduction:**
1. Start the stack: `scripts/wt start midboss --full` (from `/home/lifted/Code/dashjump/`)
2. Trigger a parse via the backend endpoint: `curl http://localhost:<wt-backend-port>/match/analysis/55423930`
3. Observe parser container logs -- expect the panic above, and expect the backend to receive a 500 from the parser with no `damage` or `positions` payload

---

## Root Cause

`get_damage_entity_id` at `parser/src/replay_parser.rs:285-300` routes damage-event entities through a short pipeline:

1. `CCitadelPlayerPawn` -- dispatch to `get_custom_id` (returns lobby player slot)
2. `BossTracker::is_boss_entity(hash)` -- return `entity.index()` (boss-like objectives get per-instance identity)
3. `CPROJECTILE_PRIEST_SLIDETRAP_ENTITY` -- return `entity.index()` (projectile special case)
4. Fall through to `get_custom_id` -- fixed-ID match on NPC classes, panics on anything unknown

The mid-boss is not a player pawn, is not listed in `BossTracker::is_boss_entity` (`parser/src/tracking/boss_tracker.rs:63-69` hardcodes guardian/shrine/walker/base_guardian/patron), and is not the priest slide-trap, so every mid-boss damage event falls through to `get_custom_id`. The NPC match at `parser/src/replay_parser.rs:261-282` has fifteen arms and no entry for `CNPC_MIDBOSS_ENTITY` -- the `_ =>` arm panics.

The mid-boss tracker (`parser/src/tracking/mid_boss_tracker.rs`) was added in a prior change that wired `CNPC_MIDBOSS_ENTITY` into `on_entity`'s CREATE/UPDATE routing (`replay_parser.rs:498-508`) but never touched `get_damage_entity_id` or `get_custom_id`. No existing test parses a replay with mid-boss damage, so the gap was invisible.

**Confidence:** `confirmed`

Spike `boss-serializer-hash-drift.md` proved via three-replay probe runs that:
- The panicking hash equals `fxhash(b"CNPC_MidBoss")` exactly (reverse lookup against every replay's symbol table)
- All 20 tracked entity-class constants in `entities/constants.rs` are byte-identical across replays
- Runtime `entity.serializer().serializer_name.hash` joins 100% against the static fxhash table (190/190, 177/177, 185/185)

So the panicking value is neither drift nor corruption -- it is the mid-boss itself hitting an incomplete dispatch match.

---

## Scope

| Service  | Involved | Agent            |
|----------|----------|------------------|
| Parser   | yes      | `rust-parser`    |
| Backend  | no       | `backend-python` |
| Frontend | no       | `frontend-react` |

**Contract change:** no

Parser JSON output schema is unchanged -- mid-boss damage already surfaces as `damage` map entries keyed by the attacker/victim id pair; this fix only keeps the parser from panicking before those entries are produced. `parser-output.md`, `backend/app/domain/parsed_match.py`, `frontend/src/domain/matchAnalysis.ts` need no updates.

---

## Fix

Add a short-circuit for `CNPC_MIDBOSS_ENTITY` in `get_damage_entity_id`, mirroring the existing priest slide-trap special case. This gives mid-boss damage events per-instance identity via `entity.index()` (there is only one mid-boss alive at a time but entity indices are reused across spawn cycles, so per-index keying is the right pattern). This approach deliberately avoids extending `BossTracker::is_boss_entity` because that predicate is also called by `on_entity` CREATE routing at `replay_parser.rs:512`, and adding mid-boss there would double-route CREATEs through both `boss_tracker.handle_boss_create` and `mid_boss_tracker.observe_entity`.

**Files to change:**

- `parser/src/replay_parser.rs` (`get_damage_entity_id`, lines 285-300) -- add an `if hash == CNPC_MIDBOSS_ENTITY { return entity.index() as u32; }` branch alongside the existing priest slide-trap short-circuit. Add `CNPC_MIDBOSS_ENTITY` to the `use crate::entities::constants::...` import list at the top of the file if it is not already imported there.
- `parser/src/replay_parser/tests.rs` (or `parser/src/replay_parser.rs` inline `#[cfg(test)] mod tests;` per `.claude/rules/parser/CLAUDE.md` Testing Conventions) -- add the regression test described in Testing below.

**Out of scope:**

- **Extending `BossTracker::is_boss_entity` to include mid-boss.** Reason: would incorrectly double-route CREATE events through `boss_tracker.handle_boss_create` in addition to the existing `mid_boss_tracker.observe_entity` call. Mid-boss lifecycle is intentionally owned exclusively by `mid_boss_tracker`.
- **Adding mid-boss to the `get_custom_id` NPC match arm.** Reason: `get_custom_id` returns fixed class IDs that collide across instances; mid-boss damage wants per-instance (entity index) identity.
- **Refactoring the NPC dispatch pipeline into a single exhaustive match or macro-generated lookup.** Reason: worthwhile but separate scope; the secondary Learnings recommendation in the spike already flagged it.
- **Fixing `boss_name_hash` JS precision loss.** Reason: separate bug, tracked in `private/plans/fixes/boss-name-hash-js-precision-loss.md`.

---

## Acceptance Criteria

*Verifiable by test suite:*
- [ ] Regression test added that exercises the mid-boss dispatch path and fails against the unpatched code (asserts `get_damage_entity_id` returns `Ok(entity.index() as u32)` for a `CNPC_MIDBOSS_ENTITY` hash)
- [ ] Exhaustiveness guard test added that iterates every `*_ENTITY` constant in `parser/src/entities/constants.rs` and asserts `get_damage_entity_id` returns `Ok` for each
- [ ] `cargo test` and `cargo clippy` both clean inside `dashjump-parser` container

*Verifiable by manual check (see Verification):*
- [ ] No `panicked at parser/src/replay_parser.rs` line in parser container logs after parsing replay 55423930 via the backend endpoint
- [ ] Backend returns 200 with non-empty `damage` field for replay 55423930

*Process:*
- [ ] Project Definition of Done met: tests, observability, conventions, security

---

## Testing

### Regression test *(required)*

- **Test file:** `parser/src/replay_parser/tests.rs` (create per `.claude/rules/parser/CLAUDE.md` Testing Conventions if not present; declare `#[cfg(test)] mod tests;` at the bottom of `replay_parser.rs`)
- **What it asserts:** Given an `Entity` whose `serializer_name.hash == CNPC_MIDBOSS_ENTITY` and a non-zero entity index, `get_damage_entity_id` returns `Ok` with `entity.index() as u32`. Must cover both attacker and victim positions in `push_damage_record`.
- **Why it would have caught the bug:** The existing test suite exercises `boss_tracker` and `mid_boss_tracker` in isolation but never runs the damage-routing path for a mid-boss entity. A direct assertion on `get_damage_entity_id` closes that gap at the smallest unit of code that was broken.
- **Preferred approach:** unit test using a mock `Entity` constructor if one exists; otherwise an integration test that parses a fixture replay known to contain mid-boss damage. Replay `55423930_4609034.dem` (cached at `parser/src/replays/` in the main worktree) is the canonical reproduction case and makes a good fixture for an integration-style test.

Additionally, add an exhaustiveness guard that would catch *future* occurrences of this same bug class:

- **Test name suggestion:** `get_damage_entity_id_does_not_panic_for_any_tracked_entity_constant`
- **Assertion:** iterate every `*_ENTITY` constant declared in `parser/src/entities/constants.rs` and assert `get_damage_entity_id` returns `Ok` for a mock entity carrying that hash. New tracked constants will auto-enroll.
- **Fallback if no mock `Entity` constructor exists:** the haste `Entity` type may not expose a public test constructor. If that is the case, do **not** invent one or refactor `get_damage_entity_id` to take a hash directly -- both expand scope. Instead, skip the exhaustiveness guard for this fix and file a follow-up spike (`private/plans/spikes/parser-dispatch-exhaustiveness-test-shape.md`) to design a testable seam. The point regression test (above) is sufficient to land this fix; the guard is a nice-to-have.

### Existing tests affected

None -- no current test exercises `get_damage_entity_id` for mid-boss, so none will regress.

---

## Verification

| Service | Command | Expected |
|---------|---------|----------|
| Parser  | `docker compose exec dashjump-parser cargo test` | All pass, including the new mid-boss dispatch regression test |
| Parser  | `docker compose exec dashjump-parser cargo clippy` | Clean |

**Manual check:** With the stack running (`scripts/wt start midboss --full`), hit `GET http://localhost:<wt-backend-port>/match/analysis/55423930` and confirm the parser container logs contain no `panicked at parser/src/replay_parser.rs` line and the backend returns a 200 with a non-empty `damage` field covering the mid-boss area in match time. Spot-check that at least one entry in the damage map keys on an entity index that matches a `CNPC_MidBoss` create/delete event in the parser logs.

---

## Learnings

The mid-boss tracker was added without any cross-reference check against the damage dispatch pipeline, and no regression test covered the happy path for a complete feature. This is a process-level gap: when a new entity class constant is added to `entities/constants.rs`, every dispatch site that branches on `serializer_name.hash` must be updated in the same change. The spike already drafted a learning on this topic -- the implementing agent should confirm the draft is still accurate after this fix lands and add a one-line follow-up if the exhaustiveness guard test shape proves useful enough to standardize.

- [ ] Confirm spike-era learning draft in `private/learnings.md` ## Drafts is still accurate post-fix; if not, update or replace
- [ ] Pattern identified: "new tracker = audit every dispatch site that matches on entity hash"
- [ ] Evidence cited: spike `private/plans/spikes/boss-serializer-hash-drift.md` (3-replay probe coverage proving hash stability) + this fix's regression test demonstrating the missing dispatch arm

---

## Execution Order

1. Implement the short-circuit in `get_damage_entity_id`; add the `CNPC_MIDBOSS_ENTITY` import if needed
2. Write the regression test(s) in `replay_parser/tests.rs` (point test required; exhaustiveness guard if mock `Entity` constructor exists, otherwise file the follow-up spike noted in Testing)
3. Run `cargo test` and `cargo clippy` inside the parser container; fix any issues
4. Perform the manual check on replay 55423930 via the backend endpoint
5. Run `test-auditor` and `code-reviewer` agents against the unstaged diff
6. Confirm or update the spike-era learnings draft
7. User review -- commit
