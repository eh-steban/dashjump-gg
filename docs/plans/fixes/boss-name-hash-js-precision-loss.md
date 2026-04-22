# `boss_name_hash` JS Precision Loss Fix

> **File location:** `private/plans/fixes/`

## Context

`BossSnapshot.boss_name_hash` is serialized by the parser as a JSON `u64` number. Every real fxhash value is 18-20 decimal digits, well past JavaScript's 53-bit integer precision ceiling, so by the time the frontend parses the response the hash is silently truncated to an IEEE 754 double. This corrupts the `BOSS_NAME_HASH_MAP` lookup at `frontend/src/domain/boss.ts:42-48`, which breaks boss-type labeling in the UI and causes duplicate-key collisions in downstream Sankey diagrams. Mid-boss already ships its hash as a JSON `String` (`parser/src/domain/mid_boss.rs:8`, `backend/app/domain/mid_boss.py:49`), so the fix is to bring `BossSnapshot` into alignment with that existing precedent.

**Discovered via:** spike `private/plans/spikes/boss-serializer-hash-drift.md` (2026-04-14), while reverse-engineering the frontend `BOSS_NAME_HASH_MAP` to identify the origin of the fake `11298616958347856125` hash the spike's Context referenced.
**Related:** `private/plans/spikes/boss-serializer-hash-drift.md`, `private/plans/fixes/midboss-damage-dispatch-panic.md` (sibling bug from the same spike)
**Overall confidence:** `confirmed` -- three converging lines of evidence: (1) IEEE 754 doubles cannot losslessly hold any integer above `2^53`, and every real fxhash in the probe output (`private/engineering/tools/class_symbols_55423930.txt`) is an 18-20 digit u64, (2) the in-codebase precedent `parser/src/domain/mid_boss.rs:8` already uses string-on-wire for the same kind of hash and works correctly through the same parser->backend->frontend path, (3) the contract spec (`parser-output.md:127-133`) already carries the correct decimal values, so the fix is mechanical alignment rather than fresh discovery. No new probe runs required.

---

## Symptom

`getBossDisplayName` at `frontend/src/domain/boss.ts:50-60` falls through to the `Boss #${boss.boss_name_hash}` fallback for every boss type because the lookup against `BOSS_NAME_HASH_MAP` fails. Boss tooltips and Sankey diagrams then label every objective as `Boss #<truncated-number>`, and multiple distinct classes can round to the same JS double and collapse into one Sankey node.

The `BOSS_NAME_HASH_MAP` keys in `boss.ts:43-47` all end in `000`, which is the low-bit signature of double-precision truncation of a u64 that exceeds 2^53 (~9.007e15). Every key in the map is wrong on two axes at once: it is both double-truncated **and** populated from a stale pre-haste-upgrade hash that no longer matches any real serializer. Compare the true `fxhash::hash_bytes` values from the parser probe (`private/engineering/tools/class_symbols_55423930.txt`) against the map keys:

| Class | True u64 | Map key in `boss.ts` |
|---|---|---|
| `CNPC_TrooperBoss` | `12946736302082733589` | `10648152268083397000` |
| `CNPC_Boss_Tier2` | `1942975293714691302` | `14993025469191344000` |
| `CNPC_BarrackBoss` | `793562361056549792` | `7661004720742107000` |
| `CCitadel_Destroyable_Building` | `8292725763874089450` | `3692976131341581000` |
| `CNPC_Boss_Tier3` | `7814756300278693755` | `9121244462627342000` |

The TODO comment at `boss.ts:35-41` records the date the UI broke (~2026-03-03) and correctly identifies the class of problem even though the spike Context had the wrong theory about the mechanism.

**Reproduction:**
1. Start the stack: `scripts/wt start midboss --full`
2. Load a match in the UI that includes guardians, walkers, shrines, and base guardians
3. Hover a boss marker -- the label reads `Boss #<large-number>` instead of `Guardian - Lane 2` (or similar)
4. Inspect a Sankey diagram that aggregates damage by boss type -- expect multiple classes to merge into one node, or expect node labels to be numeric fallbacks
5. Open devtools and `console.log(snapshot.boss_name_hash)` against a known guardian -- expect a JS number whose trailing digits are `000`, confirming precision loss

---

## Root Cause

Two overlapping defects:

