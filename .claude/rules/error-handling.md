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
# Error Handling Standards

Cross-service error handling philosophy and principles.

## Philosophy

- **Fail fast, recover gracefully** — Detect errors early, handle them appropriately
- **Errors are information** — Use them for debugging and improvement
- **User-facing errors: helpful without leaking internals** — Guide users, protect implementation details
- **Internal errors: detailed for debugging** — Log full context for developers

## Error Categories

| Category | Example | HTTP Status | Response Strategy |
|----------|---------|-------------|-------------------|
| Client Error | Invalid match ID format | 400 | Helpful message explaining the issue |
| Not Found | Match doesn't exist | 404 | Clear "not found" message |
| Upstream Failure | Parser/Deadlock API down | 502/503 | Retry hint, generic message |
| Internal Error | Unexpected exception | 500 | Generic message, log full details |

## Graceful Degradation Patterns

### Stale-While-Error
Return cached data when fresh data is unavailable:
- Frontend implements this with `allowStaleOnError` option
- Prefer stale data over complete failure for non-critical reads

### Partial Data Return
When possible, return available data even if some parts fail:
- Missing damage stats? Still show player positions
- Timeline failed? Still show match summary

### Circuit Breaker (Future)
For external services with repeated failures:
- Track failure rates
- Open circuit to fail fast during outages
- Gradually test recovery

## Sensitive Data Rules

### NEVER Log
- API keys, secrets, tokens
- Session identifiers
- User credentials (passwords, OAuth tokens)
- Full request/response bodies in production

### NEVER Expose in Error Responses
- Stack traces
- Internal IPs or hostnames
- Database queries or connection strings
- File system paths
- Third-party API details

### ALWAYS Sanitize
- User input before including in error messages
- URLs (remove query params with sensitive data)
- Match IDs and player IDs are OK to include

## Error Message Guidelines

### User-Facing Messages

| Scenario | Good | Bad |
|----------|------|-----|
| Invalid input | "Match ID must be a number" | "ValueError: invalid literal for int()" |
| Not found | "Match not found" | "NullPointerException at line 42" |
| Server error | "Something went wrong. Please try again." | "PostgreSQL connection refused at 10.0.0.5:5432" |
| Timeout | "Request timed out. The server may be busy." | "httpx.TimeoutException after 300s" |

### Internal Log Messages

Include actionable context:
```
# Good
ERROR - Match 12345: Failed to parse damage data - invalid entity at offset 4521

# Bad
ERROR - Parse failed
```

## Testing Requirements

Every error path should be tested. See service-specific testing rules:
- `backend/testing.md` — API error response tests
- `frontend/testing.md` — Error state and boundary tests
- `parser/rust.md` — Result type tests
