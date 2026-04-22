# Multi-Agent Worktree Workflow Plan

## Context

Running multiple Claude Code agents simultaneously on separate features requires each agent to have its own isolated git worktree and running service stack, with predictable ports so the UI is accessible for E2E verification. The existing worktree section in `git.md` documents a single, manually-configured worktree for hotfixes. This plan generalizes it to N concurrent worktrees managed by a script.

A prior plan (`cozy-sleeping-lark.md`) covered a basic single-worktree testing setup with hardcoded +1 port offsets. This plan supersedes it with dynamic port allocation and full automation.

**Goals:**
- Any worktree's frontend is reachable at a predictable localhost port for browser-based E2E testing
- Creating, listing, starting, stopping, and removing worktrees takes one command
- Agents can run `pytest` against a hot DB without waiting for the Rust parser to compile
- Rust builds are not repeated across worktrees (shared cargo cache)
- Existing worktrees (`haste-migration`, `souls`, `timing-fix`) continue to work unaffected

**Primary workflow:** WSL terminal directly (`docker compose` v2 installed on WSL host). The devcontainer remains for collaborators using VS Code -- `scripts/wt` works identically in both environments since both have `bash` and `docker compose`.

**Branch:** no new branch needed -- this is repo-level infrastructure committed to the current branch
**Review workflow:** implement → verify manually → commit

---

## Tool Evaluation

Evaluated before committing to a custom script:

| Tool | Fit | Verdict |
|------|-----|---------|
| **Worktrunk** (max-sixty/worktrunk) | Good for AI parallel workflows, post-start hooks | Experimental, requires Rust binary install, hooks need project-specific config |
| **Container Use** (dagger/container-use) | MCP server: each agent gets isolated container + git branch | **WSL blocker:** open issue #36 "How to port forward while running in WSL?" -- unresolved. Port access is a core requirement for E2E testing. Also: experimental, 47 open issues, issue #101 "Claude Code fails to connect" unresolved. Wrong layer of abstraction -- replaces git worktrees with containers rather than complementing Docker Compose. |
| **OpenSandbox** (alibaba/OpenSandbox) | Multi-tenant sandbox platform, K8s-backed, SDKs | Wrong tool. Designed for running agent code in isolated execution environments (code judges, evaluation harnesses), not for isolating a developer's Docker Compose stack per feature branch. Kubernetes + ingress gateway is massive overkill. |
| **git-worktree-runner** (coderabbitai) | Bash-based, close to what we need | Does not handle docker-compose port management |
| **wtp / gwt / git-wt** | Simple path helpers | No Docker awareness |

**Decision:** thin project-specific bash script. External tools either fail on WSL (Container Use), target a different use case (OpenSandbox), or don't handle Docker port management. The custom `scripts/wt` approach builds on Docker Compose primitives that already work in this environment.

---

## Scope

| Service  | Involved | Notes |
|----------|----------|-------|
| Parser   | no | No code changes -- only cargo-cache volume config in override yaml |
| Backend  | no | Referenced in script for migration hint only |
| Frontend | no | Referenced in script port table |
| Infra    | yes | .gitignore, .env.example, docker-compose.override.yaml.example, scripts/wt, git.md |

---

## Acceptance Criteria

- [ ] `scripts/wt create my-feature` creates `../dashjump-gg-my-feature/`, starts DB + backend + frontend, and prints the assigned ports -- stack is hot when command returns
- [ ] `scripts/wt create my-feature --full` also starts the parser
- [ ] `scripts/wt list` shows all worktrees with their allocated ports and Docker up/down status
- [ ] `scripts/wt start my-feature` restarts a stopped stack (DB + backend + frontend); prints migration hint on fresh DB
- [ ] After `scripts/wt create`, `pytest` runs directly against `localhost:PORT_DB` without waiting for a new container
- [ ] Frontend accessible at assigned port for browser/curl testing immediately after `create`
- [ ] `scripts/wt sync my-feature` rebases the branch onto main and runs alembic upgrade head in the worktree's DB
- [ ] `scripts/wt remove my-feature` tears down the stack and removes the worktree + branch
- [ ] Multiple worktrees can run simultaneously without port conflicts
- [ ] Freed port slots are reused -- if slot 2 is removed, next `create` takes slot 2, not slot 3
- [ ] Rust parser builds in worktree 2 reuse cached artifacts from worktree 1 (`dashjump-gg-cargo-cache` volume is shared)
- [ ] `.env` and `docker-compose.override.yaml` do not appear in `git status` inside any worktree