**(1) Wire format.** `parser/src/domain/boss.rs:11` declares `pub boss_name_hash: u64` and `#[derive(Serialize)]`, so serde emits the hash as a JSON number. `backend/app/domain/boss.py:7` holds it as `int` (Python handles arbitrary precision, no corruption), and then FastAPI's `JSONResponse` re-emits it as a JSON number. When the frontend calls `JSON.parse` on the response, any integer above `Number.MAX_SAFE_INTEGER` (`2^53 - 1 = 9007199254740991`) is silently coerced to the nearest representable double. Every real fxhash exceeds that ceiling, so every `boss_name_hash` arrives in TypeScript already corrupted -- `BossSnapshot.boss_name_hash` is typed `number` at `frontend/src/domain/boss.ts:4` and the lossy double is what populates it.

**(2) Stale map.** Orthogonal to (1), the `BOSS_NAME_HASH_MAP` at `frontend/src/domain/boss.ts:42-48` was populated from a previous-generation parser build (or hand-typed from an older haste fork whose fxhash output differed). The current values do not match any live u64 in the probe output. Even if we fixed the wire format tomorrow, the map would still need to be rewritten with the true decimal strings. The contract spec at `private/specs/contracts/parser-output.md:127-133` already documents the correct u64 values -- the frontend just never resynced.

Bug (1) alone would be sufficient to break the UI; bug (2) ensures that even after a precision fix the map lookups would still miss until the keys are replaced.

**Confidence:** `confirmed`

Three-replay probe evidence from the spike proves every real u64 hash is distinct, byte-stable, and always exceeds `2^53`. The contract spec already carries the correct decimal values. The JS precision-loss mechanism is a well-known property of IEEE 754 doubles.

---

## Scope

| Service  | Involved | Agent            |
|----------|----------|------------------|
| Parser   | yes      | `rust-parser`    |
| Backend  | yes      | `backend-python` |
| Frontend | yes      | `frontend-react` |

**Contract change:** yes

The parser output contract at `private/specs/contracts/parser-output.md:107-111` currently types `boss_name_hash` as `int`. It must change to `string` for consistency with the mid-boss hash (`parser-output.md:233`) and to match the new wire format. The "Boss Type Identification" table at `parser-output.md:127-133` already carries the correct decimal values and needs no value changes -- only the header column label `(u64)` should be updated to `(u64 as string)` to clarify the wire type.

### Contract pre-check *(must complete before Phase 0 ships)*

Before editing the contract spec or any code, open `private/specs/contracts/parser-output.md:127-133` and confirm **all five** of these decimal values are present in the Boss Type Identification table, byte-for-byte:

| Class | Required decimal value |
|---|---|
| `CNPC_TrooperBoss` (Guardian) | `12946736302082733589` |
| `CNPC_Boss_Tier2` (Walker) | `1942975293714691302` |
| `CNPC_BarrackBoss` (Base Guardian) | `793562361056549792` |
| `CCitadel_Destroyable_Building` (Shrine) | `8292725763874089450` |
| `CNPC_Boss_Tier3` (Patron) | `7814756300278693755` |

If any value is missing, mismatched, or carries an old truncated form, **stop** and update the spec to the values above (verified from `private/engineering/tools/class_symbols_55423930.txt`) before continuing. The downstream backend constants and frontend map keys all derive from this table -- a stale spec row guarantees the fix re-introduces the same bug under a different mask.

- [ ] Contract pre-check completed (5/5 values verified in `parser-output.md:127-133`)
- [ ] `private/specs/contracts/parser-output.md` -- change `boss_name_hash` row in the BossSnapshot table from `int` to `string`; update the Boss Type Identification table header to say the values are transported as decimal strings

---

## Fix

Align `BossSnapshot.boss_name_hash` with the existing mid-boss precedent: parser serializes as `String`, backend reads as `str`, frontend typed as `string`, lookup map rekeyed on the true decimal strings. No hash arithmetic is performed anywhere in the codebase, so switching from numeric to string type is behavior-preserving -- only string-equality comparisons are needed.

**Files to change:**

### Parser (`rust-parser`)

- `parser/src/domain/boss.rs` -- change `pub boss_name_hash: u64` to `pub boss_name_hash: String` (mirrors `parser/src/domain/mid_boss.rs:8`)
- `parser/src/tracking/boss_tracker.rs:92` -- `boss_name_hash: hash` currently assigns a `u64`; change to `boss_name_hash: hash.to_string()`
- `parser/src/tracking/boss_tracker/tests.rs:13` -- update the `boss_name_hash: 0` literal to `boss_name_hash: "0".to_string()` (or whichever test fixture value is used); search the file for every `boss_name_hash:` assignment and update
- `parser/src/replay_parser.rs` -- search for any other construction site of `BossSnapshot` or uses of `.boss_name_hash` that need `.to_string()` added or removed

