# dashjump.gg

Esports analytics platform for Deadlock. Monorepo with three microservices.

## Writing Style

- Use `--` (double-hyphen) instead of em-dashes (`—`) in all prose, docs, and commit messages. Em-dashes render as `<E2><80><94>` in git diffs and terminals.
- Never hard-wrap prose lines in markdown files. Do not insert manual line breaks mid-sentence. Let lines be as long as needed -- editors and renderers handle wrapping. Hard-wrapped lines with continuation indents double-wrap in terminals and look broken.

## Quick Reference

```bash
# Full stack (all services + database)
docker-compose up

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
- **SOLID Principles:** Apply across all services -- each service CLAUDE.md has service-specific thresholds and DIP guidance

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
- `backend/CLAUDE.md` -- Structure, commands, DDD layers
- `frontend/CLAUDE.md` -- Structure, commands, components
- `parser/CLAUDE.md` -- Structure, commands

## Agents

Specialized subagents for autonomous work:
- `backend-python` -- Python/FastAPI: endpoints, use cases, domain services, backend tests
- `frontend-react` -- React/TypeScript: components, hooks, state, frontend tests
- `rust-parser` -- Rust/Axum parser: replay parsing, message listeners, entity tracking, parser tests
- `haste-expert` -- Deadlock domain expert: maps Citadel messages to features, updates `private/specs/` reference docs, answers "what data can we get?" questions
- `spec-writer` -- Specs, experiment katas, strategy docs, learnings consolidation
- `code-reviewer` -- Security, convention, logic, and test coverage review (read-only)
- `test-auditor` -- Periodic test suite audit across all services (read-only)
- `e2e-playwright` -- Cross-service end-to-end tests spanning full user flows
- `dashjump-designer` -- UI/UX design specialist: visual work, layouts, design system (brand-aware; for logic-heavy components use `frontend-react`)

## Coding Standards

See `.claude/rules/` for detailed standards:
- `backend/` -- Python, DDD architecture, testing
- `frontend/` -- React, TypeScript, visualization, testing
- `parser/` -- Rust conventions
- `infra/git.md` -- Commit message and branch naming conventions

## Infrastructure

See `.claude/rules/infra/` for infrastructure and deployment:
- `INFRA.md` -- Infrastructure overview, environments, roadmap
- `containers.md` -- Docker images, multi-stage builds, optimization
- `docker-compose.md` -- Local development, networking, volumes
- `devcontainer.md` -- Unified development environment setup
- `ci-cd.md` -- GitHub Actions workflows, testing strategy
- `deployment.md` -- Production deployment, Kubernetes, scaling
- `worktrees.md` -- Parallel agent worktree workflow (`scripts/wt`)

### Environment Quick Reference

| Environment | Status | Purpose |
|-------------|--------|---------|
| Devcontainer | ✅ Active | Unified dev environment (Node + Python + Rust) |
| Docker Compose | ✅ Active | Local service orchestration |
| GitHub Actions CI | ✅ Active | Automated testing on PRs |
| CD Pipeline | 🚧 In Progress | Automated deployment (separate branch) |

## Error Handling & Observability

See `.claude/rules/` for error handling and observability standards:
- `error-handling.md` -- Cross-service error philosophy, categories, sensitive data rules
- `observability.md` -- Logging standards, log levels, long-term roadmap
- `backend/error-handling.md` -- Python exception hierarchy, HTTP status mapping
- `backend/observability.md` -- Python logging setup, required log points
- `frontend/error-handling.md` -- Error types, Error Boundaries, graceful degradation
- `frontend/observability.md` -- Console logging guidelines, web-vitals
- `parser/error-handling.md` -- Rust Result types, eliminating panics
- `parser/observability.md` -- Tracing setup, log levels

### Service Quick Reference

| Service | Error Strategy | Logging |
|---------|----------------|---------|
| Backend | Exception hierarchy + HTTPException mapping | Python logging to stdout |
| Frontend | AppError type + Error Boundaries | Console (minimal) |
| Parser | Result<T, E> + custom error types | tracing crate |

## Workflow

This project uses a Product Kata-driven development workflow.

### Key Locations
- Product strategy: `private/product/strategy/`
- Active experiments: `private/product/experiments/` (find `Status: active-experiment`)
- Feature specs: `private/specs/`
- Plans: pick the right template from `private/templates/plans/`:

  | Template | File location | When to use |
  |----------|---------------|-------------|
  | `spike.md` | `private/plans/spikes/` | Single narrow question, timebox ≤ 1 day, no implementation |
  | `discovery.md` | `private/plans/discovery/` | Multiple unknowns blocking implementation design |
  | `implementation.md` | `private/plans/implementation/` | Implementation where unknowns are already resolved |
- Machine-switch state: `private/CONTEXT.md` (read at session start only)

### Knowledge Management
- Before starting work, check `private/learnings-index.md` for relevant cross-project learnings
- Full knowledge management rules: `.claude/knowledge-management.md`
- Service mental models: `.claude/rules/[service]/[service]-mental-model.md`
- If you discover a cross-project pattern, append to `private/learnings.md` ## Drafts section
- Run `/consolidate-learnings` weekly to promote drafts (spec-writer agent)

### Shared File Ownership
- Strategy files (`vision.md`, `current-options.md`): spec-writer agent only
- `private/learnings-index.md`: spec-writer only (updated during `/consolidate-learnings`)
- `private/learnings.md` (promoted entries): spec-writer only
- `private/learnings.md` ## Drafts: any service agent may append

### Definition of Done (applies to ALL work)
Every completed unit of work must meet these standards:
- Code reviewed (use code-reviewer agent or self-review for quick-fixes)
- Tests written and passing for new/changed code
- Observability: logging instrumented per service conventions
- Security: no violations of dashjump-compliance skill checklist
- Conventions: follows relevant `.claude/rules/[service]/CLAUDE.md` patterns

### Development Principles
- NEVER build without a linked experiment defining the outcome we're targeting
- Specs require task shards -- atomic units a subagent can execute independently
- Each experiment step must be ≤ 1 week
- Use `/kata-check` weekly to review experiment progress
- Use `/quick-fix` for bugs and small changes (skip experiment/spec ceremony)

### Before Starting Any Feature Work
1. Check `private/product/experiments/` for the active experiment
2. Read the current experiment's `kata.md` -- what step are we on?
3. If building: find the spec in `private/specs/` with task shards
4. Work from a single task shard -- don't load the full spec into context
5. After completing a shard: run the "Verify before proceeding" check

### Context Budgets (enforce when creating/updating files)
- Clear at 30%: Don't wait for context to fill. Quality degrades noticeably past 30%.
- Full budget reference: `.claude/skills/dashjump-context-audit/references/context-budgets.md`

### Project Health Auditing
- Run the dashjump-context-audit skill periodically to audit all .claude/ files
- Ownership rules: .claude/skills/dashjump-context-audit/references/ownership-map.md
- Quality criteria per file type: .claude/skills/dashjump-context-audit/references/
