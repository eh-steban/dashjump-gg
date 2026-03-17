---
paths:
  - "parser/src/*.rs"
  - "parser/src/**/*.rs"
---
# Parser Service

Rust/Axum microservice that extracts match data from Deadlock replay (demo) files.

## Current Structure

```
parser/
├── src/
│   ├── main.rs               # Axum server setup, module registration
│   ├── config.rs             # Configuration constants
│   ├── replay_parser.rs      # Core parsing coordinator (~400 lines)
│   │
│   ├── domain/               # Data Structures (pure, serializable)
│   │   ├── player.rs         # Player, PlayerPosition
│   │   ├── boss.rs           # BossSnapshot
│   │   ├── damage.rs         # DamageRecord
│   │   └── creep.rs          # CreepSnapshot, LaneCreepData, WaveMeta
│   │
│   ├── entities/             # Entity Identification
│   │   └── constants.rs      # Entity hashes, field keys (fkey_from_path)
│   │
│   ├── tracking/             # Stateful Trackers
│   │   ├── boss_tracker.rs   # BossTracker (spawn/despawn lifecycle)
│   │   └── creep_tracker/    # CreepTracker (per-creep lane tracking)
│   │       ├── mod.rs        #   implementation
│   │       └── tests.rs      #   unit tests
│   │
│   ├── utils/                # Pure Helper Functions
│   │   ├── entity_position.rs # get_entity_position()
│   │   └── steam_id.rs       # steamid64_to_accountid()
│   │
│   ├── handlers/             # HTTP Route Handlers
│   │   ├── check_demo.rs
│   │   └── parse_demo.rs
│   │
│   └── demo/                 # Demo File Operations
│       ├── downloader.rs
│       └── decompressor.rs
│
├── Cargo.toml
├── Dockerfile
└── docker-compose.yaml
```

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

Unit tests live in a separate `tests.rs` file alongside the module they test, not inline in the source file. For a module `foo.rs`, create `foo/mod.rs` (implementation) and `foo/tests.rs` (tests), then declare `#[cfg(test)] mod tests;` at the bottom of `mod.rs`.

This keeps `tests.rs` a child module of `foo`, so private fields remain accessible without any API changes.

## Commands

```bash
# Dev server (hot reload) — run from parser/
cargo watch -i src/compressed-replays/ -i src/replays/ -x run

# Tests
cargo test

# Lint
cargo clippy

# Format check
cargo fmt --check

# Build release
cargo build --release
```

### From repo root (without local Rust toolchain)

```bash
# Tests
docker-compose run --rm dashjump-parser cargo test

# Specific module
docker-compose run --rm dashjump-parser cargo test creep_tracker

# With output
docker-compose run --rm dashjump-parser cargo test -- --nocapture
```

## Domain References

Before implementing new message listeners or entity subscriptions, check:
- `private/specs/citadel-messages-reference.md` -- Citadel user message catalog (IDs 300–366): fields, product alignment, implementation notes
- `private/specs/citadel-messages-supplemental.md` -- Low-alignment message namespaces (ECitadelGameEvents IDs 450–466); load only when investigating engine-level messaging
- `private/specs/entity-fields-reference.md` -- Entity field semantics, gotchas, and deprecated fields (e.g. removed m_eZipLineLaneColor)
- `private/specs/entity-fields-supplemental.md` -- Background-context entity fields not load-bearing for current features (m_nPlatformType, m_MoveType)
- `private/specs/deadlock-api-haste-reference.md` -- haste parse lifecycle, Visitor trait, message subscription patterns, haste-inspector tool

## Data Flow

1. Backend sends replay file URL to parser (port `9000`)
2. Parser extracts match data from demo file
3. Parser compresses and returns positional and damage data
4. Backend receives and transforms for storage

## Output Data

Parser produces (compressed before sending to backend):
- Per-second positional data, player metadata
- Damage events, objective events, creep wave snapshots