---

## Critical Files

| File | Action |
|------|--------|
| `.gitignore` | Add `.env` and `docker-compose.override.yaml` |
| `.env.example` | Append `COMPOSE_PROJECT_NAME` + `PORT_*` comment block |
| `docker-compose.override.yaml.example` | Create at repo root (parameterized, shared cargo cache) |
| `scripts/wt` | Create -- bash script, executable |
| `.claude/rules/git.md` | Replace `## Worktree Workflow` section with script-aware docs |
| `private/plans/cozy-sleeping-lark.md` | Move to `private/plans/archived/old-cozy-sleeping-lark.md` |

---

## Port Allocation

Base ports match the main stack. Each worktree gets a **slot** (1, 2, 3...) with a `+slot*10` offset. Slot is assigned as the **lowest available slot** -- scanning sibling `.env` files to find which slot numbers are already in use, then returning the first free one. If slot 2 is removed, the next create reuses slot 2, not slot 4.

| Slot | Frontend | Backend | Parser | DB   |
|------|----------|---------|--------|------|
| 1    | 3010     | 8010    | 9010   | 5442 |
| 2    | 3020     | 8020    | 9020   | 5452 |
| 3    | 3030     | 8030    | 9030   | 5462 |

---

## Phase 1 -- Foundation Files

### 1.1. `.gitignore`

Append after the `parser/.cargo/config.toml` line:

```
docker-compose.override.yaml
.env
```

`.env.example` stays committed. `.env` holds computed ports + project name, never committed.

### 1.2. `.env.example`

Append after the existing `VITE_BACKEND_DOMAIN` line:

```
# Worktree isolation (written automatically by `scripts/wt create`)
# For manual setup, choose a unique name and port offset (multiples of 10).
# COMPOSE_PROJECT_NAME=dashjump-gg-[short-name]
# PORT_FRONTEND=3010
# PORT_BACKEND=8010
# PORT_PARSER=9010
# PORT_DB=5442
```

### 1.3. `docker-compose.override.yaml.example` (create at repo root)

```yaml
# Worktree isolation override -- committed template, NOT auto-merged by Compose.
# Populated automatically by `scripts/wt create`. For manual setup, copy this file
# to docker-compose.override.yaml and write PORT_* + COMPOSE_PROJECT_NAME to .env.

services:
  dashjump-frontend:
    ports:
      - "${PORT_FRONTEND:-3001}:3000"
    environment:
      - VITE_BACKEND_DOMAIN=localhost:${PORT_BACKEND:-8001}

  dashjump-backend:
    ports:
      - "${PORT_BACKEND:-8001}:8000"

  dashjump-parser:
    ports:
      - "${PORT_PARSER:-9001}:9000"
    volumes:
      - cargo-cache-shared:/home/lifted/.cargo

  dashjump-db:
    ports:
      - "${PORT_DB:-5433}:5432"

volumes:
  cargo-cache-shared:
    name: dashjump-gg-cargo-cache
```

The `name: dashjump-gg-cargo-cache` bypasses `COMPOSE_PROJECT_NAME` namespacing -- all worktrees share one Rust build cache. The `cargo-cache-shared` volume entry replaces the `cargo-cache` mount for `dashjump-parser` (same path, Docker Compose merges by path). The base `cargo-cache` volume still gets created but is unmounted -- harmless orphan.

