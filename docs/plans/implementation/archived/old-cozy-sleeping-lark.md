# Plan: Worktree Testing Workflow

## Context

When working in a git worktree (e.g. `dashjump-gg-timing-fix`), the worktree's `docker-compose.yaml`
bind-mounts its own `./backend`, `./parser`, etc. -- but the main stack running from `dashjump-gg`
has hardcoded ports (8000, 9000, 3000, 5432) that conflict if both run simultaneously.

The user asked whether `/ide` (Claude Code IDE integration) helps. It does not -- it is a visual
enhancement (diff viewing, diagnostics) with no effect on Docker volumes or service isolation.

**Goal:** Add a low-friction, documented workflow for testing worktree changes alongside a running
main stack. Two cases covered:
1. Python/Node changes (no compilation) -- run directly as a local process
2. Rust parser or full-stack changes -- docker-compose with port offsets

**Key constraint:** `postgres-data` is a Docker named volume. Two postgres processes cannot share
the same data directory (postgres file-locks it). External volume sharing is not viable when both
stacks run DB containers simultaneously. Each worktree that needs Docker gets its own fresh DB.

## Files to Create / Modify

| File | Action |
|------|--------|
| `.gitignore` | Add `docker-compose.override.yaml` and `.env` |
| `.env.example` | Add commented `COMPOSE_PROJECT_NAME` field |
| `docker-compose.override.yaml.example` | Create -- committed template, not auto-merged by Compose |
| `.claude/rules/git.md` | Add `### Testing` subsection between Setup and Merge sections |

## Changes

### 1. `.gitignore` (modify)

Add after the existing `parser/.cargo/config.toml` line:

```
docker-compose.override.yaml
.env
```

`.env.example` stays committed. `.env` holds local project name / secrets for worktrees.

---

### 2. `.env.example` (modify)

Add at end of file:

```
# Worktree testing: set a unique name per worktree to isolate containers and volumes.
# Use the worktree short name (e.g. dashjump-gg-timing-fix).
# COMPOSE_PROJECT_NAME=dashjump-gg-[short-name]
```

---

### 3. `docker-compose.override.yaml.example` (create at repo root)

```yaml
# Worktree testing override -- committed template, NOT auto-merged by Compose.
#
# One-time worktree setup:
#   echo "COMPOSE_PROJECT_NAME=dashjump-gg-[short-name]" > .env
#   cp docker-compose.override.yaml.example docker-compose.override.yaml
#
# Start only the service(s) you changed:
#   docker-compose up dashjump-backend          # backend + its own DB
#   docker-compose up dashjump-parser           # parser only
#   docker-compose up                           # full stack
#
# After first `up`, run migrations if DB is fresh:
#   docker-compose exec dashjump-backend alembic upgrade head

services:
  dashjump-frontend:
    ports:
      - "3001:3000"
    environment:
      - VITE_BACKEND_DOMAIN=localhost:8001

  dashjump-backend:
    ports:
      - "8001:8000"

  dashjump-parser:
    ports:
      - "9001:9000"

  dashjump-db:
    ports:
      - "5433:5432"
```

The `COMPOSE_PROJECT_NAME` in `.env` namespaces all Docker resources: containers, networks, and
volumes. The worktree gets `dashjump-gg-timing-fix_postgres-data` (fresh DB, separate from main).
Port offsets let both stacks run simultaneously with no conflicts.

---

### 4. `.claude/rules/git.md` (modify)

Insert a `### Testing` section between `### Setup` (line 101) and `### Merge and rebase` (line 107).

```markdown
### Testing

**Python or Node changes (most common):** skip Docker entirely. Run the changed service directly,
pointing at the main stack's DB and parser via localhost:

```bash
# Backend on port 8001 -- connects to main stack's DB and parser
cd ../dashjump-gg-[short-name]/backend
DATABASE_URL=postgresql+psycopg://deadlock:deadlockpass@localhost:5432/deadlock_db \
PARSER_BASE_URL=http://localhost:9000 \
JWT_SECRET_KEY=dev \
uvicorn app.main:app --reload --host 0.0.0.0 --port 8001
```

Hot reload works. Main stack stays untouched.

**Rust parser or full-stack changes:** use the committed override template to run an isolated stack
with port-offset services alongside the main stack:

```bash
# One-time setup per worktree
echo "COMPOSE_PROJECT_NAME=dashjump-gg-[short-name]" > .env
cp docker-compose.override.yaml.example docker-compose.override.yaml

# Start only changed service(s) -- worktree backend on 8001, parser on 9001
docker-compose up dashjump-backend
docker-compose up dashjump-parser

# If DB is fresh (first run), apply migrations
docker-compose exec dashjump-backend alembic upgrade head
```

The worktree DB (`postgres-data` namespaced per project) is separate from the main stack's --
two postgres processes cannot share a data directory.

| Change type | Approach | Worktree port |
|-------------|----------|---------------|
| Backend (Python) | `uvicorn` directly | 8001 |
| Frontend (Node) | `npm run dev -- --port 3001` | 3001 |
| Parser (Rust) | `docker-compose up dashjump-parser` | 9001 |
| Backend + Parser | docker-compose both | 8001 + 9001 |
| Full stack | docker-compose all services | 8001 + 9001 + 3001 |
```

## Rejected Alternative: Shared Postgres + Named Databases

An alternative considered: one shared Postgres container, multiple named databases (`deadlock_agent_a`,
etc.), with `deploy: replicas: 0` in the override to disable the worktree's DB service.

**Why rejected:**
- `deploy: replicas: 0` is a Swarm-mode directive -- it is silently ignored by `docker compose up`
  in non-Swarm mode. The DB container still starts. The approach does not achieve its stated goal.
- Still requires `alembic upgrade head` against the new named database -- no migration savings.
- Requires main stack's Postgres to be running; breaks full-stack isolation.
- Adds per-worktree naming convention overhead (`deadlock_agent_a/b/c`).

`COMPOSE_PROJECT_NAME` namespacing is simpler and correct: one env var isolates all resources
including the DB volume.

## Verification

1. Apply changes to the current worktree (`dashjump-gg-timing-fix`):
   - Test the Python direct-run approach: `uvicorn ... --port 8001` from the worktree backend dir
   - Hit `localhost:8001/match/analysis/{match_id}` and confirm new timing log lines appear
   - Confirm main stack on port 8000 is unaffected

2. Verify `.gitignore` works: create `.env` and `docker-compose.override.yaml` in a worktree,
   confirm `git status` does not show them as untracked

3. Confirm `docker-compose.override.yaml.example` is tracked by git (it should be -- only the
   exact name `docker-compose.override.yaml` is ignored)
