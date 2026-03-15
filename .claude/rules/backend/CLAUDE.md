---
paths:
  - "backend/**/*.py"
  - "backend/**/**/*.py"
  - "backend/**/**/**/*.py"
  - "backend/**/**/**/**/*.py"
---
# Backend Service

Python/FastAPI microservice that orchestrates data flow between the parser, external APIs, and storage.

## Current Structure

```
backend/
├── app/
│   ├── api/                          # HTTP Layer (thin routes)
│   │   ├── match.py
│   │   ├── auth.py
│   │   ├── users.py
│   │   ├── account.py
│   │   ├── replay.py
│   │   └── session.py
│   │
│   ├── application/                  # Use Cases / Orchestration
│   │   ├── mappers/                  # ORM → Domain model mapping
│   │   │   └── match_mapper.py
│   │   └── use_cases/
│   │       ├── analyze_match.py
│   │       └── ...
│   │
│   ├── domain/                       # Business Logic (pure, no framework deps)
│   │   ├── models/                   # Entities & Value Objects
│   │   │   ├── match_analysis.py
│   │   │   ├── player.py
│   │   │   ├── boss.py
│   │   │   ├── steam_account.py
│   │   │   └── deadlock_api.py
│   │   ├── services/                 # Domain Services (pure, no I/O)
│   │   │   ├── transform_match_data.py
│   │   │   └── ...
│   │   └── exceptions.py
│   │
│   ├── infra/                        # Infrastructure Layer
│   │   ├── db/
│   │   │   ├── models/               # SQLModel table definitions
│   │   │   ├── repositories/         # Data access layer
│   │   │   ├── session.py
│   │   │   └── migrations/
│   │   ├── external/                 # External API clients
│   │   │   └── deadlock_api_client.py
│   │   └── storage/                  # Future: S3 storage
│   │
│   ├── utils/                        # Cross-cutting utilities
│   │   ├── datetime_utils.py
│   │   ├── http_cache.py
│   │   ├── logger.py
│   │   └── steam_id_utils.py
│   │
│   ├── config.py
│   └── main.py
│
├── tests/                            # Mirrors app/ structure
│   ├── api/
│   ├── application/
│   │   ├── mappers/
│   │   └── use_cases/
│   ├── domain/
│   │   ├── models/
│   │   └── services/
│   ├── infra/
│   │   └── db/
│   │       └── repositories/
│   └── conftest.py
│
├── Dockerfile
├── pyproject.toml
└── requirements.txt
```

> **Navigation note:** This shows the target DDD architecture. Current layout diverges:
> `domain/` files are directly in `domain/` (no `models/` subdir) · Services at `app/services/` · Repos at `app/repo/` · `infra/external/` → `infra/deadlock_api/`

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
3. If not cached: fetch demo URL → call Parser → transform → store
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
