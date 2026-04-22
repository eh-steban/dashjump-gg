# Boss Serializer Hash Drift Spike

> **File location:** `private/plans/spikes/boss-serializer-hash-drift.md`

## Context

Parsing replay `55423930` panics at `parser/src/replay_parser.rs:277` with `Unknown entity - Index: 3029, Hash: 16112031173533486177`, immediately after the Phase A mid-boss tracker logs its spawn event. The mid-boss tracker stores `boss_name_hash = fxhash::hash_bytes(b"CNPC_MidBoss") = 11298616958347856125` -- a different value from the one in the panic. Daisy has observed similar drift before (the Sankey UI broke when a boss hash the parser emitted no longer matched the hard-coded frontend constant, and was hand-patched at least once). Every `fxhash::hash_bytes(b"...")` constant in `parser/src/entities/constants.rs` is potentially fragile, so we need to understand the failure mode before fixing it. This blocks Phase C UI validation on branch `feature/midboss-tracking` and may force a cross-service change to how we identify entity classes.

---

## Question

For the entity classes we track via `fxhash::hash_bytes(b"...")` constants in `parser/src/entities/constants.rs`, does haste's runtime `entity.serializer().serializer_name.hash` value:
(a) always equal `fxhash::hash_bytes(&class_name_string.as_bytes())` where `class_name_string` is the symbol stored in `CDemoSendTables.symbols`; and
(b) remain stable across replays for the same logical entity (e.g. does "the mid-boss" always hash to the same value)?

Additionally: what class name corresponds to the hash `16112031173533486177` that is panicking the parser on replay 55423930? (Pure data lookup, answerable from the symbol dump alone.)

---

## Assumptions

### To Validate

- [ ] `CNPC_MidBoss` (and every other constant in `entities/constants.rs`) appears in `CDemoSendTables.symbols` with the exact same byte-for-byte class name string across replays 55423930, 68175583, and at least one replay where Phase A parsing is known to succeed. -- *How to check: `probe_all_entity_classes.rs` run on all three replays; diff the emitted symbol lists for each constant.*
- [ ] For a class whose string is byte-identical across replays, the runtime `entity.serializer().serializer_name.hash` value reported by haste is also byte-identical across replays. -- *How to check: runtime mini-probe logs `(class_name_hash, entity_index)` for every `CREATE` event in the first 60 seconds; compare the hash column per class across the three replays.*
- [ ] For every class where the above holds, the runtime hash equals our `fxhash::hash_bytes(class_name.as_bytes())` computation. -- *How to check: same runtime probe also logs `fxhash::hash_bytes(symbol_bytes)` alongside the runtime hash and asserts equality; divergences get dumped to a report file.*
- [ ] The panicking hash `16112031173533486177` on replay 55423930 corresponds to a specific symbol in that replay's `CDemoSendTables.symbols`. -- *How to check: for each symbol `s` in 55423930's symbols, compute `fxhash::hash_bytes(s.as_bytes())` and find the one that equals `16112031173533486177`. Report the class name.*
- [ ] The entity causing the panic is semantically "a mid-boss" or adjacent (rejuv crystal, pit guard, etc.) and not an unrelated NPC class that happened to appear for the first time in this replay. -- *How to check: once the class name is known, grep `private/specs/entity-types-reference.md` and deadlock.wiki for the name; confirm role.*

### Accepted (not tested here)

- `deadlock-api/haste` strips `preserve-metadata` from `Symbol`, so runtime code cannot read class name strings via the `Entity` API; strings must be recovered from `CDemoSendTables.symbols` directly. -- *Risk if wrong: the runtime mini-probe becomes trivial (just read the string from the entity) and a runtime registry is easy. Source: `private/engineering/tools/probe_all_entity_classes.rs:17-21`.*
- `fxhash::hash_bytes` is deterministic and reproducible across haste versions we pin. -- *Risk if wrong: every `_ENTITY` constant in the codebase is wrong for some cohort of replays; fix would be runtime resolution only.*
- The panic at `replay_parser.rs:277` is the immediate cause of the 502 backend response, not a secondary symptom. -- *Risk if wrong: fixing the match-statement won't unblock Phase C; we'd need to investigate the httpx disconnect separately.*

---

## Agent & Timebox

**Agent:** `rust-parser` (primary, probe authorship and runtime comparison), with a handoff to `haste-expert` if hashes disagree for byte-identical class names (indicates the drift is inside haste itself, not in our codebase).
**Timebox:** 1 day (8 hours)

