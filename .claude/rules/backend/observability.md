---
paths:
  - "backend/**/*.py"
  - "backend/**/**/*.py"
  - "backend/**/**/**/*.py"
  - "backend/**/**/**/**/*.py"
---
# Backend Observability

Python/FastAPI logging and monitoring guidelines.

## Logger Setup

Use the existing `utils/logger.py` singleton pattern:

```python
from app.utils.logger import get_logger

logger = get_logger(__name__)
```

The logger is initialized once in `main.py` via `LoggerManager`.

## Current Log Format

```
%(asctime)s - %(name)s - %(levelname)s - %(message)s
```

Example output:
```
2024-01-15 10:30:45 - app.api.match - INFO - Match 123 parsed successfully
```

## Log Level Usage

```python
# DEBUG - Development troubleshooting (not in production)
logger.debug("Query params: %s", params)

# INFO - Normal operations worth noting
logger.info("Match %s: Parsed successfully in %.2fs", match_id, duration)

# WARNING - Recoverable issues
logger.warning("Cache miss for match %s, fetching from source", match_id)

# ERROR - Failures (use logger.exception for stack traces)
logger.error("External API failed: %s", error_message)
logger.exception("Unexpected error processing match %s", match_id)
```

## Required Log Points

### API Routes

```python
@router.get("/analysis/{match_id}")
async def get_match_analysis(match_id: int):
    logger.info("Match %s: Analysis requested", match_id)

    # ... processing ...

    logger.info("Match %s: Response size %s bytes", match_id, len(response))
    return response
```

### External Service Calls

```python
logger.info("Calling parser service: %s", url)
start = time.time()
response = await client.post(url, json=payload)
duration = time.time() - start
logger.info("Parser response: status=%s, duration=%.2fs", response.status_code, duration)
```

### Repository Operations

```python
# DEBUG level for routine queries
logger.debug("Fetching match %s from database", match_id)

# ERROR level for failures
logger.error("Database error fetching match %s: %s", match_id, str(e)[:200])
```

### Cache Operations

```python
logger.debug("Cache hit for match %s", match_id)
logger.info("Cache miss for match %s, fetching fresh data", match_id)
```

## Performance Profiling Logs

Keep the existing pattern for data size logging:

```python
logger.info("Match %s - Raw parser response size: %s bytes", match_id, raw_size)
logger.info("Match %s - Compressed size: %s bytes", match_id, compressed_size)
logger.info("Match %s - Compression ratio: %.1f%%", match_id, ratio)
```

## Correlation IDs (Future)

Add correlation ID support for cross-service tracing:

```python
# middleware.py (future)
@app.middleware("http")
async def add_correlation_id(request: Request, call_next):
    correlation_id = request.headers.get("X-Correlation-ID", str(uuid.uuid4()))
    # Add to logging context
    with logger.contextualize(correlation_id=correlation_id):
        response = await call_next(request)
        response.headers["X-Correlation-ID"] = correlation_id
        return response
```

## What NOT to Log

- API keys, tokens, passwords
- Full request bodies (use DEBUG if needed)
- User session data
- SQL queries with user data (sanitize first)

## Environment Configuration (Future)

```python
# config.py
import os

LOG_LEVEL = os.getenv("LOG_LEVEL", "INFO")
LOG_FORMAT = os.getenv("LOG_FORMAT", "plain")  # "plain" or "json"
```