### Backend (`backend-python`)

- `backend/app/domain/boss.py:7` -- change `boss_name_hash: int` to `boss_name_hash: str`
- `backend/app/services/lane_pressure_service.py:71-75` -- change the five `BOSS_HASH_*` module constants from `int` literals to string literals (values unchanged, just wrap in quotes):
  ```python
  BOSS_HASH_GUARDIAN = "12946736302082733589"       # CNPC_TrooperBoss
  BOSS_HASH_WALKER = "1942975293714691302"          # CNPC_Boss_Tier2
  BOSS_HASH_BASE_GUARDIAN = "793562361056549792"    # CNPC_BarrackBoss
  BOSS_HASH_SHRINE = "8292725763874089450"          # CCitadel_Destroyable_Building
  BOSS_HASH_PATRON = "7814756300278693755"          # CNPC_Boss_Tier3
  ```
- `backend/app/services/lane_pressure_service.py` -- all existing `==` comparisons and `.setdefault((s.team, s.lane, s.boss_name_hash), [])` tuple keys keep working unchanged; Python equality and hashing work identically for string keys
- `backend/tests/test_match_data_service.py`, `backend/tests/test_match_api.py`, `backend/tests/test_parsed_matches_repo.py`, `backend/tests/application/use_cases/test_analyze_match.py` -- every fixture that currently uses `boss_name_hash=<int>` or `"boss_name_hash": <int>` must become a string. Also audit every hardcoded value -- some fixtures may carry the stale/fake `11298616958347856125` value; replace with a real decimal string from the contract spec table

### Frontend (`frontend-react`)

- `frontend/src/domain/boss.ts:4` -- change `boss_name_hash: number` to `boss_name_hash: string`
- `frontend/src/domain/boss.ts:42-48` -- rewrite `BOSS_NAME_HASH_MAP` keys with the true decimal strings:
  ```ts
  const BOSS_NAME_HASH_MAP: Record<string, string> = {
    '12946736302082733589': 'Guardian',      // CNPC_TrooperBoss, custom_id=21
    '1942975293714691302':  'Walker',        // CNPC_Boss_Tier2, custom_id=28
    '793562361056549792':   'Base Guardian', // CNPC_BarrackBoss, custom_id=26
    '8292725763874089450':  'Shrine',        // CCitadel_Destroyable_Building, custom_id=27
    '7814756300278693755':  'Patron',        // CNPC_Boss_Tier3, custom_id=29
  };
  ```
- `frontend/src/domain/boss.ts:51` -- `getBossDisplayName` currently calls `String(boss.boss_name_hash)` to coerce the number to a string key. Once the type is already `string`, **drop the `String()` wrapper** and use `boss.boss_name_hash` directly. The wrapper is dead code under the new type and obscures the fact that the value is already string-typed; defensive coercion against a typed string is misleading, not safer.
- `frontend/src/domain/boss.ts:35-41` -- delete the stale TODO comment; it is the historical record of this bug and no longer applies once the fix ships
- `frontend/src/domain/matchAnalysis.ts:99` -- the hardcoded `boss_name_hash: "11298616958347856125"` fixture value is fake (never matched any real hash). **Default replacement: `"12946736302082733589"` (Guardian / `CNPC_TrooperBoss`)** -- this is the most generic boss type and any test that doesn't care about the specific class will continue to make sense. If the surrounding fixture context clearly represents a different class (e.g. lane 0 walker, patron damage), pick the matching value from the BOSS_NAME_HASH_MAP rewrite above instead
- Any other frontend test fixtures using the old numeric type or the fake `11298616958347856125` value (grep the frontend for both patterns)

**Out of scope:**

- **Changing `custom_id` or `entity_index` types.** Reason: those fields are not affected by precision loss; `custom_id` is a small int and `entity_index` is a u16 slot number. Keep them as-is.
- **Introducing a parser-owned `boss_type` enum string alongside or in place of `boss_name_hash`.** Reason: worth considering as a follow-up once this fix is verified; out of scope here because it changes the semantic shape of the contract rather than just the wire format.
- **Fixing the mid-boss damage dispatch panic.** Reason: tracked separately in `private/plans/fixes/midboss-damage-dispatch-panic.md`.
- **Adding `entity_index` to `PlayerPosition` to disambiguate multi-instance shrines/base guardians in the positions stream.** Reason: the display-path instance collision for `BossSnapshot` is already handled by `getBossDisplayName` appending `entity_index` for Base Guardian and Shrine types (`boss.ts:56-58`). The `PlayerPosition` stream may have its own collision issue but it is unrelated to precision loss and should be investigated separately.
- **Rewriting `lane_pressure_service.py`'s `BOSS_HASH_*` lookups to use string class names instead of decimal hashes.** Reason: a cosmetic improvement that expands the diff without changing behavior; decimal-string hashes match the contract spec and are fine.

