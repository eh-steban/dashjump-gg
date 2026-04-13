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

All commands run inside the `dashjump-backend` container via `docker compose exec`. See the root [`.claude/CLAUDE.md`](../../../.claude/CLAUDE.md) **Runtime: everything runs in containers** section for `exec` vs `run`, stack startup, and the worktree `--project-directory` invocation.

```bash
# Tests
docker compose exec dashjump-backend pytest
docker compose exec dashjump-backend pytest tests/test_match_api.py -x -q
docker compose exec dashjump-backend pytest --cov=app --cov-report=term-missing

# Linting / typing
docker compose exec dashjump-backend ruff check app/
docker compose exec dashjump-backend ruff format app/
docker compose exec dashjump-backend mypy app/

# Database migrations
docker compose exec dashjump-backend alembic upgrade head
docker compose exec dashjump-backend alembic revision --autogenerate -m "description"
```

### Known pre-existing test failures

`tests/test_parsed_matches_repo.py` and `tests/test_users_repo.py` need a `deadlock_test_db` database that is not provisioned automatically; they error out on connection. This is unrelated to any recent feature work. When running the full suite, either provision the DB or `--ignore` those two files to confirm your change is clean.

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

## Code Quality

- Split modules at ~200-300 lines
- Use cases with >5-7 injected dependencies are a refactor signal
- Functions with >4-5 parameters are a refactor signal -- bundle into a dataclass or schema
- Inject ALL external dependencies (DB, APIs, services) via FastAPI `Depends` -- never instantiate concrete infrastructure inside a use case or domain service

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
