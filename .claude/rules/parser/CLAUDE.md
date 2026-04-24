---
paths:
  - "parser/src/*.rs"
  - "parser/src/**/*.rs"
---
# Parser Service

Rust/Axum microservice that extracts match data from Deadlock replay (demo) files.

## Structure

See `.claude/rules/parser/parser-mental-model.md` for the full module structure diagram.

## Module Dependency Rules

```
┌─────────────────────────────────────────────────────────────┐
│                      replay_parser.rs                       │
│                          ↓                                  │
│            tracking/ ←───┼───→ handlers/                    │
│                ↓         ↓         ↓                        │
│            entities/ ← domain/ → demo/                      │
│                          ↓                                  │
│                       utils/                                │
└─────────────────────────────────────────────────────────────┘
```

| Layer | Can Import |
|-------|------------|
| `replay_parser.rs` | `domain/`, `entities/`, `tracking/`, `utils/` |
| `tracking/` | `domain/`, `entities/`, `utils/` |
| `handlers/` | `replay_parser`, `demo/`, `domain/` |
| `entities/` | Nothing (pure constants) |
| `domain/` | Nothing (pure data structures) |
| `utils/` | `entities/` only (for field keys) |

## Testing Conventions

Unit tests live in a separate `tests.rs` file alongside the module they test, not inline in the source file. Use the Rust 2018 `name.rs` style: keep the implementation in `foo.rs` and place tests in `foo/tests.rs`, then declare `#[cfg(test)] mod tests;` at the bottom of `foo.rs`.

Do NOT use `foo/mod.rs` -- `foo.rs` and `foo/mod.rs` are equivalent to the compiler, but `foo.rs` is preferred.

This keeps `tests.rs` a child module of `foo`, so private fields remain accessible without any API changes.

## Commands

All commands run inside the `dashjump-parser` container via `docker compose exec`. See the root [`.claude/CLAUDE.md`](../../../.claude/CLAUDE.md) **Runtime: everything runs in containers** section for `exec` vs `run`, stack startup (`scripts/wt start <name> --full` for worktrees -- parser is opt-in), and the worktree `--project-directory` invocation.

```bash
# Tests
docker compose exec dashjump-parser cargo test
docker compose exec dashjump-parser cargo test creep_tracker       # specific module
docker compose exec dashjump-parser cargo test -- --nocapture      # with output

# Build / run / lint / format
docker compose exec dashjump-parser cargo build
docker compose exec dashjump-parser cargo build --release
docker compose exec dashjump-parser cargo run --bin <binary> -- <args>
docker compose exec dashjump-parser cargo clippy
docker compose exec dashjump-parser cargo fmt --check
```

Note: `scripts/wt start <name>` does NOT bring up the parser by default -- pass `--full` when you need it (`scripts/wt start midboss --full`). Parser changes are expensive to rebuild, so worktrees run without it unless actively iterating on Rust.

## Code Quality

- Split modules at ~200-300 lines
- Functions with >4-5 parameters are a refactor signal -- bundle into a struct
- Pass dependencies via constructors or function parameters -- no global mutable state
- Separate handlers (HTTP), demo operations (file I/O), and parsing logic (data extraction) -- each has one reason to change

## Domain References

Before implementing new message listeners or entity subscriptions, check:
- `private/specs/citadel-messages-reference.md` -- Citadel user message catalog (IDs 300-366): fields, product alignment, implementation notes
- `private/specs/citadel-messages-supplemental.md` -- Low-alignment message namespaces (ECitadelGameEvents IDs 450-466); load only when investigating engine-level messaging
- `private/specs/entity-fields-reference.md` -- Entity field semantics, gotchas, and deprecated fields (e.g. removed m_eZipLineLaneColor)
- `private/specs/entity-fields-supplemental.md` -- Background-context entity fields not load-bearing for current features (m_nPlatformType, m_MoveType)
- `private/specs/deadlock-api-haste-reference.md` -- haste parse lifecycle, Visitor trait, message subscription patterns, haste-inspector tool

## Data Flow

1. Backend sends replay file URL to parser (port `9000`)
2. Parser extracts match data from demo file
3. Parser compresses and returns positional and damage data
4. Backend receives and transforms for storage

## Parser API Contract

`POST http://localhost:9000/parse` -- expects:
```json
{ "demo_url": "<base64url-encoded Valve CDN URL>" }
```

The parser does NOT accept a local file path or match ID directly. It decodes the URL, downloads the `.dem.bz2` from Valve's CDN (caches at `src/compressed-replays/`), decompresses (caches at `src/replays/`), and parses.

**To test against real data locally**, go through the backend -- it handles CDN URL fetching and encoding:
```
GET http://localhost:8000/match/analysis/{match_id}
```

To call the parser directly, base64url-encode the CDN URL yourself:
```bash
ENCODED=$(echo -n "http://replay.valve.net/..../match.dem.bz2" | base64 -w0)
curl -X POST http://localhost:9000/parse -H "Content-Type: application/json" \
  -d "{\"demo_url\": \"$ENCODED\"}"
```

## Output Data

Parser produces (compressed before sending to backend):
- Per-second positional data, player metadata
- Damage events, objective events, creep wave snapshots

## Contracts

The parser output contract is defined in `private/specs/contracts/parser-api.md`.

**Before serializing a new field or changing a type in `parser/src/domain/`:**
1. Update `parser-api.md` first (add field, type, required/optional, notes)
2. Then implement in Rust
3. The backend's `ParsedMatchResponse` must be updated to match before the shard is complete

Do not add fields to serde structs that are not in `parser-api.md`. Field names in serialized
output are part of the contract -- rename only with a corresponding spec update.
