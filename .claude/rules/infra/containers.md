---
paths:
  - ".devcontainer/*"
  - "**/Dockerfile"
  - "docker-compose.yaml"
  - ".github/workflows/*"
---
# Container Architecture

Detailed documentation of container structure, optimization, and best practices.

## Container Design Philosophy

1. **Minimal production images** — Only include runtime dependencies
2. **Security first** — Non-root users, minimal attack surface
3. **Layer caching** — Optimize build times with proper layer ordering
4. **Reproducibility** — Pin versions, use lock files

## Production Containers

### Backend (Python 3.13)

**File:** `backend/Dockerfile`

**Key Features:**
- Non-root user (`lifted` UID 1000)
- PostgreSQL client for database operations
- Dependencies installed before code copy (better caching)
- Uvicorn ASGI server for FastAPI

**Optimization Opportunities:**
- Multi-stage build to reduce final image size
- Use `python:3.13-slim` base image
- Copy only necessary files (use `.dockerignore`)

### Frontend (Node 24)

**File:** `frontend/Dockerfile`

**Key Features:**
- Alpine Linux base (minimal size)
- Multi-stage build ready (named stage)
- `npm ci` for reproducible installs
- Vite preview server for built assets

### Parser (Rust)

**File:** `parser/Dockerfile`

**Key Features:**
- Non-root user (`lifted` UID 1000)
- Protobuf compiler for replay parsing
- `cargo-watch` for development hot reload
- Replay directories pre-created

## Development Container

**File:** `.devcontainer/Dockerfile`

Unified container with **all three language stacks** for development.

**Key Features:**
- UID 1000 matches typical host user (volume permissions)
- All three language stacks in one image
- User-level Rust/pip installation (avoids duplication)
- Interactive bash shell for development

**Why Unified?**
- Single container for entire monorepo
- No context switching between services
- Consistent environment for all developers
- Fast iteration without rebuilding multiple images

## Container User Strategy

| Container | User | UID | Rationale |
|-----------|------|-----|-----------|
| Devcontainer | `lifted` | 1000 | Matches host user for volume permissions |
| Backend | `lifted` | 1000 | Matches devcontainer, non-root |
| Parser | `lifted` | 1000 | Matches devcontainer, non-root |
| Frontend | `node` | 1000 | Built-in Node image user |

**Security Note:** Never run production containers as root. Even if compromised, damage is limited.

## Image Size Comparison

| Container | Current | Optimized (Future) | Savings |
|-----------|---------|-------------------|---------|
| Backend | ~1.2 GB | ~200 MB | ~83% |
| Frontend | ~200 MB | ~50 MB | ~75% |
| Parser | ~2.5 GB | ~100 MB | ~96% |
| Devcontainer | ~3 GB | N/A | Dev image |

**Optimization Strategies:**
- Multi-stage builds (compile vs runtime)
- Slim/Alpine base images
- Remove build tools from runtime
- Copy only compiled artifacts

## Best Practices

### Dependency Installation Order

```dockerfile
# ✅ Good - dependencies cached separately
COPY package.json package-lock.json ./
RUN npm ci
COPY . .

# ❌ Bad - dependencies reinstall on any code change
COPY . .
RUN npm ci
```

### Clean Up After Install

```dockerfile
# ✅ Good - single RUN layer, cleanup
RUN apt-get update && apt-get install -y \
    build-essential \
    && rm -rf /var/lib/apt/lists/*

# ❌ Bad - multiple layers, no cleanup
RUN apt-get update
RUN apt-get install -y build-essential
```

### Use .dockerignore

Prevent unnecessary files from being copied:

```
# backend/.dockerignore
__pycache__/
*.pyc
.pytest_cache/
.venv/
*.egg-info/

# frontend/.dockerignore
node_modules/
dist/
.cache/
coverage/

# parser/.dockerignore
target/
Cargo.lock
*.swp
```

## Volume Strategy

### Development Volumes

```yaml
# Code volumes (hot reload)
- ./backend:/backend
- ./frontend:/app
- ./parser:/parser

# Dependency caches (persist between rebuilds)
- /app/node_modules          # Frontend dependencies
- cargo-cache:/home/lifted/.cargo  # Rust dependencies
```

### Production Volumes

```yaml
# Data persistence only
- postgres-data:/var/lib/postgresql/data

# No code volumes - baked into image
```

## Health Checks (Future)

Add health checks for production containers:

```dockerfile
# Backend
HEALTHCHECK --interval=30s --timeout=3s \
  CMD curl -f http://localhost:8000/health || exit 1

# Frontend
HEALTHCHECK --interval=30s --timeout=3s \
  CMD curl -f http://localhost:3000/ || exit 1

# Parser
HEALTHCHECK --interval=30s --timeout=3s \
  CMD curl -f http://localhost:9000/health || exit 1
```

## Security Scanning (Future)

Integrate container security scanning:

```yaml
# .github/workflows/security.yaml
- name: Scan Backend Image
  uses: aquasecurity/trivy-action@master
  with:
    image-ref: dashjump-backend:latest
    format: 'sarif'
    output: 'trivy-results.sarif'
```

## Related Documentation

- [docker-compose.md](docker-compose.md) — Orchestration
- [devcontainer.md](devcontainer.md) — Development setup
- [deployment.md](deployment.md) — Production deployment
