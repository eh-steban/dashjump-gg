# Entity Class Names Plan

## Context

The frontend maintains a static `BOSS_NAME_HASH_MAP` in `frontend/src/domain/boss.ts` that maps
`fxhash` entity class name hashes to human-readable strings. These hashes are not stable across
Valve patches -- even without class renames, field layout changes can shift the hash. The correct,
per-demo authoritative mapping lives in the `DemClassInfo` packet inside every demo file and must
be derived at parse time, not hardcoded.

**Goals:**
- Parser emits `entity_classes: { "<hash_string>": "<ClassName>" }` in every parse response,
  derived from the demo's own `DemClassInfo` packet
- Frontend resolves boss names per-match from this mapping, with no manual update needed after
  Valve patches

**Branch:** `feature/entity-class-names`
**Review workflow:** implement → test → subagent updates plan → pause for user review → commit → next phase

---

## Scope

| Service  | Involved | Agent            |
|----------|----------|------------------|
| Parser   | yes      | `rust-parser`    |
| Backend  | yes      | `backend-python` |
| Frontend | yes      | `frontend-react` |

---

## Acceptance Criteria

Feature is done when ALL of the following are true:

- [ ] Parser parse response includes `entity_classes` key mapping hash strings to class name strings
- [ ] `probe_serializers` binary prints sorted hash→name table from a real demo
- [ ] Backend passes `entity_classes` through to the `GET /match/analysis/{match_id}` response
- [ ] Frontend `getBossDisplayName` resolves names from per-match `entity_classes`, not the static map
- [ ] Static `BOSS_NAME_HASH_MAP` constant is removed from `frontend/src/domain/boss.ts`
- [ ] Boss display names render correctly in the UI for a freshly parsed match
- [ ] All in-scope phase checkpoints complete and signed off by user

---

## Reference Data

The `DemClassInfo` packet is identified by `cmd_header.cmd == EDemoCommands::DemClassInfo`.
Decoding the bytes as `CDemoClassInfo` gives a list of class entries; each exposes
`cls.network_name()` as a `&str`. The hash is `fxhash::hash_bytes(cls.network_name().as_bytes())`,
matching the same function used in `parser/src/entities/constants.rs` for compile-time constants.

Estimated output size: ~50--100 entries, ~5 KB uncompressed per match response. Acceptable for
the existing JSON blob storage path.

---

## Critical Files

| Layer | File | Change |
|-------|------|--------|
| Parser visitor | `parser/src/replay_parser.rs` | Modify |
| Parser output | `parser/src/replay_parser.rs` (`get_match_data_json`) | Modify |
| Parser probe binary | `parser/src/bin/probe_serializers.rs` | Create |
| Parser Cargo.toml | `parser/Cargo.toml` | Modify (add `[[bin]]` entry) |
| Backend domain | `backend/app/domain/match_analysis.py` | Modify |
| Backend use case | `backend/app/application/use_cases/analyze_match.py` | Modify |
| Frontend domain | `frontend/src/domain/boss.ts` | Modify |
| Frontend domain | `frontend/src/domain/matchAnalysis.ts` | Modify (add `entity_classes` field) |

---

## Phase A -- Parser (`rust-parser` agent)

### A1. Add `class_names` to visitor state and populate from `DemClassInfo`

In `parser/src/replay_parser.rs`, add `class_names: HashMap<u64, String>` to `MyVisitor` and
initialize it as `HashMap::new()` in `Default`.

In the `on_cmd` implementation (currently a no-op), handle `DemClassInfo`:

```rust
use haste::parser::EDemoCommands;
use haste::valveprotos::common::CDemoClassInfo;
use haste::valveprotos::prost::Message;

fn on_cmd(&mut self, _ctx: &Context, cmd_header: &CmdHeader, data: &[u8]) -> Result<()> {
    if cmd_header.cmd == EDemoCommands::DemClassInfo {
        let class_info = CDemoClassInfo::decode(data)?;
        for cls in &class_info.classes {
            let name = cls.network_name();
            let hash = fxhash::hash_bytes(name.as_bytes());
            self.class_names.insert(hash, name.to_owned());
        }
    }
    Ok(())
}
```

Confirm the correct import paths by checking `haste`'s re-exports; `EDemoCommands` may be under
`haste::parser` or `haste::demostream`. The `CDemoClassInfo` type is in
`haste::valveprotos::common`. Both are already pulled into the crate transitively -- verify before
adding new `Cargo.toml` dependencies.

### A2. Include `entity_classes` in parser JSON output

In `get_match_data_json`, serialize `class_names` with string-keyed hashes (JSON object keys must
be strings):

