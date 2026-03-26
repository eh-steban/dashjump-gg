---
paths:
  - "parser/src/*.rs"
  - "parser/src/**/*.rs"
---
# Rust Coding Standards

## General

- Follow standard Rust conventions (rustfmt, clippy)
- Use `snake_case` for files, functions, variables
- Use `PascalCase` for types and structs

## File Naming

| Type | Convention | Example |
|------|------------|---------|
| Files | snake_case | `replay_parser.rs`, `match_data.rs` |
| Modules | snake_case | `handlers`, `models` |

## Project Structure

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

## Error Handling

See `.claude/rules/parser/error-handling.md` for detailed error handling standards.

### Quick Reference

- Use `Result<T, E>` for all fallible operations
- Define custom error types with `thiserror`
- Use `anyhow` for application-level error handling
- Avoid `.unwrap()` in production code paths — use `?` or `ok_or_else`

```rust
// ✅ Good
fn parse_entity(entity: &Entity) -> Result<ParsedEntity, ParseError> {
    let value = entity.get_value(key)
        .ok_or_else(|| ParseError::MissingField("key"))?;
    Ok(ParsedEntity { value })
}

// ❌ Bad - can panic
fn parse_entity(entity: &Entity) -> ParsedEntity {
    let value = entity.get_value(key).unwrap();
    ParsedEntity { value }
}
```

### HTTP Error Responses

Return structured JSON errors with appropriate status codes:

```rust
fn error_response(status: StatusCode, message: &str) -> (StatusCode, Json<Value>) {
    (status, Json(serde_json::json!({ "error": message })))
}
```

## API Design

- Use Axum extractors for request parsing
- Return proper HTTP status codes
- Serialize responses with serde