**Use `docker compose` (with a space), not `docker-compose` (legacy v1 hyphen).** The `--project-directory` flag requires the Docker CLI plugin syntax. You're on 5.1.0 which is correct.

### Phase 1 Checkpoint

**Status:** `[ ] Not started`

- [ ] `.gitignore` has both new lines
- [ ] `.env.example` has `COMPOSE_PROJECT_NAME` + `PORT_*` block
- [ ] `docker-compose.override.yaml.example` exists at repo root, uses `${PORT_*}` vars
- [ ] `docker compose config` in main repo still validates (no change to main `docker-compose.yaml`)

---

## Phase 2 -- `scripts/wt` Script

Create `scripts/` directory and write `scripts/wt` (executable). Use `docker compose` (v2 syntax).

### Script structure

```
scripts/wt <command> [args]

Commands:
  create <name> [branch] [--full]   create worktree + start stack immediately (default: no parser)
  list                              show all worktrees with ports and Docker status
  start <name> [--full]             (re)start a stopped stack; --full includes parser
  stop <name>                       docker compose down (keeps postgres-data volume)
  sync <name>                       rebase worktree onto main + run alembic upgrade head
  remove <name>                     docker compose down --volumes; git worktree remove; branch delete
```

`create` always starts the stack immediately -- the worktree is ready to test as soon as the command returns. `start` exists for restarting a stopped worktree without recreating it. Both accept `--full` to include the Rust parser. `sync` handles the one non-obvious step in the merge workflow: each worktree has its own isolated postgres DB, so after rebasing onto `main` that contains new migrations the worktree's DB needs `alembic upgrade head` -- `sync` does both in sequence.

### Port allocation (next_slot logic)

```bash
REPO_ROOT="$(git -C "$(dirname "$0")" rev-parse --show-toplevel)"
PARENT_DIR="$(dirname "$REPO_ROOT")"
REPO_NAME="$(basename "$REPO_ROOT")"   # dashjump-gg
BASE_FRONTEND=3000; BASE_BACKEND=8000; BASE_PARSER=9000; BASE_DB=5432; PORT_STEP=10

next_slot() {
  local slot=1
  while true; do
    local target_port=$(( BASE_FRONTEND + slot * PORT_STEP ))
    local taken=false
    for env_file in "${PARENT_DIR}/${REPO_NAME}"-*/.env; do
      local pf
      pf=$(grep "^PORT_FRONTEND=" "$env_file" 2>/dev/null | cut -d= -f2)
      [[ "$pf" == "$target_port" ]] && { taken=true; break; }
    done
    $taken || break
    (( slot++ ))
  done
  echo "$slot"
}
```

Scans slots 1, 2, 3... and returns the first whose port is not claimed by any sibling `.env`. Reuses freed slots -- if slot 2 is removed, the next `create` takes slot 2. Scanning `.env` files directly (not `git worktree list`) is robust to mid-removal state.

### `cmd_create`

`create` sets up the worktree directory, writes `.env` and `docker-compose.override.yaml`, then immediately calls `cmd_start` -- so the stack is hot by the time the command returns. Pass `--full` to also start the parser.