---

## Research Standards

Follow `.claude/docs/shared/research.md` for confidence labels (`confirmed` / `inferred` / `hypothesis`), citation format (`file:line` or proto field), and scope discipline. Every numeric claim in Findings must cite the probe run that produced it.

---

## Investigation Approach

**Replay set** (baseline is two replays; a third is optional if accessible):
1. `55423930_379917638.dem` -- currently panics, baseline failure case. Uncompressed `.dem` files are symlinked into `/parser/src/replays/` inside the container, so no bzip2 decompression step is needed before running the probe.
2. `68175583_527726523.dem` -- the replay used by `probe_entity_counts.rs` (see `entity_counts_68175583_527726523.txt`); a known-good reference point for the existing hash constants. This is the required comparison replay.
3. **Optional third replay**: if any of the replays used by `probe_midboss_runtime` (per the comment on `parser/src/entities/constants.rs:52`) is still cached in `/parser/src/compressed-replays/`, include it. Do **not** spend time fetching a new replay to fill this slot -- replay 2 alone is sufficient evidence if nothing else is cached. The third replay exists to strengthen a "yes hashes are stable" finding, not to gate the investigation.

**Step 1 -- Symbol extraction (static, no runtime):**

Copy `private/engineering/tools/probe_all_entity_classes.rs` into `parser/src/bin/` and, in the copy only, extend the `SERIALIZER CLASS NAMES` emission loop (around `println!("{n}");` near line 139 of the original probe) to also emit `fxhash::hash_bytes(n.as_bytes())` on the same line, separated by a tab. Run the extended probe against each replay in the set. The probe already decodes `CDemoSendTables` directly from the proto, so it produces the authoritative string list. Save each run's output as `private/engineering/tools/class_symbols_<match_id>.txt`. After all runs, delete the `parser/src/bin/` copy per the probe's own documented workflow; do **not** commit the extended copy.

Output format (each file, one line per serializer class in the `SERIALIZER CLASS NAMES` section):
```
<symbol_string>\t<fxhash_of_symbol_bytes>
```
Other sections of the probe's output (`C-PREFIXED SYMBOLS`, field dumps) are not needed for this spike; leaving them untouched in the output is fine.

**Step 2 -- Cross-replay string diff:**

For every constant in `parser/src/entities/constants.rs` that uses `fxhash::hash_bytes(b"...")`, check whether the literal string is present in all three `class_symbols_*.txt` files. Record:
- Classes present-and-identical across all replays.
- Classes missing from one or more replays.
- Classes present with a different string (e.g. the name was renamed between patches).

**Step 3 -- Compile-time vs static hash agreement:**

For every entry found in step 2, compute `fxhash::hash_bytes(symbol.as_bytes())` and compare with the declared constant value. Any divergence here means the constant was authored against a different string than what actually appears in the replay -- a pure source-of-truth bug.

**Step 4 -- Runtime hash comparison mini-probe:**

Write a short `probe_entity_runtime_hashes.rs` in `private/engineering/tools/` (copy-to-bin-then-delete, same workflow). Its visitor implements `on_entity` and for every CREATE event:

- Records `(entity.index(), entity.serializer().serializer_name.hash)`.
- Skips duplicates per serializer hash (one row per distinct class).
- Runs over the full replay, not a time window. Mid-boss does not spawn until ~10 minutes in, and other entities we care about may appear even later. There is no cost reason to cap duration for a one-shot probe -- correctness over performance.

At end-of-parse, emit a line per distinct class hash seen:
```
<runtime_hash>\t<entity_index_first_seen>\t<match_time_s>
```

Run on the replay set. Save as `class_runtime_hashes_<match_id>.txt`.

**Step 5 -- Join step 1 + step 4:**

For each replay, left-join the symbol-extraction output on `fxhash(symbol_bytes) == runtime_hash`. Rows with a match prove that runtime `serializer_name.hash` is `fxhash` on the symbol string (answering question part (a)). Rows without a match reveal classes where haste is hashing something other than the raw class name -- those are the interesting cases.

**Step 6 -- Identify the panicking entity:**

In the replay-55423930 symbol table, find the entry whose `fxhash(symbol.as_bytes()) == 16112031173533486177` and report its class name. Then check whether that class name appears in replays (2) and (3); if not, we have a replay-specific NPC the parser has never seen before, and the fix is fundamentally different from "hash drift" -- it's missing coverage.

---

## Findings