---

## Acceptance Criteria

*Verifiable by test suite:*
- [ ] Parser regression test asserts `BossSnapshot` serializes `boss_name_hash` as a JSON string (quoted, not bare number)
- [ ] Backend regression test asserts a parser response with `"boss_name_hash": "12946736302082733589"` deserializes into a `BossSnapshot` whose `boss_name_hash` equals `BOSS_HASH_GUARDIAN`
- [ ] Frontend regression test asserts `getBossDisplayName` returns the correct label for each of the five boss types given the real decimal-string hashes
- [ ] `BossSnapshot.boss_name_hash` is statically typed `string` in TypeScript (compile-time check via `npm run tsc`)
- [ ] `cargo test`, `pytest`, and `npm test` all pass inside their containers
- [ ] `cargo clippy`, `ruff check`, `mypy app`, and `npm run tsc` all clean

*Verifiable by manual check (see Verification):*
- [ ] On a real parsed match, every boss tooltip in the UI reads `"<Type> - Lane <N>"` (or `"<Type> - Lane <N> (<entity_index>)"` for shrines/base guardians), never `"Boss #<number>"`
- [ ] Sankey diagrams that aggregate by `boss_name_hash` render five distinct nodes (Guardian, Walker, Base Guardian, Shrine, Patron) with no collapsed duplicates
- [ ] Devtools network tab shows `boss_name_hash` fields as quoted JSON strings in the `/match/analysis/<id>` response

*Process:*
- [ ] Contract spec `private/specs/contracts/parser-output.md` updated to type `boss_name_hash` as `string`
- [ ] Project Definition of Done met: tests, observability, conventions, security

---

## Testing

### Regression test *(required)*

Three regression tests, one per service, each failing against the unpatched code and passing after the fix:

**Parser:**
- **Test file:** `parser/src/domain/boss/tests.rs` (create per `.claude/rules/parser/CLAUDE.md` Testing Conventions) or extend an existing `boss_tracker/tests.rs` case
- **What it asserts:** A constructed `BossSnapshot` with `boss_name_hash = "12946736302082733589".to_string()` serializes to JSON containing `"boss_name_hash":"12946736302082733589"` (note: quoted, not bare numeric)
- **Why it would have caught the bug:** Pins the wire format as a JSON string; any future accidental revert to `u64` will fail the quoted-literal assertion

**Backend:**
- **Test file:** `backend/tests/test_match_data_service.py` or `backend/tests/application/use_cases/test_analyze_match.py` (whichever already covers the BossSnapshot transform path)
- **What it asserts:** A parser response containing `"boss_name_hash": "12946736302082733589"` (as a string in the JSON payload) deserializes into a `BossSnapshot` whose `boss_name_hash` is the string `"12946736302082733589"`, and any downstream use (e.g. `lane_pressure_service` priority lookup) matches the `BOSS_HASH_GUARDIAN` constant
- **Why it would have caught the bug:** Pins the Pydantic model's string type and confirms `lane_pressure_service`'s equality comparisons still work after the int-to-str constant change

**Frontend:**
- **Test file:** `frontend/src/domain/boss.test.ts` (create if absent, or extend any existing boss-domain test)
- **What it asserts:** For each of the five boss types, construct a `BossSnapshot` with the real decimal-string hash and assert `getBossDisplayName` returns the expected `"<Type> - Lane <N>"` or `"<Type> - Lane <N> (<entity_index>)"` display name. Additionally assert that `BossSnapshot.boss_name_hash` is typed `string` in the interface (enforced at compile time by TypeScript)
- **Why it would have caught the bug:** The old `number` type silently allowed precision-loss values through; the `string` type plus keyed-lookup assertion makes the truncation class of bug impossible

### Existing tests affected

- `backend/tests/test_match_data_service.py`, `backend/tests/test_match_api.py`, `backend/tests/test_parsed_matches_repo.py`, `backend/tests/application/use_cases/test_analyze_match.py` -- fixture values must change from int to string; audit for fake values (`11298616958347856125`)
- `parser/src/tracking/boss_tracker/tests.rs` -- fixture construction must use `"0".to_string()` or similar
- Any frontend test file that references `BossSnapshot` fixtures (grep)

