# dashjump.gg

Esports analytics platform for Deadlock. Monorepo with three microservices.

## Quick Reference

```bash
# Service-specific commands in each service's CLAUDE.md
```

## Project Structure

```
dashjump-gg/
├── backend/          # Python/FastAPI - API, orchestration, storage
├── frontend/         # React/TypeScript - Web app, visualizations
├── parser/           # Rust/Axum - Replay file parsing
├── docker-compose.yaml
└── k8s/              # Kubernetes manifests
```

## Key Principles

- **Game Alignment:** Valve's Deadlock is source of truth for domain terminology
- **API Advisement:** Avoid translation layers between internal/external schemas where possible
- **DDD Architecture:** Domain layer is pure business logic, no framework dependencies
- **Visualization Philosophy:** Tell stories with data, organize by game phase

## Game Phases

Analytics should be organized by phase (definitions need refinement beyond time):

| Phase | ~Duration | Factors to Consider |
|-------|-----------|---------------------|
| Laning | 33% | Early objectives, lane pressure, guardians |
| Mid-game | 33% | Rotations, team fights begin, walkers |
| Late game | Rest | Full team fights, final objectives |

## Current Roadmap

1. S3/Parquet Evaluation (current)
2. Architecture Stabilization
3. Event-Driven Architecture
4. Terraform

## Service Details

See `.claude/rules/` for detailed standards:
- `backend/CLAUDE.md` — Structure, commands, DDD layers
- `frontend/CLAUDE.md` — Structure, commands, components
- `parser/CLAUDE.md` — Structure, commands

## Coding Standards

See `.claude/rules/` for detailed standards:
- `backend/` — Python, DDD architecture, testing
- `frontend/` — React, TypeScript, visualization, testing
- `parser/` — Rust conventions
- `git.md` — Commit message conventions

## Infrastructure

See `.claude/rules/infra/` for infrastructure and deployment:
- `INFRA.md` — Infrastructure overview, environments, roadmap
- `containers.md` — Docker images, multi-stage builds, optimization
- `docker-compose.md` — Local development, networking, volumes
- `devcontainer.md` — Unified development environment setup
- `ci-cd.md` — GitHub Actions workflows, testing strategy
- `deployment.md` — Production deployment, Kubernetes, scaling

### Quick Reference

| Environment | Status | Purpose |
|-------------|--------|---------|
| Devcontainer | ✅ Active | Unified dev environment (Node + Python + Rust) |
| Docker Compose | ✅ Active | Local service orchestration |
| GitHub Actions CI | ✅ Active | Automated testing on PRs |
| CD Pipeline | 🚧 In Progress | Automated deployment (separate branch) |

## Error Handling & Observability

See `.claude/rules/` for error handling and observability standards:
- `error-handling.md` — Cross-service error philosophy, categories, sensitive data rules
- `observability.md` — Logging standards, log levels, long-term roadmap
- `backend/error-handling.md` — Python exception hierarchy, HTTP status mapping
- `backend/observability.md` — Python logging setup, required log points
- `frontend/error-handling.md` — Error types, Error Boundaries, graceful degradation
- `frontend/observability.md` — Console logging guidelines, web-vitals
- `parser/error-handling.md` — Rust Result types, eliminating panics
- `parser/observability.md` — Tracing setup, log levels

### Quick Reference

| Service | Error Strategy | Logging |
|---------|----------------|---------|
| Backend | Exception hierarchy + HTTPException mapping | Python logging to stdout |
| Frontend | AppError type + Error Boundaries | Console (minimal) |
| Parser | Result<T, E> + custom error types | tracing crate |