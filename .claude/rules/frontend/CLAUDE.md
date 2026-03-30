---
paths:
  - "frontend/src/**/*.ts"
  - "frontend/src/**/*.tsx"
  - "frontend/src/**/**/*.ts"
  - "frontend/src/**/**/*.tsx"
---
# Frontend Service

React/TypeScript web application for viewing match analytics and visualizations.

## Structure

See `.claude/rules/frontend/frontend-mental-model.md` for the full module structure diagram.

## Layer Dependency Rules

```
┌─────────────────────────────────────────────────────────────┐
│                        pages/                               │
│                          ↓                                  │
│                     components/                             │
│                    ↓     ↓    ↓                             │
│               hooks/   api/  services/                      │
│                    ↓     ↓    ↓                             │
│                       domain/                               │
│                                                             │
│              utils/ ← (available to all)                    │
│              data/  ← (available to all)                    │
└─────────────────────────────────────────────────────────────┘
```

| Layer | Can Import |
|-------|------------|
| `pages/` | `components/`, `hooks/`, `api/`, `domain/`, `services/`, `utils/`, `data/` |
| `components/` | `hooks/`, `domain/`, `utils/`, `data/`, other `components/` |
| `hooks/` | `api/`, `domain/`, `services/`, `utils/` |
| `api/` | `domain/`, `utils/` |
| `services/` | `domain/`, `utils/`, `data/` |
| `domain/` | Nothing (pure type definitions) |
| `utils/` | Nothing (pure utilities) |
| `data/` | `domain/` only (for typing static data) |

## Commands

```bash
# Run dev server directly
cd frontend
npm run dev

# Run tests (headless browser)
npm test

# Run tests in visible browser
npm run test:browser

# Run tests with coverage
npm test -- --coverage

# From repo root (without local Node toolchain)
docker-compose run --rm dashjump-frontend npm test
docker-compose run --rm dashjump-frontend npm test -- --coverage

# Linting
npm run lint

# Type checking
npm run typecheck

# Build
npm run build
```

## Current Features

- Interactive minimap with player positions
- Objective state visualization
- Time-based navigation
- Player position overlays
- ETag caching for match data
- Damage analysis with SankeyDiagrams

## Planned Features

- Timeline scrubbing
- Concept overlays
- Comparative match views
- Team dashboards

## Tech Stack

- React
- TypeScript (strict mode)
- Vite
- Tailwind CSS
- React Query (for server state)
- Vitest + Playwright (browser testing)

## Data Flow

1. Page component initiates data fetch via hook
2. Hook uses API client to fetch from backend
3. API client handles caching (ETag)
4. Data flows down through components via props/context
5. Services handle data transformation for visualizations

## Code Quality

- Split components and hooks at ~200-300 lines
- Components with >5-7 props are a refactor signal -- consider splitting or lifting state to context
- No "kitchen sink" props: each component interface should cover exactly what it needs, nothing more
- Pass data and callbacks down via props or context -- avoid importing services directly inside components

## Contracts

The backend API contract is defined in `private/specs/contracts/backend-api.md`.

**TypeScript interfaces in `frontend/src/domain/` must match `backend-api.md` exactly.**

- Do not infer field types by reading Python files -- read the spec
- When the backend adds a field, update the spec first, then update the TypeScript interface
- Optional fields (`field?: type`) correspond to fields marked `no` in the Required column
- Field names are snake_case throughout -- the backend does not camelCase its JSON keys