---

## Verification

| Service  | Command                                                                                              | Expected                                                                                           |
|----------|------------------------------------------------------------------------------------------------------|----------------------------------------------------------------------------------------------------|
| Parser   | `docker compose exec dashjump-parser cargo test`                                                     | All pass including new string-serialization regression test                                        |
| Parser   | `docker compose exec dashjump-parser cargo clippy`                                                   | Clean                                                                                              |
| Backend  | `docker compose exec dashjump-backend pytest`                                                        | All pass; updated fixtures load; lane_pressure_service tests pass unchanged                        |
| Backend  | `docker compose exec dashjump-backend ruff check`                                                    | Clean                                                                                              |
| Backend  | `docker compose exec dashjump-backend mypy app`                                                      | Clean                                                                                              |
| Frontend | `docker compose exec dashjump-frontend npm test`                                                     | All pass including new display-name regression test                                                |
| Frontend | `docker compose exec dashjump-frontend npm run tsc`                                                  | No TypeScript errors                                                                               |

**Manual check:**

1. Start the stack (`scripts/wt start midboss --full`), pick a match with at least one guardian, one walker, one shrine, one base guardian, and one patron visible
2. Load the match analysis page in the browser
3. Hover each objective marker -- expect labels like `Guardian - Lane 1`, `Base Guardian - Lane 3 (124)`, `Patron - Lane 0`, never `Boss #<number>`
4. Open any Sankey diagram that aggregates damage by boss type -- expect five distinct nodes (Guardian, Walker, Base Guardian, Shrine, Patron), no collapsed duplicates
5. Devtools network tab: inspect the `/match/analysis/<id>` JSON response and confirm `boss_name_hash` fields are string-typed (`"boss_name_hash": "12946736302082733589"` with quotes), not numeric

---

## Learnings

This bug is the second case in recent history where a 64-bit identifier traveled through a JSON number wire format to JavaScript and got silently corrupted. The mid-boss code at `parser/src/domain/mid_boss.rs:8` already established the string-transport convention with `pub boss_name_hash: String` -- that line is the confirmed in-codebase precedent. The regular `BossSnapshot` (`parser/src/domain/boss.rs:11`) just never got the same treatment, and the gap silently broke the UI for several weeks (TODO comment at `frontend/src/domain/boss.ts:35-41` records the discovery date). Worth recording as a cross-service pattern: **any u64 crossing a JSON boundary to a JavaScript consumer must be serialized as a string, not a number.** Candidates to audit for the same issue: `steam_id_64`, match IDs, entity indexes (safe -- under 2^16), tick counts (safe -- under 2^53 for realistic match lengths), any future ID fields added in the parser.

- [ ] Draft appended to `private/learnings.md` ## Drafts
- [ ] Pattern identified: u64 JSON interop with JavaScript requires string wire format
- [ ] Evidence cited: in-codebase precedent `parser/src/domain/mid_boss.rs:8` (string-typed hash that already works correctly across the same parser->backend->frontend path); spike `private/plans/spikes/boss-serializer-hash-drift.md` (3-replay probe coverage proving every real fxhash exceeds `Number.MAX_SAFE_INTEGER`)

---

## Execution Order

1. Fill in Context, Symptom, Root Cause, Scope, and Fix *(already done in this file)*
2. **Phase 0 -- Contract spec.** Update `private/specs/contracts/parser-output.md` (`int` -> `string`, clarify Boss Type Identification table header). Pause for user review before touching code.
3. **Phase A -- Parser.** Change `domain/boss.rs`, `tracking/boss_tracker.rs`, fixture tests; add regression test. Run `cargo test` and `cargo clippy`. User review -- commit.
4. **Phase B -- Backend.** Change `domain/boss.py`, `services/lane_pressure_service.py` constants, update all fixture files. Add Pydantic regression test. Run `pytest`, `ruff`, `mypy`. User review -- commit.
5. **Phase C -- Frontend.** Change `domain/boss.ts` interface and `BOSS_NAME_HASH_MAP`, update stale fixture in `matchAnalysis.ts`, delete the TODO comment. Add display-name regression test. Run `npm test` and `npm run tsc`. User review -- commit.
6. Manual end-to-end check via the browser (step above)
7. Run `test-auditor` and `code-reviewer` against the full cross-service diff
8. Append learnings draft
