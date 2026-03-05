---
paths:
  - "backend/**/*.py"
  - "backend/**/**/*.py"
  - "backend/**/**/**/*.py"
  - "backend/**/**/**/**/*.py"
---
# Backend Error Handling

Python/FastAPI error handling patterns and standards.

## Exception Hierarchy

```
Exception
├── DomainException (base for domain errors)
│   ├── MatchDataUnavailableException
│   ├── MatchParseException
│   └── MatchDataIntegrityException
├── ExternalServiceException (base for external failures)
│   ├── DeadlockAPIError
│   └── ParserServiceError
└── ValidationException (base for input validation)
```

## HTTP Status Code Mapping

| Exception | Status | Client Message |
|-----------|--------|----------------|
| ValidationException | 400 | Describe the validation error |
| MatchDataUnavailableException | 404 | "Match not found" |
| DeadlockAPIError | 502 | "Upstream service unavailable" |
| ParserServiceError | 502 | "Parse service unavailable" |
| MatchDataIntegrityException | 500 | "Data processing error" |
| SQLAlchemyError | 500 | "Database error" |
| Unhandled Exception | 500 | "Internal server error" |

## API Route Error Handling Pattern

```python
from fastapi import HTTPException
from app.domain.exceptions import DomainException
from app.utils.logger import get_logger

logger = get_logger(__name__)

@router.get("/analysis/{match_id}")
async def get_match_analysis(match_id: int, ...):
    try:
        # Business logic here
        result = await use_case.execute(match_id)
        return result

    except DomainException as e:
        # Domain errors - expected failures, log at WARNING
        logger.warning("Domain error for match_id=%s: %s", match_id, e)
        raise HTTPException(status_code=_map_status(e), detail=str(e))

    except ExternalServiceException as e:
        # External failures - log with context
        logger.error("External service error for match_id=%s: %s", match_id, e)
        raise HTTPException(status_code=502, detail="Upstream service error")

    except HTTPException:
        # Re-raise FastAPI exceptions as-is
        raise

    except Exception as e:
        # Unexpected errors - full stack trace, generic response
        logger.exception("Unhandled error for match_id=%s", match_id)
        raise HTTPException(status_code=500, detail="Internal server error")
```

## Repository Error Handling

Convert infrastructure exceptions to domain exceptions at the repository boundary:

```python
from sqlalchemy.exc import SQLAlchemyError
from app.domain.exceptions import MatchDataIntegrityException

class ParsedMatchRepository:
    def get_by_id(self, match_id: int) -> ParsedMatchModel | None:
        try:
            return self.session.query(ParsedMatchModel).filter_by(id=match_id).first()
        except SQLAlchemyError as e:
            logger.error("Database error fetching match %s: %s", match_id, e)
            raise MatchDataIntegrityException(f"Failed to fetch match {match_id}")
```

## External Service Error Handling

```python
import httpx
from app.domain.exceptions import ParserServiceError

async def call_parser(url: str, payload: dict) -> dict:
    try:
        async with httpx.AsyncClient(timeout=300.0) as client:
            response = await client.post(url, json=payload)
            response.raise_for_status()
            return response.json()
    except httpx.TimeoutException:
        logger.error("Parser timeout for URL: %s", url)
        raise ParserServiceError("Parser service timeout")
    except httpx.HTTPStatusError as e:
        logger.error("Parser error: %s - %s", e.response.status_code, e.response.text[:200])
        raise ParserServiceError(f"Parser returned {e.response.status_code}")
```

## Error Response Format

Consistent JSON error responses:

```python
# All error responses follow this structure
{
    "detail": "Human-readable error message"
}

# For validation errors (422), FastAPI provides:
{
    "detail": [
        {
            "loc": ["body", "match_id"],
            "msg": "value is not a valid integer",
            "type": "type_error.integer"
        }
    ]
}
```

## Exception Best Practices

1. **Define exceptions in domain layer** — Keep `app/domain/exceptions.py` as the source of truth
2. **Include context in exception messages** — `f"Match {match_id} not found"` not just `"Not found"`
3. **Don't catch too broadly** — Avoid bare `except:` clauses
4. **Re-raise HTTPException** — Don't wrap FastAPI exceptions in your handlers
5. **Log before raising** — Ensure errors are logged even if response fails
