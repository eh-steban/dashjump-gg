---
paths:
  - "parser/src/*.rs"
  - "parser/src/**/*.rs"
---
# Parser Observability

Rust/Axum logging and monitoring guidelines.

## Current Setup

Using `tracing` with default formatter:

```rust
use tracing_subscriber;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    // ...
}
```

## Log Prefix Convention

All logs use the `[parse_demo]` prefix for easy filtering:

```rust
info!("[parse_demo] Received request to parse demo");
error!("[parse_demo] Failed to decode base64: {}", e);
```

## Log Levels

```rust
use tracing::{debug, info, warn, error};

// DEBUG - Internal state, variable values (development only)
debug!("[parse_demo] Processing entity: {:?}", entity);

// INFO - Normal operations worth noting
info!("[parse_demo] Received request to parse demo with URL: {}", url);
info!("[parse_demo] Replay parsed successfully");

// WARN - Recoverable issues, unexpected but handled
warn!("[parse_demo] Unknown entity type: {}", entity_type);
warn!("[parse_demo] Skipping malformed damage record");

// ERROR - Failures requiring attention
error!("[parse_demo] Failed to decode base64: {}", e);
error!("[parse_demo] Parse failed: {:?}", e);
```

## Required Log Points

### Request Lifecycle

```rust
info!("[parse_demo] Received request to parse demo with URL: {}", url);
// ... processing ...
info!("[parse_demo] Replay parsed successfully");
```

### File Operations

```rust
info!("[parse_demo] Downloading replay from URL: {}", url);
info!("[parse_demo] Replay file already exists, skipping download");
info!("[parse_demo] Decompressing replay to {}", path);
```

### Errors

```rust
error!("[parse_demo] Failed to decode base64: {}", e);
error!("[parse_demo] Download failed: {}", e);
error!("[parse_demo] Decompression failed: {}", e);
error!("[parse_demo] Parse failed: {:?}", e);
```

## Performance Logging

Add timing for key operations:

```rust
use std::time::Instant;

let start = Instant::now();
let result = parse_replay(path)?;
let duration = start.elapsed();

info!("[parse_demo] Parse completed in {:?}", duration);
```

## Structured Logging (Future)

Move to structured fields for better log parsing:

```rust
// Current
info!("[parse_demo] Replay parsed successfully.");

// Future - structured fields
info!(
    match_id = %match_id,
    duration_ms = %duration.as_millis(),
    entities_parsed = %entity_count,
    "[parse_demo] Replay parsed successfully"
);
```

## Environment Configuration

Configure log level via environment variable:

```rust
use tracing_subscriber::EnvFilter;

fn init_logging() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .init();
}
```

Set via `RUST_LOG` environment variable:

```bash
RUST_LOG=debug cargo run     # All debug logs
RUST_LOG=parser=debug cargo run  # Debug for parser crate only
```

## What NOT to Log

- Full replay file contents
- Raw binary data
- High-frequency per-entity logs in production (use DEBUG)

## Log Output Format

Current: default human-readable format

```
2024-01-15T10:30:45.123Z  INFO parser: [parse_demo] Replay parsed successfully
```

Future: JSON format for log aggregation

```rust
tracing_subscriber::fmt()
    .json()
    .init();
```