**Answer:** Yes to both halves of the question. For every entity class we track via `fxhash::hash_bytes(b"...")` in `parser/src/entities/constants.rs`, haste's runtime `entity.serializer().serializer_name.hash` equals `fxhash::hash_bytes(symbol_bytes)` where `symbol_bytes` is the exact symbol stored in `CDemoSendTables.symbols`. The hash is stable across replays for any class whose symbol string is byte-identical. The panicking hash `16112031173533486177` is the true `fxhash("CNPC_MidBoss")` -- the mid-boss itself -- not a drifted or unknown class. The panic is a missing match arm in `replay_parser.rs::get_custom_id`, not a hash-integrity failure.

**Supporting evidence:**
- `haste_core/src/fxhash.rs:66` -- `pub const fn hash_bytes(bytes: &[u8]) -> u64` is a `const fn`; the same rotate-xor-multiply walk runs at const-eval time and at runtime, so `CNPC_MIDBOSS_ENTITY` (evaluated at compile time in `constants.rs:49`) and the runtime call in `mid_boss_tracker.rs:73` necessarily produce the same 64-bit result. No const-vs-runtime divergence is possible.
- `private/engineering/tools/class_symbols_55423930.txt`, `class_symbols_68175583.txt`, and `class_symbols_68182475.txt` each contain `CNPC_MidBoss\t16112031173533486177`. The extended `probe_all_entity_classes` emits the fxhash alongside each serializer name so this mapping is computed from the actual raw symbol table, not a hand-entered constant.
- Cross-replay diff (step 2): all 20 entity-class constants in `constants.rs` appear byte-identically in all three `class_symbols_*.txt` files, and their fxhash columns are equal across all three. No renames, no drift, no missing classes. Summary table: 20/20 constants with identical hash triples across 55423930, 68175583, 68182475.
- Runtime probe (`class_runtime_hashes_55423930.txt`, `class_runtime_hashes_68175583.txt`, `class_runtime_hashes_68182475.txt`): 190, 177, and 185 distinct runtime hashes respectively. When joined against each replay's static symbol table by raw u64 value, `190/190`, `177/177`, and `185/185` hashes find at least one symbol whose `fxhash::hash_bytes(symbol_bytes)` equals the runtime value. No unmatched rows in any replay. No exceptions.
- Reverse lookup (step 6): in every replay's symbol table, the only symbol whose fxhash equals `16112031173533486177` is `CNPC_MidBoss`. No collision, no alternate interpretation. The mid-boss runtime hash is observed in all three replays' runtime probe output, confirming the class actually instantiates in every replay (not just the symbol table).
- The frontend/backend hardcoded magic number `11298616958347856125` (`frontend/src/domain/matchAnalysis.ts:99`, also duplicated in four backend tests and four frontend tests) is **not** the fxhash of `CNPC_MidBoss`, nor of any symbol observed in either replay. It is a fabricated placeholder that has never matched a real runtime hash. The spike Context line that reported it as the tracker's stored value was derived from this fixture, not from the tracker's actual runtime output.

**Overall confidence:** `confirmed`

### Per-question breakdown

- **Question (a), `runtime_hash == fxhash(symbol)`:** held -- 100% join rate across all three replays (190/190 in 55423930, 177/177 in 68175583, 185/185 in 68182475). Additionally justified structurally by `fxhash::hash_bytes` being a `const fn` used by both the code under test and the probe.
- **Question (b), hash stability across replays:** held -- every entity-class constant produces the same fxhash in all three replays (20/20, step 2 table). No replay-specific drift. A small number of classes are absent from one or another replay's runtime hash list (e.g. `CNPC_ShieldedSentry` never spawns in 55423930; `CNecro_HauntingSkullEntity` never spawns in 68175583) but absence is because the class never instantiated in that replay, not because the hash differs. Symbol-table presence is universal: all 20 constants appear in all three symbol tables.
- **Identity of `16112031173533486177`:** `CNPC_MidBoss` -- the mid-boss NPC itself. Not an adjacent NPC, not a new class. Both replays contain this symbol and its fxhash. The panic message's hash value is literally the value of `CNPC_MIDBOSS_ENTITY` in `parser/src/entities/constants.rs:49`.
- **Implications for follow-up plan:**
  - Follow-up is an **implementation plan** (not discovery): parser-internal fix only. The `get_custom_id` match in `replay_parser.rs:261-282` must route `CNPC_MIDBOSS_ENTITY` somewhere (either a new fixed ID slot, or an explicit `=> entity.index() as u32` arm treating it like other bosses, or a short-circuit in `get_damage_entity_id` alongside the existing `is_boss_entity` branch so mid-boss damage events bypass `get_custom_id` the way objective-boss damage events already do). The spike does not prescribe which; that is the implementation plan's job.
  - No cross-service contract drift. The parser JSON output is unaffected; `frontend/src/domain/midBoss.ts` and `backend-api.md` do not need changes. The `11298616958347856125` magic number in frontend fixtures is a pre-existing fiction that was never wired to real parser output, so it does not need to be "corrected" as part of the fix -- it should be revisited (possibly deleted) during the implementation plan's test pass, but out of scope here.
  - The broader pattern `fxhash::hash_bytes(b"...")` is safe to keep. No runtime registry is needed. We should, however, add a regression test at the `get_custom_id` site (or higher in the damage path) that panics loudly on unknown class hashes *before* we ship any new tracker that introduces a new entity class constant without updating all dispatch sites. The mid-boss tracker was added without updating `get_custom_id`, and no test caught it because no regression case exercises mid-boss damage routing.

