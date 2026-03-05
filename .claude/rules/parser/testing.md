---
paths:
  - "parser/src/*.rs"
  - "parser/src/**/*.rs"
---
# Parser Testing Standards

## Philosophy

**Test error paths aggressively — the parser is the most failure-prone service.**

Malformed replay data, missing fields, and corrupt files are expected inputs. Every parse operation must have tests for both the happy path and realistic failure scenarios.

## Test Framework

Rust's built-in test framework (`#[cfg(test)]`). No external test runner.

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_steamid_conversion_known_value() {
        let account_id = steamid64_to_accountid(76561198000000000);
        assert_eq!(account_id, 39734272);
    }
}
```

## Unit Tests: Domain Functions

Test pure parsing functions in isolation:

```rust
// ✅ Good — tests a pure function with known input/output
#[test]
fn test_entity_position_extracts_coordinates() {
    let entity = make_test_entity(vec![
        ("CBodyComponent.m_cellX", 128.0f32),
        ("CBodyComponent.m_cellY", 64.0f32),
    ]);

    let pos = get_entity_position(&entity).unwrap();
    assert_eq!(pos.x, 128.0);
    assert_eq!(pos.y, 64.0);
}

// ✅ Good — tests error path for missing field
#[test]
fn test_entity_position_returns_error_on_missing_field() {
    let entity = make_test_entity(vec![]);  // no position fields

    let result = get_entity_position(&entity);
    assert!(result.is_err());
}
```

## Error Path Tests

Every fallible operation needs an error path test:

```rust
// ✅ Good — error path for corrupt data
#[test]
fn test_parse_damage_record_rejects_negative_value() {
    let result = parse_damage_record(-1.0, entity_index);
    assert!(matches!(result, Err(ParseError::EntityParse(_))));
}

// ✅ Good — error path for missing entity field
#[test]
fn test_creep_tracker_skips_unknown_entity_type() {
    let mut tracker = CreepTracker::new();
    let result = tracker.update_with_unknown_entity_hash(0xDEADBEEF);
    // Should not panic — unknown entity types are skipped, not errors
    assert!(result.is_ok());
}
```

## Integration Tests: Axum Endpoints

Test HTTP handlers using Axum's test utilities:

```rust
#[cfg(test)]
mod handler_tests {
    use axum::http::StatusCode;
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_check_demo_returns_200_for_valid_url() {
        let app = build_test_app();
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/check_demo")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"demo_url": "valid-base64-url"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_parse_demo_returns_400_for_invalid_base64() {
        let app = build_test_app();
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/parse_demo")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"demo_url": "!!!invalid!!!"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }
}
```

## Required Error Scenarios

| Scenario | Test Pattern | Expected |
|----------|--------------|---------|
| Invalid base64 in URL | Malformed demo_url | 400 Bad Request |
| Missing entity field | Entity with no position keys | `Err(ParseError::EntityParse)` |
| Unknown entity hash | Hash not in constants | Skip gracefully, continue |
| Empty positions array | Zero-frame replay | Return empty result, no panic |

## Coverage Goals

| Layer | Target |
|-------|--------|
| Pure parsing functions | 90%+ |
| Tracker state machines | All state transitions |
| HTTP handlers | Happy path + each error type |
| Entity extraction | Field-present and field-missing cases |

## Run Tests

```bash
# All tests
cargo test

# Specific module
cargo test creep_tracker

# With output (useful for debugging)
cargo test -- --nocapture
```
