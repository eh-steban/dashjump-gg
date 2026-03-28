---
paths:
  - "backend/**/*.py"
  - "backend/**/**/*.py"
  - "backend/**/**/**/*.py"
  - "backend/**/**/**/**/*.py"
---
# Backend Service

Python/FastAPI microservice that orchestrates data flow between the parser, external APIs, and storage.

## Structure

See `.claude/rules/backend/backend-mental-model.md` for the full module structure diagram.

## Commands

```bash
# Run locally (from repo root)
docker-compose up backend

# Run tests
cd backend
pytest

# Run with coverage
pytest --cov=app --cov-report=term-missing

# From repo root (without local Python toolchain)
docker-compose run --rm dashjump-backend pytest
docker-compose run --rm dashjump-backend pytest --cov=app --cov-report=term-missing

# Linting
ruff check app/
ruff format app/

# Type checking
mypy app/

# Database migrations
alembic upgrade head
alembic revision --autogenerate -m "description"
```

## Data Flow

1. User requests match analysis via API
2. Backend checks PostgreSQL cache
3. If not cached: fetch demo URL -- call Parser -- transform -- store
4. Return transformed data with ETag caching

## Current State

- **Storage:** PostgreSQL with JSONB (15-18 MB JSON per match)
- **Caching:** ETag-based HTTP caching
- **Migration in progress:** Planning S3/Parquet evaluation for large data storage

## Testing Notes

- Test files mirror `app/` structure in `tests/`
- Use `conftest.py` for shared fixtures
- Domain tests should be pure (no mocking)
- Application tests mock infrastructure dependencies

## Database

- PostgreSQL 16
- Alembic for migrations
- Never edit existing migrations
- Test migrations up AND down

## Contracts

Two contracts involve the backend:

| Contract | Spec file | Role |
|----------|-----------|------|
| Parser output | `private/specs/contracts/parser-output.md` | Consumer -- `ParsedMatchResponse` must match |
| Backend API | `private/specs/contracts/backend-api.md` | Owner -- `TransformedMatchData` defines the spec |

**When changing `ParsedMatchResponse`:** verify it still matches `parser-output.md` -- the parser
owns that spec, so coordinate with `rust-parser` if a parser-side change is needed.

**When changing `TransformedMatchData` or any type reachable from `MatchAnalysis`:** update
`backend-api.md` first, then implement, then run `pytest tests/test_match_api.py` to confirm the
schema tests still pass. The frontend's `domain/` types must match before the shard is complete.
