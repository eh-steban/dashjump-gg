# dashJump.gg

Esports analytics platform for competitive Deadlock. Parses Valve replay files and delivers interactive match analysis to coaches and players through a multi-service architecture.

## Overview

dashJump ingests raw `.dem` replay files, extracts structured game state data at 60Hz, and exposes it through a layered API that powers real-time visualizations -- positional heatmaps, damage flow diagrams, lane pressure timelines, and objective tracking.

## Architecture

Three independent microservices with clear domain boundaries:

```
┌─────────────────┐     ┌──────────────────┐     ┌──────────────────┐
│   React/Vite    │────▶│  FastAPI Backend  │────▶│  Rust/Axum       │
│   (TypeScript)  │◀────│  (Python 3.13)    │◀────│  Replay Parser   │
└─────────────────┘     └──────────────────┘     └──────────────────┘
                                │                          │
                         ┌──────▼──────┐           Valve CDN
                         │ PostgreSQL  │
                         │    + JSONB  │
                         └─────────────┘
```

**Backend** -- FastAPI with async SQLAlchemy, Domain-Driven Design, Pydantic v2 schemas, Alembic migrations.

**Parser** -- Rust/Axum service that downloads, decompresses, and parses `.dem.bz2` replay files using protobuf. Outputs normalized JSON per match. 97% data compression (15 MB replay → ~500 KB queryable dataset).

**Frontend** -- React 19 + TypeScript with Vite, Tailwind CSS, ECharts/Recharts visualizations, Vitest + Playwright browser testing.

## Features

- **Replay parsing** -- Streams Valve `.dem` files frame-by-frame, extracting player positions, damage events, creep waves, and objective health at each game tick
- **Lane pressure analysis** -- Computes territory control per lane per second, phase-weighted between creep position and player clustering
- **Damage attribution** -- Sankey diagram mapping attacker → victim → damage amount with type classification
- **Minimap visualization** -- Canvas-rendered real-time positions, creep wave overlays, and death pins
- **Objective tracking** -- Health timelines for Guardians, Walkers, Shrines, and Patron across the full match
- **Game phase filtering** -- Slice any view by Laning / Mid-game / Late-game time windows
- **Steam OAuth** -- Login via Steam; player profiles show career stats

## Tech Stack

| Layer | Technology |
|-------|-----------|
| Frontend | React 19, TypeScript, Vite 7, Tailwind CSS 4 |
| Charts | ECharts 6, Recharts |
| Backend | Python 3.13, FastAPI, SQLAlchemy 2 (async), Pydantic v2 |
| Parser | Rust (stable), Axum 0.8, Tokio, Haste (protobuf demo parser) |
| Database | PostgreSQL 16 (JSONB) |
| Auth | Steam OAuth via Authlib |
| Testing | pytest, Vitest, Playwright, cargo test |
| CI | GitHub Actions (lint + type check + test on all PRs) |
| Dev | Docker Compose, unified devcontainer (Node + Python + Rust) |

## Design Highlights

**Domain-Driven Design** -- Backend separates API routes, application use cases, pure domain logic, and infrastructure. Domain layer has zero framework dependencies and is unit-tested without mocks.

**Type safety end-to-end** -- Rust's compiler, Python type hints (mypy enforced in CI), TypeScript strict mode. Errors are typed at every service boundary.

**Error handling as a first-class concern** -- Typed exception hierarchy in the backend maps cleanly to HTTP status codes. `Result<T, E>` everywhere in the parser. Error boundaries and stale-while-error patterns in the frontend prevent full-page failures.

**ETag caching** -- Backend returns ETag headers; frontend re-validates stale data and falls back gracefully on errors rather than showing blank states.

## Project Status

Active development. Current focus: S3/Parquet storage evaluation for large match datasets.

## Contact

Steven Rodriguez -- https://www.linkedin.com/in/srodriguez1234/
