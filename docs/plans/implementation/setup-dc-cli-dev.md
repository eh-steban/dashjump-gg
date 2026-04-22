# Plan: Move Dev Dockerfile to Project Root

## Context

The unified dev container Dockerfile (Node + Python + Rust + Claude) lives in `.devcontainer/` but is the primary artifact for the project's development environment. The goal is to promote it to the project root so it's the obvious, first-class entry point for dev setup — while keeping `.devcontainer/` functional for VSCode users as a secondary concern.

**Daily driver workflow (unchanged by this migration):**
```bash
docker compose up -d            # starts frontend, backend, parser, db
claude                          # run from WSL terminal directly
```

**VSCode devcontainer (preserved, secondary):**
- "Reopen in Container" → VSCode uses `.devcontainer/` config → attaches to dev container

The devcontainer service is intentionally NOT added to root `docker-compose.yaml` — it should never start silently. It only starts when VSCode explicitly invokes it via `.devcontainer/docker-compose.yml`.

---

## What Changes

### Files moved (content unchanged)
| From | To |
|------|-----|
| `.devcontainer/Dockerfile` | `./Dockerfile` (project root) |
| `.devcontainer/setup.sh` | `./setup.sh` (project root) |

### Files updated
| File | Change |
|------|--------|
| `.devcontainer/docker-compose.yml` | Update `dockerfile:` path to point at `./Dockerfile` |
| `.devcontainer/devcontainer.json` | Update `postCreateCommand` path |
| `.claude/rules/infra/devcontainer.md` | Update 5 path references |
| `.claude/rules/infra/containers.md` | Update 1 path reference |
| `.claude/rules/infra/docker-compose.md` | Update 1 path reference |

### Files deleted
| File | Reason |
|------|--------|
| `.devcontainer/Dockerfile` | Moved to project root |
| `.devcontainer/setup.sh` | Moved to project root |

### Files NOT changing
| File | Reason |
|------|--------|
| `docker-compose.yaml` | 4-service default stays exactly as-is; no devcontainer service added |
| `.github/workflows/ci.yaml` | No devcontainer references |
| `backend/`, `frontend/`, `parser/` Dockerfiles | Independent production images |

---

## End State of `.devcontainer/`

```
.devcontainer/
├── devcontainer.json    # VSCode config (minor update to postCreateCommand)
└── docker-compose.yml   # Defines devcontainer service pointing at ../Dockerfile
```

The `docker-compose.yml` stays in `.devcontainer/` because VSCode's devcontainer tooling requires a compose file to define the devcontainer service. It cannot be eliminated without moving the service definition into the root compose (which would make it always-on or require profile workarounds). It becomes a lean ~20-line file.

---

## Step-by-Step Implementation

### 1. Copy Dockerfile and setup.sh to project root
No content changes — all paths inside both files are workspace-root-relative already.

### 2. Update `.devcontainer/docker-compose.yml` dockerfile path

Current `.devcontainer/docker-compose.yml` build config:
```yaml
build:
  context: ..
  dockerfile: dashjump-gg/.devcontainer/Dockerfile
```

Docker Compose resolves `context: ..` relative to the compose file's directory (`.devcontainer/`), making it the parent of the project root (`~/Code`). The dockerfile path is then relative to that context, so `dashjump-gg/.devcontainer/Dockerfile` resolves to the current location correctly.

After moving Dockerfile to project root, update to:
```yaml
build:
  context: ..
  dockerfile: dashjump-gg/Dockerfile
```

### 3. Update `devcontainer.json` postCreateCommand

```json
"postCreateCommand": "bash setup.sh"
```
(Was: `"bash .devcontainer/setup.sh"`)

The workspace folder is `/workspaces/dashjump-gg`, so `setup.sh` at the project root resolves correctly.

### 4. Delete old files from `.devcontainer/`
- Delete `.devcontainer/Dockerfile`
- Delete `.devcontainer/setup.sh`

### 5. Update documentation path references
- `.claude/rules/infra/devcontainer.md` — 5 occurrences of `.devcontainer/Dockerfile` → `Dockerfile`
- `.claude/rules/infra/containers.md` — 1 occurrence
- `.claude/rules/infra/docker-compose.md` — 1 occurrence

---

## Verification

1. **Terminal workflow unchanged:** `docker compose up -d` starts only 4 services; `docker compose ps` shows no devcontainer
2. **VSCode path:** "Reopen in Container" → builds from `./Dockerfile` (via `.devcontainer/docker-compose.yml`), workspace opens at `/workspaces/dashjump-gg`, `setup.sh` runs via `postCreateCommand`
3. **Dockerfile is primary:** `./Dockerfile` visible at project root, clearly the dev environment definition
