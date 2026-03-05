---
paths:
  - "backend/**/*.py"
  - "backend/**/**/*.py"
  - "backend/**/**/**/*.py"
  - "backend/**/**/**/**/*.py"
---
# Backend Architecture Rules

## DDD Layer Structure

```
backend/
├── app/
│   ├── api/              # HTTP layer (thin routes)
│   ├── application/      # Use cases, orchestration
│   │   ├── mappers/      # ORM → Domain model mapping
│   │   └── use_cases/    # Application use cases
│   ├── domain/           # Pure business logic
│   │   ├── models/       # Entities, value objects
│   │   ├── services/     # Domain services (pure, no I/O)
│   │   └── exceptions.py
│   ├── infra/            # Infrastructure
│   │   ├── db/
│   │   │   ├── models/       # SQLModel table definitions
│   │   │   └── repositories/ # Data access layer
│   │   ├── external/     # External API clients
│   │   └── storage/      # S3 storage (future)
│   └── utils/            # Cross-cutting utilities
```

## Layer Dependency Rules

| Layer | Can Import |
|-------|------------|
| `api/` | `application/`, `domain/models/`, `utils/` |
| `application/use_cases/` | `application/mappers/`, `domain/`, `infra/`, `utils/` |
| `application/mappers/` | `domain/models/`, `infra/db/models/`, `utils/` |
| `domain/models/` | Nothing (pure data structures) |
| `domain/services/` | `domain/models/`, `utils/` |
| `infra/db/models/` | `utils/` only |
| `infra/db/repositories/` | `infra/db/models/`, `utils/` |
| `infra/external/` | `domain/models/`, `utils/` |
| `utils/` | Nothing (pure utilities) |

## File Naming Conventions

| Language | Convention | Example |
|----------|------------|---------|
| Python | snake_case | `match_data_service.py`, `user_repository.py` |

## Key Patterns

### Domain vs Application Services

- **Domain services** (`domain/services/`): Pure business logic, no I/O, no framework dependencies
- **Application services** (`application/`): Orchestration, use cases, coordinates domain + infra

### Repository Return Types

Repositories return ORM models. Mapping to domain models happens in `application/mappers/`.

```python
# ✅ Repository returns ORM model
# infra/db/repositories/match_repository.py
class MatchRepository:
    def get_by_id(self, match_id: int) -> ParsedMatchModel | None:
        ...

# ✅ Mapper converts ORM → Domain
# application/mappers/match_mapper.py
class MatchMapper:
    def to_domain(self, orm: ParsedMatchModel) -> TransformedMatchData:
        ...

# ✅ Use case orchestrates repo + mapper
# application/use_cases/analyze_match.py
class AnalyzeMatchUseCase:
    def execute(self, match_id: int) -> MatchAnalysis:
        orm_model = self.repo.get_by_id(match_id)
        return self.mapper.to_domain(orm_model)
```

### Service Naming Convention

Domain services follow the `Class.verb()` pattern so the call reads naturally:

```python
# ✅ Good - reads as "MatchDataService transform" or "transform match data"
class MatchDataService:
    @staticmethod
    def transform(parsed_match: ParsedMatchResponse) -> TransformedMatchData:
        ...

# ✅ Good - reads as "PlayersDataService aggregate"
class PlayersDataService:
    @staticmethod
    def aggregate(parsed_match: ParsedMatchResponse) -> dict[str, PlayerMatchData]:
        ...

# ❌ Bad - redundant naming
class PlayerDataAggregator:
    def aggregate_player_data(...)  # "player data" appears twice
```

**Guidelines:**
- Class name = noun (what data you're working with), suffixed with `Service`
- Method name = verb (what action you're performing)
- Use plural form to distinguish services that work with multiple items (e.g., `PlayersDataService` handles data for multiple players)
- Service name relates to output domain concept (e.g., `MatchDataService` produces `TransformedMatchData`)
