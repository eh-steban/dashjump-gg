---
paths:
  - "parser/src/*.rs"
  - "parser/src/**/*.rs"
---
# Parser Error Handling

Rust/Axum error handling patterns and standards.

## Error Strategy

Use `Result<T, E>` for all fallible operations. Avoid panics in production code paths.

## Result Type Pattern

```rust
// ✅ Good - explicit error handling
fn parse_replay(path: &str) -> Result<ParsedData, ParseError> {
    let file = File::open(path)?;
    // ...
}

// ❌ Bad - can panic
fn parse_replay(path: &str) -> ParsedData {
    let file = File::open(path).unwrap();
    // ...
}
```

## Custom Error Types

Use `thiserror` for defining error types:

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ParseError {
    #[error("Failed to open replay file: {0}")]
    FileOpen(#[from] std::io::Error),

    #[error("Invalid replay format: {0}")]
    InvalidFormat(String),

    #[error("Decompression failed: {0}")]
    Decompression(String),

    #[error("Entity parsing failed: {0}")]
    EntityParse(String),

    #[error("Network download failed: {0}")]
    Download(String),
}
```

## HTTP Error Responses

Map errors to appropriate HTTP status codes:

| Error | Status Code | Response Body |
|-------|-------------|---------------|
| Invalid base64 URL | 400 | `{"error": "Invalid base64 in demo_url"}` |
| Invalid URL format | 400 | `{"error": "Could not extract filename from URL"}` |
| Download failed | 502 | `{"error": "Failed to download demo"}` |
| Decompression failed | 500 | `{"error": "Failed to decompress file"}` |
| Parse failed | 500 | `{"error": "Failed to parse replay"}` |

### Response Construction

```rust
fn error_response(status: StatusCode, message: &str) -> (StatusCode, Json<serde_json::Value>) {
    (status, Json(serde_json::json!({ "error": message })))
}

// Usage
if let Err(e) = decode_base64(input) {
    error!("[parse_demo] Failed to decode base64: {}", e);
    return Err(error_response(StatusCode::BAD_REQUEST, "Invalid base64 in demo_url"));
}
```

## Eliminating Panics

Replace `.unwrap()` calls with proper error handling:

```rust
// ❌ Current (can panic)
let value: u32 = entity.get_value(key).unwrap();

// ✅ Improved - with context
let value: u32 = entity
    .get_value(key)
    .ok_or_else(|| ParseError::EntityParse(format!("Missing value for key: {:?}", key)))?;

// ✅ Alternative - with default
let value: u32 = entity.get_value(key).unwrap_or(0);
```

### Server Startup

Use `expect` with descriptive messages for truly unrecoverable situations:

```rust
// Server must bind to start - panic is acceptable here
let listener = TcpListener::bind(addr)
    .await
    .expect("Failed to bind to address");
```

## Error Propagation

Use the `?` operator for clean error propagation:

```rust
async fn parse_demo(payload: DemoPayload) -> Result<Json<ParsedData>, (StatusCode, Json<Value>)> {
    let decoded_url = decode_demo_url(&payload.demo_url)?;
    let (filename, path) = setup_replay_path(&decoded_url)?;
    download_if_needed(&decoded_url, &path).await?;
    let decompressed = decompress_replay(&path)?;
    let result = parse_replay(&decompressed)?;
    Ok(Json(result))
}
```

## Graceful Degradation

When parsing non-critical data fails, continue with partial results:

```rust
// Skip unknown entities instead of panicking
match entity_type {
    KnownType::Player => handle_player(entity)?,
    KnownType::Damage => handle_damage(entity)?,
    _ => {
        warn!("[parse_demo] Unknown entity type: {:?}", entity_type);
        // Continue parsing, don't panic
    }
}
```

## Priority Fixes

High-priority `.unwrap()` calls to address:

1. Server startup (use `expect` with message)
2. Path/string conversions (handle invalid UTF-8)
3. Entity value access (use `ok_or_else` or defaults)
4. Regex operations (compile once, handle match failures)

## Error Context

Always include context in error messages:

```rust
// ✅ Good - includes context
error!("[parse_demo] Failed to parse entity at index {}: {}", idx, e);

// ❌ Bad - no context
error!("Parse failed");
```
