# Frontend Service

React/TypeScript web application for viewing match analytics and visualizations.

## Current Structure

```
frontend/
├── src/
│   ├── api/                          # API Clients
│   │   └── matchAnalysis.ts
│   │
│   ├── domain/                       # Domain Models (mirrors backend)
│   │   ├── matchAnalysis.ts
│   │   ├── player.ts
│   │   ├── boss.ts
│   │   ├── timeline.ts
│   │   └── ...
│   │
│   ├── services/                     # Business Logic
│   │   ├── damage/
│   │   │   ├── index.ts
│   │   │   ├── aggregation.ts
│   │   │   └── helpers.ts
│   │   └── timeline/
│   │       └── index.ts
│   │
│   │
│   ├── components/                   # Feature-grouped Components
│   │   ├── matchAnalysis/
│   │   │   ├── MatchTimeViewer.tsx
│   │   │   ├── Minimap.tsx
│   │   │   ├── PlayerCards.tsx
│   |   |   ├── hooks/                        # React Hooks
│   |   |   │   └── useMatchAnalysis.ts
│   │   │   └── ...
│   │   ├── damageAnalysis/
│   │   │   ├── PlayerDamageDistribution.tsx
│   │   │   └── ...
│   │   ├── profile/
│   │   └── login/
│   │
│   ├── pages/                        # Route-level Components
│   │   ├── MatchAnalysis.tsx
│   │   └── ...
│   │
│   ├── data/                         # Static Data
│   │   ├── combatTypes.ts
│   │   ├── regions.ts
│   │   └── ...
│   │
│   ├── utils/                        # Utilities
│   │   └── time.ts
│   │
│   ├── App.tsx
│   ├── index.tsx
│   └── vite-env.d.ts                 # TypeScript defs for Vite env vars
│
├── tests/                            # Mirrors src/ structure
│   ├── setup.ts                      # Global test setup (cleanup)
│   ├── api/
│   ├── domain/
│   ├── services/
│   ├── components/
│   └── pages/
│
├── Dockerfile
├── package.json
└── vite.config.ts                    # Includes Vitest config in `test` block
```

> **Navigation note:** This shows the target architecture. Current layout diverges:
> `matchAnalysis/` has 15+ components (CreepWave, LanePressure, Objectives, etc.) · No `hooks/` subdir yet · `domain/` has additional types (boss, creep, lanePressure, matchMetadata, etc.) · Required env var: `VITE_BACKEND_DOMAIN`

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