```bash
cmd_create() {
  local name="$1" branch="" full_flag=""
  # Parse: optional branch name (no --) and optional --full
  shift
  for arg in "$@"; do
    case "$arg" in
      --full) full_flag="--full" ;;
      *)      branch="$arg" ;;
    esac
  done
  branch="${branch:-$name}"

  local dir="${PARENT_DIR}/${REPO_NAME}-${name}"
  [[ -d "$dir" ]] && { echo "Error: $dir already exists"; exit 1; }

  local slot; slot=$(next_slot)
  git -C "$REPO_ROOT" worktree add "$dir" -b "$branch" main

  # Write .env
  {
    grep -vE "^(COMPOSE_PROJECT_NAME|PORT_)|(^#.*[Ww]orktree)|(^# (COMPOSE|PORT_))" \
      "${REPO_ROOT}/.env.example"
    echo ""
    echo "# Worktree isolation (written by scripts/wt create)"
    echo "COMPOSE_PROJECT_NAME=${REPO_NAME}-${name}"
    echo "PORT_FRONTEND=$(( BASE_FRONTEND + slot * PORT_STEP ))"
    echo "PORT_BACKEND=$(( BASE_BACKEND  + slot * PORT_STEP ))"
    echo "PORT_PARSER=$(( BASE_PARSER   + slot * PORT_STEP ))"
    echo "PORT_DB=$(( BASE_DB           + slot * PORT_STEP ))"
  } > "${dir}/.env"

  cp "${REPO_ROOT}/docker-compose.override.yaml.example" \
     "${dir}/docker-compose.override.yaml"

  echo "Created: $dir (slot $slot)"
  echo "  Frontend: localhost:$(( BASE_FRONTEND + slot * PORT_STEP ))"
  echo "  Backend:  localhost:$(( BASE_BACKEND  + slot * PORT_STEP ))"
  echo ""
  cmd_start "$name" $full_flag
}
```

### `cmd_list`

Uses process substitution (`< <(...)`) to avoid subshell variable scoping issues in the while loop:

```bash
cmd_list() {
  printf "%-28s  %-4s  %-8s  %-7s  %-6s  %-5s  %s\n" \
    WORKTREE SLOT FRONTEND BACKEND PARSER DB STATUS
  echo "---"
  while IFS= read -r dir; do
    if [[ "$dir" == "$REPO_ROOT" ]]; then
      printf "%-28s  %-4s  %-8s  %-7s  %-6s  %-5s  %s\n" \
        "(main)" "--" "3000" "8000" "9000" "5432" "(main stack)"
      continue
    fi
    local name="${dir#${PARENT_DIR}/${REPO_NAME}-}"
    local env_file="${dir}/.env"
    if [[ ! -f "$env_file" ]]; then
      printf "%-28s  %s\n" "$name" "no .env (pre-dates wt script)"
      continue
    fi
    local pf pb pp pd proj
    pf=$(grep "^PORT_FRONTEND="        "$env_file" | cut -d= -f2)
    pb=$(grep "^PORT_BACKEND="         "$env_file" | cut -d= -f2)
    pp=$(grep "^PORT_PARSER="          "$env_file" | cut -d= -f2)
    pd=$(grep "^PORT_DB="              "$env_file" | cut -d= -f2)
    proj=$(grep "^COMPOSE_PROJECT_NAME=" "$env_file" | cut -d= -f2)
    local slot=$(( (pf - BASE_FRONTEND) / PORT_STEP ))
    local running
    running=$(docker ps --filter "label=com.docker.compose.project=${proj}" \
                        --format '{{.Names}}' 2>/dev/null | wc -l)
    local status="down"
    (( running > 0 )) && status="up (${running} containers)"
    printf "%-28s  %-4s  %-8s  %-7s  %-6s  %-5s  %s\n" \
      "$name" "$slot" "$pf" "$pb" "$pp" "$pd" "$status"
  done < <(git -C "$REPO_ROOT" worktree list --porcelain \
    | grep "^worktree " | awk '{print $2}')
}
```

### `cmd_start`

Starts DB + backend + frontend by default (fast, ~10s). The Rust parser is skipped unless `--full` is passed -- this is intentional. The parser image takes 3+ minutes to build from scratch, and most development/testing workflows don't require it running.

