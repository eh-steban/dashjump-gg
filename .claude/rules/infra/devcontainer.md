---
paths:
  - ".devcontainer/*"
  - "**/Dockerfile"
  - "docker-compose.yaml"
  - ".github/workflows/*"
---
# Development Container (Devcontainer)

Unified development environment with Node.js, Python, and Rust.

## Overview

The devcontainer provides a **consistent development environment** for all team members, eliminating "works on my machine" issues.

**Key Features:**
- All three language stacks in one container (Node.js 24, Python 3.13, Rust)
- VSCode extensions pre-installed
- Database access via shared Docker network
- Volume persistence for dependencies (cargo cache)
- Consistent formatting and linting

## Architecture

### Multi-Language Stack

```
ubuntu:22.04
├── Node.js 24              # Frontend development
├── Python 3.13             # Backend development
├── Rust (latest)           # Parser development
├── PostgreSQL client       # Database operations
├── Protobuf compiler       # Replay parsing
└── Build tools             # gcc, make, curl, git
```

### Why Unified?

**Pros:**
- Single container for entire monorepo
- No context switching between services
- Shared tools (git, formatters, linters)
- Faster iteration (no rebuilding multiple containers)

**Cons:**
- Larger image size (~3 GB)
- Longer initial build time

**Trade-off:** Development experience wins. Production containers remain optimized.

## Configuration

### devcontainer.json

**Key Settings:**
- **dockerComposeFile:** Merges production services with devcontainer
- **workspaceFolder:** VSCode opens at repo root (monorepo pattern)
- **mounts:** Share Claude CLI config from host
- **remoteUser:** Non-root user for safety
- **postCreateCommand:** Automatic dependency installation

## User and Permissions

### UID Strategy

```dockerfile
# Use UID 1000 to match typical host user
RUN useradd -m -u 1000 -s /bin/bash lifted
```

**Why UID 1000?**
- Most Linux users have UID 1000
- Files created in container match host permissions
- No ownership conflicts on mounted volumes

**Check Your UID:**
```bash
id -u  # Should output 1000
```

If different, modify `.devcontainer/Dockerfile`:
```dockerfile
RUN useradd -m -u YOUR_UID -s /bin/bash lifted
```

## Volumes and Persistence

### Source Code Volume

```yaml
volumes:
  - ../dashjump-gg:/workspaces/dashjump-gg:cached
```

**`:cached` flag:** Optimizes I/O performance on macOS/Windows (slight delay in file sync acceptable for performance).

### Cargo Cache Volume

```yaml
volumes:
  - cargo-cache:/home/lifted/.cargo/registry
  - cargo-cache:/home/lifted/.cargo/git
```

**Why?**
- Rust dependencies (~500 MB) persist between rebuilds
- Significantly faster `cargo build` after first run
- Shared across devcontainer rebuilds

### Node Modules

**Not mounted** — `npm install` runs in container after creation.

```bash
# postCreateCommand
cd frontend && npm install
```

**Why not mount?**
- Native modules may differ between host and container
- Avoids platform-specific binary conflicts
- Faster container startup (no sync needed)

## Development Workflows

### Starting Services

#### Option 1: Manual (Recommended)

```bash
# Terminal 1: Backend
cd backend
uvicorn app.main:app --reload --host 0.0.0.0 --port 8000

# Terminal 2: Frontend
cd frontend
npm run dev -- --host 0.0.0.0

# Terminal 3: Parser
cd parser
cargo watch -i src/compressed-replays/ -i src/replays/ -x run
```

**Pros:**
- Full control over each service
- See logs for each service separately
- Easy to restart individual services
- Better for debugging

#### Option 2: Production Containers

```bash
# Start production containers from devcontainer terminal
docker-compose up backend frontend parser
```

**Pros:**
- Production-like environment
- Automated startup
- Tests production container configuration

**Cons:**
- Harder to debug
- Logs mixed together
- Can't easily attach debugger

### Database Operations

```bash
# Connect to database
psql postgresql://deadlock:deadlockpass@dashjump-db:5432/deadlock_db

# Run migrations
cd backend
alembic upgrade head

# Create migration
alembic revision --autogenerate -m "description"
```

### Testing

```bash
# Backend
cd backend
pytest

# Frontend
cd frontend
npm run test

# Parser
cd parser
cargo test
```

## Debugging

### Python Debugging

VSCode launch.json (future):

