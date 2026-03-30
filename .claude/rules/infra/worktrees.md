# Worktree Workflow

Use a git worktree when running a parallel Claude Code agent on a separate branch -- for parallel feature development, hotfixes, or dependency migrations that need to land independently.

## When to Use It

| Scenario | Worktree? |
|----------|-----------|
| Parallel agent working on a separate feature | yes |
| Comparing two feature variants (A/B testing approaches) | yes |
| Hotfix needed on main while feature work is active | yes |
| Dependency migration that unblocks a feature branch | yes |
| Normal feature work, no conflicting active branch | no |
| Quick fix on the current branch | no |

## Creating a Worktree

```bash
# Creates ../dashjump-gg-<name>/ with isolated ports and a full stack
scripts/wt create <short-name> [branch-name]

# If branch-name is omitted, the short-name is used as the branch name
scripts/wt create souls feature/souls-tracking
```

Run from the repo root. The worktree branches from `main`, so each agent starts from a clean base. The script allocates ports, writes `.env`, copies the docker-compose override, and starts the stack. Then open a new terminal in the new directory and run `claude`.

## Viewing All Worktrees

```bash
scripts/wt list
# Shows: name, slot, ports (frontend/backend/parser/db), and Docker up/down status
```

## Starting the Stack

```bash
# Default: starts DB + backend + frontend only (~10s). No parser -- avoids 3+ min Rust compile.
scripts/wt start <name>

# Full stack including parser (needed for replay parsing workflows):
scripts/wt start <name> --full

# Apply migrations on first start:
docker compose --project-directory ../dashjump-gg-<name> exec dashjump-backend alembic upgrade head
```

## Replay Files in Worktrees

Worktree parser containers share the main repo's replay cache via a bind mount defined in `docker-compose.override.yaml`:

```yaml
volumes:
  - ${MAIN_REPLAYS_DIR}:/parser/src/replays
```

`MAIN_REPLAYS_DIR` is written to the worktree's `.env` by `wt create` and points to `<repo-root>/parser/src/replays`. Any replay downloaded by the main stack is immediately available to all worktree parsers -- no copying needed. To use a replay in a probe or integration test, reference it by the path it has inside the container (`/parser/src/replays/<filename>`).

## Running Tests

After `scripts/wt start <name>`, the DB and backend are hot. Run tests directly -- no need for `docker exec`:

```bash
# Backend tests -- run from the worktree directory
cd ../dashjump-gg-<name>
DATABASE_URL=postgresql+psycopg://deadlock:deadlockpass@localhost:<PORT_DB>/deadlock_db pytest backend/tests/

# Browser / curl testing
curl http://localhost:<PORT_BACKEND>/health
# Open browser: http://localhost:<PORT_FRONTEND>

# Parser unit tests (no running container needed)
cd ../dashjump-gg-<name>/parser && cargo test
```

PORT_DB and PORT_BACKEND values are in `scripts/wt list` or in the worktree's `.env`.

## Stopping

```bash
scripts/wt stop <name>    # docker compose down (keeps postgres-data volume)
```

## Port Assignment

The script assigns ports by lowest available slot (reuses freed slots):

| Slot | Frontend | Backend | Parser | DB   |
|------|----------|---------|--------|------|
| 1    | 3010     | 8010    | 9010   | 5442 |
| 2    | 3020     | 8020    | 9020   | 5452 |
| N    | 3000+N*10 | 8000+N*10 | 9000+N*10 | 5432+N*10 |

The Rust cargo cache (`dashjump-gg-cargo-cache` volume) is shared across all worktrees -- no repeated recompilation.

## private/ Submodule Branching

`private/` is a git submodule. Each worktree gets its own `private/` branch that mirrors the parent branch name. This keeps learnings and docs commits isolated per feature -- agents don't step on each other, and the knowledge added by a feature is visible as a clean diff when the branch merges.

`wt create` handles this automatically: it initializes the submodule and creates the matching branch. For existing worktrees that predate this behaviour, run manually:

```bash
git -C <worktree>/private checkout -b <branch-name>
```

When the feature PR merges to main, also merge the private branch:

```bash
git -C private checkout main
git -C private merge feature/<name>
git add private
git commit -m "chore: update private submodule after feature/<name>"
```

`wt sync` rebases the private branch alongside the parent. `wt remove` deletes the private branch if it is already merged, or warns if it has unmerged commits.

## Merge Workflow

When another worktree's branch is merged to `main`, bring your worktree up to date:

```bash
scripts/wt sync <name>
# Rebases the parent branch and private/ branch onto main,
# then runs alembic upgrade head against the worktree's DB.
# Each worktree has its own isolated postgres -- migrations don't auto-propagate.
```

Standard flow: agent commits + pushes -> PR merged to main -> run `wt sync` on other active worktrees.

## Removing a Worktree

```bash
scripts/wt remove <name>
# Runs docker compose down --volumes (removes namespaced postgres-data, not cargo cache)
# Removes the git worktree directory
# Deletes the branch (warns if unpushed commits)
```