```rust
let entity_classes: serde_json::Map<String, serde_json::Value> = self
    .class_names
    .iter()
    .map(|(hash, name)| (hash.to_string(), serde_json::Value::String(name.clone())))
    .collect();

serde_json::json!({
    // ... existing fields ...
    "entity_classes": entity_classes,
})
```

### A3. Use `class_names` for the panic message in `get_custom_id`

The `_ => panic!("Unknown entity - Name: {}, Hash: {}", ...)` arm in `get_custom_id` currently
uses `serializer_entity_name.str` for the name. That field is a haste internal. Optionally enrich
the error message with the runtime lookup:

```rust
_ => {
    let resolved = self.class_names.get(&serializer_entity_name.hash)
        .map(|s| s.as_str())
        .unwrap_or("<unknown>");
    panic!(
        "Unknown entity - RuntimeName: {}, Hash: {}",
        resolved, serializer_entity_name.hash
    )
}
```

This does not change behavior -- it only improves the panic message for future debugging.

### A4. Add `probe_serializers` binary

Create `parser/src/bin/probe_serializers.rs`. The binary opens a demo, runs the parser to end,
and prints every entry in `class_names` sorted by name:

```
Usage (from repo root):
  docker-compose run --rm dashjump-parser cargo run --bin probe_serializers -- /parser/src/replays/<file>.dem
```

Output format (one line per class, sorted by name):

```
<hash>  <ClassName>
```

Model it on `probe_currency_changed.rs`: minimal visitor, populates `class_names` in `on_cmd`,
prints summary in `main`. Add a `[[bin]]` entry to `parser/Cargo.toml`:

```toml
[[bin]]
name = "probe_serializers"
path = "src/bin/probe_serializers.rs"
```

### A5. Record learnings

Append to `private/learnings.md` ## Drafts:
- Whether `CDemoClassInfo` appears once (at the start of every demo) or multiple times
- Whether `network_name()` matches the compile-time class name strings used in `constants.rs`
- Any import path surprises (e.g., `EDemoCommands` location in the haste API)

### A Checkpoint

**Status:** `[ ] Not started`

> **Agent instructions:** Stop here. Before returning you MUST:
> 1. Run `cargo test` and record results below
> 2. Run `probe_serializers` against a real replay and paste the first 20 lines of output below
> 3. Run the main parser against a real replay (via the parse endpoint) and confirm `entity_classes` key is present in the JSON response
> 4. Check off every item below -- add date and actual result inline, not just a tick
> 5. Note any deferred items with reason
> 6. Update **Status** above to reflect current state

#### Results *(agent fills in)*

- [ ] `cargo test` -- [X passed, Y failed]
- [ ] `entity_classes` key present in parse response JSON
- [ ] `probe_serializers` prints sorted hash→name table without error
- [ ] Panic message in `get_custom_id` uses runtime class name (or deferred -- state reason)
- [ ] Learnings appended to `private/learnings.md`

#### Sample output *(agent fills in)*
```
[First 20 lines from probe_serializers output]
```

#### Deferred items
[None, or list with reason]

Await user review and commit approval before proceeding to Phase B.

---

## Phase B -- Backend (`backend-python` agent)

### B1. Pass `entity_classes` through the domain model

`entity_classes` flows from the parser JSON response through `ParsedMatchResponse` and
`TransformedMatchData` to the API response. The backend does not need to interpret the mapping --
it only needs to carry it through.

In `backend/app/domain/match_analysis.py`:

```python
class ParsedMatchResponse(SQLModel):
    # ... existing fields ...
    entity_classes: dict[str, str] = {}

class TransformedMatchData(SQLModel):
    # ... existing fields ...
    entity_classes: dict[str, str] = {}
```

Default to `{}` so that cached matches parsed before this change remain valid without a schema
migration or cache invalidation.

### B2. Wire `entity_classes` through the use case

In `backend/app/application/use_cases/analyze_match.py`, in `_transform_and_store`, populate
`entity_classes` on `parsed_match` from `parsed_json_resp`:

```python
parsed_match = ParsedMatchResponse(
    # ... existing fields ...
    entity_classes=parsed_json_resp.get("entity_classes", {}),
)
```

Then in `MatchDataService.transform` (or wherever `TransformedMatchData` is constructed from
`ParsedMatchResponse`), pass it through:

```python
TransformedMatchData(
    # ... existing fields ...
    entity_classes=parsed_match.entity_classes,
)
```

Locate `MatchDataService.transform` in `backend/app/services/match_data_service.py` and add the
field there.

### B3. Record learnings

Append to `private/learnings.md` ## Drafts any schema version notes -- specifically, whether the
existing `schema_version = 1` constant in `match.py` needs bumping due to the new field, or
whether the `{}` default makes it backward-compatible without a cache bust.

### B Checkpoint

**Status:** `[ ] Not started`