```json
{
  "name": "Backend",
  "type": "debugpy",
  "request": "attach",
  "connect": {
    "host": "localhost",
    "port": 5678
  }
}
```

### Rust Debugging

**Already configured:**
- `SYS_PTRACE` capability enabled
- `seccomp:unconfined` security option

VSCode launch.json (future):

```json
{
  "name": "Parser",
  "type": "lldb",
  "request": "launch",
  "program": "${workspaceFolder}/parser/target/debug/parser",
  "cwd": "${workspaceFolder}/parser"
}
```

## Starting from Host (without VSCode)

When you need to start and exec into the devcontainer manually -- e.g. to run nvim across worktrees -- both compose files must be specified so the devcontainer's dependencies on the main services resolve correctly.

```bash
# Start the devcontainer (and its dependencies)
docker compose -f docker-compose.yaml -f .devcontainer/docker-compose.yml up -d devcontainer

# Exec into it
docker compose -f docker-compose.yaml -f .devcontainer/docker-compose.yml exec devcontainer bash

# Then run nvim (installed at /usr/local/bin/nvim, config bind-mounted from host)
nvim
```

## Rebuilding the Devcontainer

### When to Rebuild

Rebuild when you change:
- `.devcontainer/Dockerfile`
- `.devcontainer/devcontainer.json`
- System dependencies (apt packages)

**Don't rebuild for:**
- Application code changes (hot reload)
- Python/npm dependency changes (reinstall inside container)

### How to Rebuild

```bash
# VSCode Command Palette
"Dev Containers: Rebuild Container"

# Or: Rebuild without cache
"Dev Containers: Rebuild Container Without Cache"
```

### Rebuild Time

| Scenario | Time |
|----------|------|
| First build | ~10 minutes |
| Rebuild (cached) | ~2 minutes |
| Rebuild (no cache) | ~10 minutes |

## Customization

### Adding System Dependencies

```dockerfile
# .devcontainer/Dockerfile
RUN apt-get update && apt-get install -y \
    your-package-here \
    && rm -rf /var/lib/apt/lists/*
```

### Adding VSCode Extensions

```json
// .devcontainer/devcontainer.json
"extensions": [
  "your-extension-id"
]
```

### Adding Shell Aliases

```dockerfile
# .devcontainer/Dockerfile
RUN echo 'alias ll="ls -la"' >> /home/lifted/.bashrc
RUN echo 'alias dk="docker-compose"' >> /home/lifted/.bashrc
```

## Troubleshooting

### Container Won't Start

```bash
# Check Docker is running
docker ps

# Check docker-compose services
docker-compose ps

# View container logs
docker-compose logs devcontainer
```

### Dependencies Not Installing

```bash
# Re-run postCreateCommand
cd backend && pip3 install --user -r requirements.txt
cd frontend && npm install

# Or rebuild container
```

### Database Connection Refused

```bash
# Ensure database is running
docker-compose up dashjump-db

# Check network
docker network ls
docker network inspect dashjump-gg_default
```

### Rust Builds Slow

```bash
# Check cargo cache volume exists
docker volume ls | grep cargo-cache

# If missing, rebuild devcontainer
```

### File Permission Issues

```bash
# Check your UID
id -u

# Should be 1000. If not, modify Dockerfile:
RUN useradd -m -u YOUR_UID -s /bin/bash lifted
```

## Performance Tips

### macOS/Windows

- Use `:cached` flag on volume mounts
- Increase Docker Desktop memory (8 GB recommended)
- Use fast SSD for Docker storage

### Linux

- Native Docker performance (no virtualization overhead)
- No special flags needed

### All Platforms

- Close unused services to save memory
- Use `cargo-watch` with ignore flags to avoid recompiling on file changes
- Run tests in watch mode (`pytest --watch`, `npm test`)

## Security Considerations

### Non-Root User

Always develop as `lifted` user (UID 1000), never root.

### Exposed Ports

Only expose ports needed for development:
- 3000 (frontend)
- 8000 (backend)
- 9000 (parser)
- 5432 (database)

### Secrets

**Never commit:**
- `.env` files
- Database passwords (use environment variables)
- API keys

## Related Documentation

- [containers.md](containers.md) — Individual Dockerfile details
- [docker-compose.md](docker-compose.md) — Service orchestration
- [INFRA.md](INFRA.md) — Infrastructure overview
