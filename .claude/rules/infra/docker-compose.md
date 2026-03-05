---
paths:
  - ".devcontainer/*"
  - "**/Dockerfile"
  - "docker-compose.yaml"
  - ".github/workflows/*"
---
# Docker Compose Configuration

Local development orchestration with Docker Compose.

## Overview

Two Docker Compose files work together:

1. **`docker-compose.yaml`** (root) — Production services (backend, frontend, parser, database)
2. **`.devcontainer/docker-compose.yml`** — Development container overlay

When using VSCode devcontainer, both are loaded together to provide:
- Unified dev environment (all three languages)
- Production-like service definitions (database, networking)

## Production Services (docker-compose.yaml)

### Service Definitions

```yaml
services:
  dashjump-frontend:     # React app on port 3000
  dashjump-backend:      # FastAPI on port 8000
  dashjump-parser:       # Rust/Axum on port 9000
  dashjump-db:           # PostgreSQL 16 on port 5432
```

### Service Details

#### Frontend

**Key Points:**
- Hot reload with mounted source code
- Anonymous volume for `node_modules` (prevents host/container conflict)
- Vite dev server bound to `0.0.0.0` (accessible from host)

#### Backend

**Key Points:**
- Connects to database via internal network (`dashjump-db:5432`)
- Calls parser via internal URL (`dashjump-parser:9000`)
- Hot reload via uvicorn's `--reload` flag (in Dockerfile CMD)

#### Parser

**Key Points:**
- Cargo cache volume for faster rebuilds
- `cargo-watch` in Dockerfile provides hot reload
- Replay directories configurable via environment

#### Database

**Key Points:**
- Named volume for data persistence
- Init scripts run on first startup (schema setup, seed data)
- Exposed port for direct database access (debugging, migrations)

### Volumes

**Persistence Strategy:**
- **Named volumes:** Data we want to persist between restarts (database, caches)
- **Bind mounts:** Code we want to hot reload (./backend, ./frontend, ./parser)
- **Anonymous volumes:** Dependencies we don't want to sync with host (/app/node_modules)

## Development Container (.devcontainer/docker-compose.yml)

### Devcontainer Service

**Key Points:**
- Mounts entire project (not just service subdirectories)
- Runs indefinitely (`sleep infinity`) for interactive development
- Connects to database from root compose file
- Elevated privileges for debugging (ptrace, seccomp)

### Compose File Merging

VSCode devcontainer loads **both** compose files:

Result: Devcontainer can access all services on the same network.

## Networking

### Default Bridge Network

All services on the same Docker network:

```
dashjump-frontend    → dashjump-backend:8000
dashjump-backend     → dashjump-parser:9000
dashjump-backend     → dashjump-db:5432
devcontainer         → dashjump-db:5432
devcontainer         → dashjump-parser:9000
```

**Service Discovery:**
- Containers reference each other by service name
- Docker's embedded DNS resolves service names to container IPs
- No hard-coded IPs needed

### Port Mapping

| Service | Internal | External | Purpose |
|---------|----------|----------|---------|
| Frontend | 3000 | 3000 | Browser access |
| Backend | 8000 | 8000 | API access |
| Parser | 9000 | 9000 | Direct testing |
| Database | 5432 | 5432 | Direct queries |

**Production Note:** In production, only frontend (via nginx) and backend would be exposed. Parser and database would be internal only.

## Common Operations

### Start All Services

```bash
docker-compose up
```

### Start Specific Service

```bash
docker-compose up backend
```

### Rebuild After Dependency Changes

```bash
docker-compose up --build backend
```

### View Logs

```bash
docker-compose logs -f backend
docker-compose logs --tail 50 parser
```

### Stop All Services

```bash
docker-compose down
```

### Stop and Remove Volumes (Fresh Start)

```bash
docker-compose down -v
```

### Execute Command in Running Container

```bash
docker-compose exec backend bash
docker-compose exec dashjump-db psql -U deadlock deadlock_db
```

## Development Workflows

### Using Production Containers (docker-compose)

```bash
# Start all services
docker-compose up

# Services auto-reload on code changes
# Frontend: Vite dev server
# Backend: Uvicorn --reload
# Parser: cargo-watch
```

### Using Devcontainer (VSCode)

```bash
# Open in devcontainer
# Command Palette > "Dev Containers: Reopen in Container"

# Terminal 1: Backend
cd backend
uvicorn app.main:app --reload --host 0.0.0.0

# Terminal 2: Frontend
cd frontend
npm run dev -- --host 0.0.0.0

# Terminal 3: Parser
cd parser
cargo watch -x run
```

**When to Use Each:**

| Scenario | Use |
|----------|-----|
| Full stack development | Devcontainer (unified environment) |
| Single service testing | `docker-compose up [service]` |
| Production-like testing | All production containers |
| Database migrations | Devcontainer (has alembic) |

## Environment Variables

### Centralized Configuration (Future)

```bash
# .env file (not committed)
POSTGRES_USER=deadlock
POSTGRES_PASSWORD=deadlockpass
DATABASE_URL=postgresql+psycopg://deadlock:deadlockpass@dashjump-db:5432/deadlock_db
PARSER_BASE_URL=http://dashjump-parser:9000
```

```yaml
# docker-compose.yaml
services:
  dashjump-backend:
    env_file: .env
```

## Troubleshooting

### Port Already in Use

```bash
# Find process using port 8000
lsof -i :8000

# Stop conflicting service or change port
```

### Database Connection Refused

```bash
# Ensure database is running
docker-compose ps

# Check database logs
docker-compose logs dashjump-db

# Restart database
docker-compose restart dashjump-db
```

### Hot Reload Not Working

```bash
# Rebuild container
docker-compose up --build [service]

# Check volume mounts
docker-compose exec [service] ls -la
```

### Permission Issues (Linux)

```bash
# Devcontainer uses UID 1000 to match host
# If your host UID differs, adjust .devcontainer/Dockerfile

# Check your UID
id -u
```

## Production Readiness Checklist

For production deployment, modify compose file:

- [ ] Remove volume mounts (code baked into image)
- [ ] Use secrets for passwords (not environment variables)
- [ ] Add resource limits (memory, CPU)
- [ ] Enable health checks
- [ ] Remove exposed ports for internal services
- [ ] Use production-optimized images (multi-stage builds)
- [ ] Add restart policies (`restart: unless-stopped`)

## Related Documentation

- [containers.md](containers.md) — Individual Dockerfile details
- [devcontainer.md](devcontainer.md) — Development environment
- [deployment.md](deployment.md) — Production deployment