> **Agent instructions:** Stop here. Before returning you MUST:
> 1. Run `pytest` and record results below
> 2. Hit `GET /match/analysis/{match_id}` for a freshly parsed match and confirm `entity_classes` key is present in `parsed_match_data`
> 3. Check off every item below with date and actual result
> 4. Note any deferred items with reason
> 5. Update **Status** above

#### Results *(agent fills in)*

- [ ] `pytest` -- [X passed, Y failed]
- [ ] `entity_classes` key present in API response under `parsed_match_data`
- [ ] At least one entry in `entity_classes` maps to a known class name (e.g., `CNPC_Boss_Tier2`)
- [ ] Learnings appended to `private/learnings.md`

#### Sample output *(agent fills in)*
```
[Snippet of parsed_match_data.entity_classes from the API response -- first 5 entries]
```

#### Deferred items
[None, or list with reason]

Await user review and commit approval before proceeding to Phase C.

---

## Phase C -- Frontend (`frontend-react` agent)

### C1. Add `entity_classes` to the frontend match analysis type

In `frontend/src/domain/matchAnalysis.ts` (or wherever `TransformedMatchData` is typed on the
frontend), add:

```typescript
entity_classes: Record<string, string>;
```

Default to `{}` in any place that constructs this type (e.g., loading states, stubs).

### C2. Update `getBossDisplayName` to use per-match `entity_classes`

In `frontend/src/domain/boss.ts`:

1. Remove the `BOSS_NAME_HASH_MAP` constant entirely.
2. Update `getBossDisplayName` to accept the per-match mapping as a second argument:

```typescript
export function getBossDisplayName(
  boss: BossSnapshot,
  entityClasses: Record<string, string>
): string {
  const className = entityClasses[String(boss.boss_name_hash)] ?? '';
  const typeName = CLASS_TO_DISPLAY_NAME[className] ?? `Boss #${boss.boss_name_hash}`;
  // ... rest of existing logic ...
}
```

Where `CLASS_TO_DISPLAY_NAME` is a small local constant mapping the stable *class names*
(not hashes) to display strings -- class names are stable across patches:

```typescript
const CLASS_TO_DISPLAY_NAME: Record<string, string> = {
  'CNPC_TrooperBoss': 'Guardian',
  'CNPC_Boss_Tier2': 'Walker',
  'CNPC_BarrackBoss': 'Base Guardian',
  'CCitadel_Destroyable_Building': 'Shrine',
  'CNPC_Boss_Tier3': 'Patron',
};
```

This keeps display-name customization in the frontend (where it belongs) while the hash→class
mapping comes from the per-match data (where it must live).

### C3. Update all call sites of `getBossDisplayName`

Search for all uses of `getBossDisplayName` across the frontend and pass the `entity_classes`
object from the match response. Typically this will flow down from the page-level data fetch
through props or context.

### C4. Record learnings

Append to `private/learnings.md` ## Drafts:
- Any callsite complexity discovered (how many components needed updating, whether `entity_classes`
  had to be threaded through multiple prop layers vs. available in a shared hook/context)

### C Checkpoint

**Status:** `[ ] Not started`

> **Agent instructions:** Stop here. Before returning you MUST:
> 1. Run `npm test` and record results below
> 2. Open the match analysis page in a browser and verify boss names render correctly (not `Boss #<number>`)
> 3. Confirm `BOSS_NAME_HASH_MAP` no longer exists in the codebase (`grep -r BOSS_NAME_HASH_MAP` returns nothing)
> 4. Check off every item below with date and actual result
> 5. Note any deferred items with reason
> 6. Update **Status** above

#### Results *(agent fills in)*

- [ ] `npm test` -- [X passed, Y failed]
- [ ] Boss names render correctly in UI using per-match mapping
- [ ] `BOSS_NAME_HASH_MAP` removed -- grep confirms no occurrences
- [ ] `getBossDisplayName` updated signature propagated to all call sites
- [ ] Learnings appended to `private/learnings.md`

#### Deferred items
[None, or list with reason]

Await user review and commit approval.

---

## Verification Summary

| Phase | Command | Key checks | Status |
|-------|---------|------------|--------|
| A | `cargo test` | All tests pass | |
| A | `probe_serializers` + parse endpoint | `entity_classes` in output, hash→name entries present | |
| B | `pytest` | All tests pass | |
| B | API spot-check | `entity_classes` in `parsed_match_data`, ≥1 known class name | |
| C | `npm test` | All tests pass | |
| C | Manual UI | Boss names resolve correctly, `BOSS_NAME_HASH_MAP` absent | |

---

## Execution Order

1. **Phase A** (rust-parser) → user review → commit
2. **Phase B** (backend-python) → user review → commit *(depends on A's output schema)*
3. **Phase C** (frontend-react) → user review → commit *(depends on B's API shape)*
