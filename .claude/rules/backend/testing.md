---
paths:
  - "backend/**/*.py"
  - "backend/**/**/*.py"
  - "backend/**/**/**/*.py"
  - "backend/**/**/**/**/*.py"
---
# Backend Testing Standards

## Domain Tests

Unit test all domain entities, value objects, and domain services:

- Test business rules and invariants
- No mocking of domain internals
- Examples: Hero pick validation, objective capture logic

```python
# ✅ Good domain test
def test_match_event_validates_timestamp():
    with pytest.raises(ValueError):
        MatchEvent(timestamp=-1, event_type="kill", ...)

def test_damage_calculation_applies_armor():
    result = calculate_damage(raw=100, armor=20)
    assert result == 80
```

## Application Tests

Test use cases with mocked infrastructure:

- Mock repositories and external services
- Verify DTOs and data transformations
- Test the orchestration logic

```python
def test_get_match_returns_transformed_data(mock_repo, mock_mapper):
    mock_repo.get_by_id.return_value = some_orm_model

    result = GetMatchUseCase(mock_repo, mock_mapper).execute(123)

    mock_mapper.to_domain.assert_called_once()
```

## Integration Tests

Strategy TBD — will define when we begin writing integration tests.

Focus areas:
- Repository implementations against test database
- S3 storage operations with LocalStack
- API endpoints end-to-end

## Coverage Goals

| Layer | Target |
|-------|--------|
| Domain | 90%+ |
| Application | 80%+ |
| Infrastructure | Critical paths |

## Error Path Testing

Every error path should be tested. API endpoints must have tests for each error category.

### Required Error Tests

| Scenario | Status Code | Test Pattern |
|----------|-------------|--------------|
| Invalid input | 400 | Validation failures |
| Resource not found | 404 | Missing match, user |
| External service failure | 502/503 | Parser down, API timeout |
| Database error | 500 | Connection failure |

### Mocking External Failures

```python
import pytest
from unittest.mock import patch

@pytest.fixture
def mock_parser_failure(httpx_mock):
    httpx_mock.add_response(
        method="POST",
        url="http://parser:9000/parse",
        status_code=500,
        json={"error": "Parse failed"},
    )
    yield

@pytest.mark.asyncio
async def test_parser_failure_returns_502(async_client, mock_parser_failure):
    response = await async_client.get("/match/analysis/123")
    assert response.status_code == 502
    assert "upstream" in response.json()["detail"].lower()
```

### Database Error Tests

```python
from sqlalchemy.exc import OperationalError

@pytest.mark.asyncio
async def test_db_connection_failure(async_client, mock_db_failure):
    response = await async_client.get("/match/analysis/123")
    assert response.status_code == 500
```

### Edge Case Testing

| Scenario | Test Name | Expected |
|----------|-----------|----------|
| Empty damage data | `test_empty_damage_array` | 200 with empty arrays |
| Malformed parser response | `test_invalid_parser_json` | 500 with logged error |
| Very large match (45+ min) | `test_large_match_parsing` | Success with profiling |

### Exception Behavior Tests

Test that exceptions include appropriate context:

```python
def test_match_not_found_includes_match_id():
    with pytest.raises(MatchDataUnavailableException) as exc_info:
        raise MatchDataUnavailableException("Match 123 not found")

    assert "123" in str(exc_info.value)
```
