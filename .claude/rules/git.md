# Git Standards

## Commit Messages

### Format

```
<type>[optional scope]: <description>

[optional body]

[optional footer(s)]
```

**Example:**
```
feat(parser): add creep wave tracking to lane pressure analysis

- Parse all four creep entities per wave
- Expose wave data via /parse endpoint
- Store snapshots at 1-second intervals
```

---

### Types

Commits MUST use one of the following types:

| Type | When to Use |
|------|-------------|
| `feat` | New user-facing feature (SemVer MINOR) |
| `fix` | Bug fix (SemVer PATCH) |
| `refactor` | Code restructuring without behavior change |
| `perf` | Performance improvement |
| `test` | Adding or updating tests |
| `docs` | Documentation only |
| `chore` | Tooling, dependencies, build process |
| `ci` | CI/CD pipeline changes |

---

### Scope

Scope is OPTIONAL. When included, it MUST describe the affected service or area in lowercase, enclosed in parentheses.

```
feat(parser): ...
fix(frontend): ...
chore(backend): ...
```

---

### Breaking Changes

Breaking changes MUST be indicated in one of two ways:

1. Append `!` after the type/scope: `feat!: remove legacy parse endpoint`
2. Include a `BREAKING CHANGE:` footer in the commit body

---

### Rules

- Subject line MUST follow `<type>[optional scope]: <description>`
- Type MUST be one from the types table above
- Description MUST be written in imperative mood (e.g., "add", "fix", "remove")
- Subject line MUST NOT exceed 72 characters total
- Body SHOULD be preceded by a blank line
- Body bullets SHOULD convey impact, not mechanics — three to four bullets is enough
- MUST NOT append `Co-Authored-By` or any attribution lines
- SHOULD NOT include implementation details — describe *what* changed and *why*, not *how*

---

### What to Write

| Good | Bad |
|------|-----|
| `feat: add lane pressure visualization` | `feat: add LanePressureChart component that uses useMemo to memoize filtered array` |
| `fix: correct creep wave count off-by-one` | `fix: change <= 4 to < 4 in creep entity loop condition` |
| `feat(parser): expose boss state in output` | `feat(parser): add boss_snapshots: Vec<BossSnapshot> field and serialize with serde` |
| `chore: upgrade parser dependencies` | `chore: run cargo update and bump serde from 1.0.195 to 1.0.197` |

---

## Branch Names

Branches MUST follow [Conventional Branch](https://conventional-branch.github.io/) format:

```
<type>/<description>
```

### Types

| Type | When to Use |
|------|-------------|
| `feature/` | New feature work |
| `fix/` | Bug fix |
| `hotfix/` | Urgent production fix |
| `release/` | Release preparation |
| `chore/` | Tooling, deps, maintenance |

### Rules

- Description MUST be lowercase, using only `a-z`, `0-9`, and hyphens
- No consecutive hyphens, no leading/trailing hyphens in the description
- Include ticket number when applicable: `feature/issue-123-login-flow`
- Dots are permitted only in `release/` branches for version numbers: `release/v1.2.0`

### Examples

```
feature/player-lane-pressure
fix/parse-timing
hotfix/security-patch
release/v1.2.0
chore/upgrade-parser-deps
feature/issue-42-souls-tracking
```

### `scripts/wt` integration

`wt create <name>` defaults the branch to `feature/<name>`. Pass an explicit second arg for other types:

```bash
scripts/wt create fix-parse-timing fix/parse-timing        # fix/ branch
scripts/wt create souls feature/souls-tracking             # explicit feature/
scripts/wt create release-v2 release/v2.0.0               # release/ with version
```

---

## Worktree Workflow

Use a git worktree when running a parallel Claude Code agent on a separate branch -- for parallel feature development, hotfixes, or dependency migrations that need to land independently.

### Creating a worktree

```bash
# Creates ../dashjump-gg-<name>/ with isolated ports and a full stack
scripts/wt create <short-name> [branch-name]

# If branch-name is omitted, the short-name is used as the branch name
scripts/wt create souls feature/souls-tracking
```

Run from the repo root. The worktree branches from `main`, so each agent starts from a clean base. The script allocates ports, writes `.env`, copies the docker-compose override, and starts the stack. Then open a new terminal in the new directory and run `claude`.

### Viewing all worktrees

```bash
scripts/wt list
# Shows: name, slot, ports (frontend/backend/parser/db), and Docker up/down status
```

### Starting the stack

```bash
# Default: starts DB + backend + frontend only (~10s). No parser -- avoids 3+ min Rust compile.
scripts/wt start <name>

# Full stack including parser (needed for replay parsing workflows):
scripts/wt start <name> --full

# Apply migrations on first start:
docker compose --project-directory ../dashjump-gg-<name> exec dashjump-backend alembic upgrade head
```

### Running tests

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

### Stopping

```bash
scripts/wt stop <name>    # docker compose down (keeps postgres-data volume)
```

### private/ submodule branching

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

### Merge workflow

When another worktree's branch is merged to `main`, bring your worktree up to date with one command:

```bash
scripts/wt sync <name>
# Rebases the parent branch and private/ branch onto main,
# then runs alembic upgrade head against the worktree's DB.
# Each worktree has its own isolated postgres -- migrations don't auto-propagate.
```

Standard flow: agent commits + pushes -> PR merged to main -> run `wt sync` on other active worktrees.

### Port assignment

The script assigns ports by lowest available slot (reuses freed slots):

| Slot | Frontend | Backend | Parser | DB   |
|------|----------|---------|--------|------|
| 1    | 3010     | 8010    | 9010   | 5442 |
| 2    | 3020     | 8020    | 9020   | 5452 |
| N    | 3000+N*10 | 8000+N*10 | 9000+N*10 | 5432+N*10 |

The Rust cargo cache (`dashjump-gg-cargo-cache` volume) is shared across all worktrees -- no repeated recompilation.

### Removing a worktree

```bash
scripts/wt remove <name>
# Runs docker compose down --volumes (removes namespaced postgres-data, not cargo cache)
# Removes the git worktree directory
# Deletes the branch (warns if unpushed commits)
```

### When to use it

| Scenario | Worktree? |
|----------|-----------|
| Parallel agent working on a separate feature | yes |
| Comparing two feature variants (A/B testing approaches) | yes |
| Hotfix needed on main while feature work is active | yes |
| Dependency migration that unblocks a feature branch | yes |
| Normal feature work, no conflicting active branch | no |
| Quick fix on the current branch | no |