### Assumptions check

- [x] `CNPC_MidBoss` symbol byte-identical across 3 replays -- **held** -- present as the exact string `CNPC_MidBoss` with fxhash `16112031173533486177` in all three of `class_symbols_55423930.txt`, `class_symbols_68175583.txt`, and `class_symbols_68182475.txt`. The third replay is 68182475_4609034, pulled from the `probe_midboss_runtime` set (referenced in `parser/src/entities/constants.rs:52`) which was cached in the main worktree's `parser/src/replays/` directory and reachable via the midboss worktree's compose bind-mount.
- [x] Runtime hash byte-identical across replays for byte-identical strings -- **held** -- 20/20 tracked-class constants produce the same fxhash across all three replays (step 2 diff table); all classes that spawn in multiple replays produce identical runtime hashes in each (step 5 join).
- [x] Runtime hash equals `fxhash::hash_bytes(symbol)` -- **held** -- 190/190, 177/177, and 185/185 runtime hashes find a matching symbol by raw u64 equality across 55423930, 68175583, and 68182475 respectively. Structurally guaranteed because `haste::fxhash::hash_bytes` is a `const fn` used by both the constants and haste's own symbol table construction.
- [x] Panicking hash resolvable from 55423930's symbol table -- **held** -- `fxhash("CNPC_MidBoss") = 16112031173533486177`, the only symbol in the table that hashes to that value.
- [x] Panicking entity is mid-boss-adjacent -- **held, literally the mid-boss** -- not adjacent, but the mid-boss NPC itself.
- Accepted assumptions worth flagging:
  - The "Accepted" assumption that `deadlock-api/haste` strips `preserve-metadata` and therefore runtime code cannot read class name strings via the `Entity` API remains true -- runtime code still relies on hash comparisons, and string recovery still requires decoding `CDemoSendTables.symbols` directly. Nothing in this spike challenges that.
  - The "Accepted" assumption that the panic at `replay_parser.rs:277` is the proximate cause of the backend 502 (not a secondary symptom) was not directly revalidated in this spike -- we only confirmed the panic hash resolves to `CNPC_MidBoss`. If the implementation plan fixes the dispatch and the 502 persists, reopen as a discovery spike against the httpx disconnect path.

---

## Learnings Output

- [x] Draft entry appended to `private/learnings.md` ## Drafts covering: (i) hash-by-constant is sustainable -- no drift observed, structurally guaranteed by `fxhash::hash_bytes` being a `const fn`; (ii) the panicking entity is the mid-boss itself, not a new class; (iii) hand-off to an implementation plan to add the missing dispatch arm in `get_custom_id` / `get_damage_entity_id`.
- [x] Follow-up questions or spikes needed: **none**. The answer is `confirmed` on every sub-question. No sub-spike ("haste `Symbol::new` audit", "comptime alignment review") is justified by the data.
- [x] Cross-service contract flag: **parser-internal fix only**. The parser JSON output is not affected; the frontend/backend contract does not change. Follow-up should be an **implementation plan** under `private/plans/implementation/`, not a discovery plan. The `11298616958347856125` magic value in frontend/backend fixtures is pre-existing cruft unrelated to the panic and should be addressed separately (likely during the implementation plan's test pass).

---

## Plan Review

Run `spec-writer` agent after filling in Findings to review: template alignment, confidence labels applied correctly, assumptions checked against findings, learnings drafted, and follow-up spikes identified where confidence is below `confirmed`.