```bash
cmd_start() {
  local name="$1" full=false
  [[ "${2:-}" == "--full" ]] && full=true

  local dir="${PARENT_DIR}/${REPO_NAME}-${name}"
  if $full; then
    docker compose --project-directory "$dir" up -d
  else
    docker compose --project-directory "$dir" up -d dashjump-db dashjump-backend dashjump-frontend
  fi

  # Migration hint on fresh DB
  local rows
  rows=$(docker compose --project-directory "$dir" \
    exec -T dashjump-db \
    psql -U deadlock -d deadlock_db -qt \
    -c "SELECT COUNT(*) FROM alembic_version;" 2>/dev/null | tr -d ' \n' || echo "0")
  if [[ "$rows" == "0" ]]; then
    echo ""
    echo "Fresh DB detected. Run migrations:"
    echo "  docker compose --project-directory $dir exec dashjump-backend alembic upgrade head"
  fi
}
```

### `cmd_stop`

```bash
cmd_stop() {
  local name="$1"
  docker compose --project-directory "${PARENT_DIR}/${REPO_NAME}-${name}" down
}
```

### `cmd_sync`

Rebases the worktree branch onto `main`, then runs `alembic upgrade head` against the worktree's DB. Handles the one non-standard step in the multi-agent merge workflow.

```bash
cmd_sync() {
  local name="$1"
  local dir="${PARENT_DIR}/${REPO_NAME}-${name}"
  echo "Rebasing ${name} onto main..."
  git -C "$dir" rebase main
  echo "Applying migrations..."
  docker compose --project-directory "$dir" exec -T dashjump-backend alembic upgrade head
  echo "Sync complete."
}
```

### `cmd_remove`

```bash
cmd_remove() {
  local name="$1"
  local dir="${PARENT_DIR}/${REPO_NAME}-${name}"
  local branch
  branch=$(git -C "$REPO_ROOT" worktree list --porcelain \
    | grep -A3 "^worktree ${dir}$" \
    | grep "^branch " | sed 's|branch refs/heads/||')

  docker compose --project-directory "$dir" down --volumes 2>/dev/null || true
  git -C "$REPO_ROOT" worktree remove "$dir" --force
  if [[ -n "$branch" ]]; then
    git -C "$REPO_ROOT" branch -d "$branch" 2>/dev/null \
      || echo "Note: branch '$branch' has unpushed commits -- delete manually: git branch -D $branch"
  fi
  echo "Removed: $name"
}
```

`down --volumes` removes the namespaced `postgres-data` volume but NOT `dashjump-gg-cargo-cache` (fixed name, outside project namespace).

### Phase 2 Checkpoint

**Status:** `[ ] Not started`

- [ ] `scripts/` directory created
- [ ] `scripts/wt` is executable (`chmod +x`)
- [ ] `scripts/wt create test-wt` creates `../dashjump-gg-test-wt/`, starts 3 services, and prints ports -- all in one command
- [ ] `git status` in `dashjump-gg-test-wt/` shows `.env` and `docker-compose.override.yaml` as untracked but confirmed ignored
- [ ] `scripts/wt list` shows main + existing worktrees; `test-wt` at the lowest free slot
- [ ] `pytest` runs successfully against `localhost:<PORT_DB>` immediately after `create`
- [ ] `scripts/wt remove test-wt` then `scripts/wt create test-wt-2` gets the freed slot, not slot+1
- [ ] `scripts/wt create test-wt-full --full` brings parser up too; frontend accessible at the printed port
- [ ] `scripts/wt stop test-wt` + `scripts/wt start test-wt` restarts cleanly
- [ ] `scripts/wt sync test-wt` rebases onto main and runs alembic upgrade head
- [ ] `scripts/wt remove test-wt` tears down and cleans up

---

## Phase 3 -- Documentation + Archive

### 3.1. `.claude/rules/git.md` -- replace `## Worktree Workflow` section

Replace the existing `## Worktree Workflow` section (currently lines 88-172) with:

