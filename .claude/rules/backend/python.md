---
paths:
  - "backend/**/*.py"
  - "backend/**/**/*.py"
  - "backend/**/**/**/*.py"
  - "backend/**/**/**/**/*.py"
---
# Python Coding Standards

## General

- Python 3.11+
- Use type hints everywhere
- Prefer `dataclass(frozen=True)` for immutable domain models
- Use Pydantic models for API validation and DTOs

## Naming Conventions

| Type | Convention | Example |
|------|------------|---------|
| Files | snake_case | `transform_match_data.py` |
| Classes | PascalCase | `MatchRepository`, `DamageCalculator` |
| Functions | snake_case | `calculate_damage_per_second` |
| Constants | UPPER_SNAKE_CASE | `MAX_MATCH_DURATION` |

## Domain Services

Name domain services so that `Class.method()` reads naturally as an action:

```python
# ✅ Good - reads as "PlayersDataService aggregate" or "aggregate players data"
class PlayersDataService:
    @staticmethod
    def aggregate(parsed_match: ParsedMatchResponse) -> dict[str, PlayerMatchData]:
        ...

# ✅ Good - reads as "MatchDataService transform"
class MatchDataService:
    @staticmethod
    def transform(parsed_match: ParsedMatchResponse) -> TransformedMatchData:
        ...

# ❌ Bad - redundant naming
class PlayerDataAggregator:
    def aggregate_player_data(...)  # "player data" appears twice
```

**Guidelines:**
- Class name = noun (what data you're working with)
- Method name = verb (what action you're performing)
- Use plural form to distinguish services that work with 1 domain model vs multiple models (e.g., `PlayersDataService` handles data for multiple players vs `MatchDataService` handles data for 1 match)
- Suffix with `Service` to indicate it's a service class

## Domain Models

```python
from dataclasses import dataclass

# Immutable where possible
@dataclass(frozen=True)
class MatchEvent:
    timestamp: float
    event_type: str
    player_id: int
    data: dict
```

## Dependency Injection

Use constructor injection for infrastructure concerns:

```python
class MatchRepository:
    def __init__(self, db: Database, s3_client: S3Client):
        self.db = db
        self.s3_client = s3_client
```

## API Design

- Follow REST conventions
- Use Pydantic models for request/response validation
- Return proper HTTP status codes
- Include pagination for list endpoints
