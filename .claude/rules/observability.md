---
paths:
  - "backend/**/*.py"
  - "backend/**/**/*.py"
  - "backend/**/**/**/*.py"
  - "backend/**/**/**/**/*.py"
  - "frontend/src/**/*.ts"
  - "frontend/src/**/*.tsx"
  - "frontend/src/**/**/*.ts"
  - "frontend/src/**/**/*.tsx"
  - "parser/src/*.rs"
  - "parser/src/**/*.rs"
---
# Observability Standards

Cross-service logging, monitoring, and observability guidelines.

## Current State (Phase 1: Simple Logging)

- Plain text logs to stdout
- No external dependencies
- Suitable for docker-compose and k8s log aggregation
- Manual log inspection via `docker logs` or `kubectl logs`

## Log Levels

| Level | When to Use | Examples |
|-------|-------------|----------|
| DEBUG | Development troubleshooting | Variable values, flow tracing, SQL queries |
| INFO | Normal operations worth noting | Request received, match parsed, user logged in |
| WARNING | Recoverable issues | Cache miss, retry attempt, deprecated usage |
| ERROR | Failures requiring attention | External service down, parse failure, DB error |

### Level Selection Guide

- If it helps debug during development only → DEBUG
- If you'd want to know it happened in production → INFO
- If something is wrong but we recovered → WARNING
- If something failed and needs fixing → ERROR

## What to Log

### Always Log
- Request start with identifiers (match_id, user_id)
- Operation completion with duration
- Error context (type, message, relevant IDs)
- Data sizes for performance profiling
- External service calls (URL, status, duration)

### Contextual Logging
Include enough context to debug without additional queries:
```
# Good
INFO - Match 12345: Parsed successfully in 2.3s (15MB response)

# Bad
INFO - Parse complete
```

## What NOT to Log

- Passwords, API keys, tokens, secrets
- Full request/response bodies (unless DEBUG level)
- Personal identifiable information (emails, real names)
- Session data or authentication tokens
- Credit card numbers or financial data

## Correlation ID Pattern

Track requests across services:

1. Generate UUID at API gateway (backend receives request)
2. Pass via `X-Correlation-ID` header to downstream services
3. Include in all log lines for that request

```
# Backend
INFO - [corr:abc-123] Match 456: Fetching from parser

# Parser
INFO - [corr:abc-123] [parse_demo] Processing match 456
```

## Service-Specific Guidelines

See detailed logging guidelines:
- `backend/observability.md` -- Python logging setup
- `frontend/observability.md` -- Browser console logging
- `parser/observability.md` -- Rust tracing setup

## Performance Considerations

- Logging should not significantly impact request latency
- Use async logging where possible
- Avoid logging in hot paths (per-entity loops)
- Sample high-volume events if needed