```markdown
## Worktree Workflow

Use a git worktree when running a parallel Claude Code agent on a separate branch -- for parallel feature development, hotfixes, or dependency migrations that need to land independently.

### Creating a worktree

```bash
# Creates ../dashjump-gg-<name>/ with isolated ports and a full stack
scripts/wt create <short-name> [branch-name]

# If branch-name is omitted, the short-name is used as the branch name
scripts/wt create souls feature/souls-tracking
```

Run from the repo root. The script creates a sibling directory, allocates ports, writes `.env`, and copies the docker-compose override. Then open a new terminal in the new directory and run `claude`.

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

### Merge workflow

When another worktree's branch is merged to `main`, bring your worktree up to date with one command:

```bash
scripts/wt sync <name>
# Rebases the branch onto main, then runs alembic upgrade head against the worktree's DB.
# Each worktree has its own isolated postgres -- migrations don't auto-propagate.
```

Standard flow: agent commits + pushes → PR merged to main → you run `wt sync` on other active worktrees.

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
```

### 3.2. Archive the old plan

Move `private/plans/cozy-sleeping-lark.md` → `private/plans/archived/old-cozy-sleeping-lark.md`

### Phase 3 Checkpoint

**Status:** `[ ] Not started`

- [ ] `git.md` worktree section updated -- `wt` commands are the primary docs, not manual steps
- [ ] Testing section documents `pytest` running directly (not via `docker exec`)
- [ ] `cozy-sleeping-lark.md` archived
- [ ] `git diff .claude/rules/git.md` shows only the worktree section changed

---

## Verification Summary

| Step | Command | Key check |
|------|---------|-----------|
| Phase 1 | `docker compose config` (main repo) | No errors; main stack unchanged |
| Phase 1 | `git status` (main repo) | `docker-compose.override.yaml.example` tracked; `.env` not tracked |
| Phase 2 | `scripts/wt create test-wt` | stack starts automatically; PORT_FRONTEND=3010 in `.env`; `pytest` runs immediately |
| Phase 2 | `scripts/wt list` | test-wt shows slot 1, ports 3010/8010/9010/5442, status up |
| Phase 2 | `scripts/wt create test-wt --full` | all 4 services including parser; frontend at `localhost:3010` |
| Phase 2 | Two stacks simultaneously | `scripts/wt create test-wt-2` gets slot 2; both stacks run without port conflict |
| Phase 2 | Slot reuse | `scripts/wt remove test-wt` then `scripts/wt create test-wt-new` → gets slot 1, not slot 3 |
| Phase 2 | Cargo cache shared | `docker volume ls` shows one `dashjump-gg-cargo-cache` volume, not two |
| Phase 2 | `scripts/wt sync test-wt` | rebases onto main, runs alembic upgrade head, exits cleanly |
| Phase 2 | `scripts/wt remove test-wt` | worktree + branch gone; `postgres-data` volume removed; cargo cache intact |
| Phase 3 | `git diff .claude/rules/git.md` | Only worktree section changed |

---

## Edge Cases

- **Pre-existing worktrees** (`haste-migration`, `souls`, `timing-fix`): `wt list` shows "no .env (pre-dates wt script)". `wt start`/`stop` will error cleanly. To adopt one, add a compliant `.env` manually or create a fresh worktree and switch to it.

- **`VITE_BACKEND_DOMAIN` is dev-server runtime, not build-time.** `vite dev` reads env vars at startup from the container environment -- the override's `environment:` injection works correctly. This is not a production build concern.

- **Concurrent Rust builds share the cargo cache.** Cargo uses file locks; concurrent builds will wait on locks, not corrupt. Slower, but safe.

- **Use `docker compose` (space), not `docker-compose` (hyphen).** The `--project-directory` flag requires the Docker CLI plugin. You're on 5.1.0 which is the current v2 release and is correct.

- **Devcontainer users (collaborators):** `scripts/wt` works identically inside the devcontainer. Sibling worktrees created from within the devcontainer appear at the same paths on the host filesystem. No devcontainer configuration changes required.
