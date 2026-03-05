---
paths:
  - ".devcontainer/*"
  - "**/Dockerfile"
  - "docker-compose.yaml"
  - ".github/workflows/*"
---
# CI/CD Pipelines

Continuous Integration and Continuous Deployment workflows.

## Current State

### CI (Continuous Integration) ✅

**Status:** Active on all pull requests

**File:** `.github/workflows/ci.yaml`

**Triggers:**
- Pull requests to `main` branch

**Jobs:**
- Frontend: Build, lint (planned), test (browser tests planned)
- Backend: Lint (ruff), type check (mypy), test (pytest)
- Parser: Format check (rustfmt), lint (clippy), test (cargo test)

### CD (Continuous Deployment) 🚧

**Status:** In progress (separate branch)

**Planned:**
- Automated deployment to staging on merge to `main`
- Manual approval for production deployment
- Image building and registry push
- Kubernetes manifest updates

## CI Workflow

### Overview

```yaml
name: CI

on:
  pull_request:
    branches: [main]

jobs:
  frontend:   # Node.js 24
  backend:    # Python 3.13
  parser:     # Rust
```

### Frontend Job

```yaml
frontend:
  runs-on: ubuntu-latest
  steps:
    - uses: actions/checkout@v4
    - uses: actions/setup-node@v4
      with:
        node-version: '24'
        cache: 'npm'
        cache-dependency-path: frontend/package-lock.json
    - run: npm ci
      working-directory: frontend
    - run: npm run build
      working-directory: frontend
    # TODO: Browser tests require Playwright setup
```

**Current Checks:**
- ✅ Build (Vite production build)

**Planned:**
- 🚧 Linting (`npm run lint`)
- 🚧 Browser tests (`npm test` with Playwright)
- 🚧 Type checking (`tsc --noEmit`)

**Dependencies:**
- Node.js 24
- npm package cache

### Backend Job

```yaml
backend:
  runs-on: ubuntu-latest
  steps:
    - uses: actions/checkout@v4
    - uses: actions/setup-python@v5
      with:
        python-version: '3.13'
        cache: 'pip'
    - run: pip install -r requirements.txt
      working-directory: backend
    - run: ruff check app/
      working-directory: backend
    - run: mypy app/
      working-directory: backend
    - run: pytest
      working-directory: backend
```

**Current Checks:**
- ✅ Linting (ruff)
- ✅ Type checking (mypy)
- ✅ Tests (pytest)

**Planned:**
- 🚧 Coverage reporting (`pytest --cov`)
- 🚧 Security scanning (`bandit`, `safety`)

**Dependencies:**
- Python 3.13
- pip package cache

### Parser Job

```yaml
parser:
  runs-on: ubuntu-latest
  steps:
    - uses: actions/checkout@v4
    - uses: dtolnay/rust-toolchain@stable
    - uses: Swatinem/rust-cache@v2
      with:
        workspaces: parser
    - run: cargo fmt --check
      working-directory: parser
    - run: cargo clippy -- -D warnings
      working-directory: parser
    - run: cargo test
      working-directory: parser
```

**Current Checks:**
- ✅ Format checking (rustfmt)
- ✅ Linting (clippy with warnings as errors)
- ✅ Tests (cargo test)

**Planned:**
- 🚧 Coverage reporting (`cargo tarpaulin`)
- 🚧 Security audit (`cargo audit`)

**Dependencies:**
- Rust stable toolchain
- Cargo cache (registry, git dependencies)

## CI Best Practices

### Caching Strategy

```yaml
# Node.js
- uses: actions/setup-node@v4
  with:
    cache: 'npm'
    cache-dependency-path: frontend/package-lock.json

# Python
- uses: actions/setup-python@v5
  with:
    cache: 'pip'

# Rust
- uses: Swatinem/rust-cache@v2
  with:
    workspaces: parser
```

**Benefits:**
- Faster CI runs (cached dependencies)
- Reduced GitHub Actions usage (billed by minute)
- More reliable (less network dependency)

### Fail Fast

All jobs run in parallel. If any fails, PR cannot be merged.

```
frontend ──┐
           ├──> All pass → PR can merge
backend ───┤
           │
parser ────┘
```

### Matrix Builds (Future)

Test across multiple versions:

```yaml
strategy:
  matrix:
    node-version: [22, 24]
    python-version: ['3.12', '3.13']
    rust-version: [stable, nightly]
```

## CD Workflow (Planned)

### Staging Deployment

```yaml
name: Deploy Staging

on:
  push:
    branches: [main]

jobs:
  build-and-push:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Build Docker images
        run: |
          docker build -t dashjump-backend:${{ github.sha }} ./backend
          docker build -t dashjump-frontend:${{ github.sha }} ./frontend
          docker build -t dashjump-parser:${{ github.sha }} ./parser

      - name: Push to registry
        run: |
          echo ${{ secrets.REGISTRY_TOKEN }} | docker login -u ${{ secrets.REGISTRY_USER }} --password-stdin
          docker push dashjump-backend:${{ github.sha }}
          docker push dashjump-frontend:${{ github.sha }}
          docker push dashjump-parser:${{ github.sha }}

      - name: Deploy to staging
        run: |
          kubectl set image deployment/backend backend=dashjump-backend:${{ github.sha }}
          kubectl set image deployment/frontend frontend=dashjump-frontend:${{ github.sha }}
          kubectl set image deployment/parser parser=dashjump-parser:${{ github.sha }}
```

### Production Deployment

```yaml
name: Deploy Production

on:
  release:
    types: [published]

jobs:
  deploy:
    runs-on: ubuntu-latest
    environment:
      name: production
      url: https://dashjump.gg
    steps:
      # Same as staging, but with production k8s context
      - name: Deploy to production
        run: |
          kubectl config use-context production
          kubectl set image deployment/backend backend=dashjump-backend:${{ github.event.release.tag_name }}
```

**Environment Protection:**
- Manual approval required
- Restricted to specific reviewers
- Production secrets separate from staging

## Image Registry Strategy

### Options

| Registry | Pros | Cons |
|----------|------|------|
| GitHub Container Registry (ghcr.io) | Free for public, integrated with GitHub | Rate limits |
| Docker Hub | Familiar, free tier | Rate limits |
| AWS ECR | Fast in AWS, unlimited private | AWS lock-in |
| GCP Artifact Registry | Fast in GCP, unlimited private | GCP lock-in |

**Recommendation:** Start with GitHub Container Registry, migrate to cloud provider registry when deploying to that cloud.

### Image Tagging Strategy

```bash
# Development builds (main branch)
dashjump-backend:latest
dashjump-backend:sha-abc123f

# Release builds
dashjump-backend:v1.0.0
dashjump-backend:v1.0.0-sha-abc123f

# Environment tags
dashjump-backend:staging
dashjump-backend:production
```

## Secrets Management

### GitHub Secrets

Required secrets:

```
# Docker Registry
REGISTRY_USER
REGISTRY_TOKEN

# Kubernetes
KUBECONFIG_STAGING
KUBECONFIG_PRODUCTION

# Application
DATABASE_URL
STEAM_API_KEY
DEADLOCK_API_KEY
```

### Secret Rotation

- Rotate production secrets every 90 days
- Use separate secrets for staging and production
- Never commit secrets to repository

## Monitoring CI/CD

### GitHub Actions Dashboard

View workflow runs:
- https://github.com/your-org/dashjump-gg/actions

### Notifications

Configure notifications for:
- Failed deployments (Slack, PagerDuty)
- Successful production deployments (Slack)
- Security vulnerabilities (GitHub Security tab)

## Testing Strategy

### CI Tests (Fast)

Run on every PR:
- Unit tests
- Integration tests (mocked external services)
- Linting, type checking

**Target:** < 5 minutes total

### Pre-Deployment Tests (Thorough)

Run before staging deployment:
- Full integration tests (real database)
- Browser tests (Playwright)
- Load tests (k6)

**Target:** < 15 minutes total

### Post-Deployment Tests (Smoke)

Run after deployment:
- Health check endpoints
- Critical user flows (login, view match)
- Database connectivity

**Target:** < 2 minutes total

## Rollback Strategy

### Automated Rollback

If post-deployment tests fail:

```yaml
- name: Rollback on failure
  if: failure()
  run: |
    kubectl rollout undo deployment/backend
    kubectl rollout undo deployment/frontend
    kubectl rollout undo deployment/parser
```

### Manual Rollback

```bash
# Rollback to previous version
kubectl rollout undo deployment/backend

# Rollback to specific version
kubectl rollout undo deployment/backend --to-revision=5
```

## Cost Optimization

### GitHub Actions Usage

- Free tier: 2,000 minutes/month (public repos)
- Paid: $0.008/minute (private repos)

**Optimization:**
- Cache dependencies aggressively
- Run tests in parallel
- Use matrix builds sparingly
- Skip CI for documentation-only changes

### Self-Hosted Runners (Future)

For high-volume CI:
- Run on own infrastructure
- No per-minute charges
- Full control over environment

## Security Scanning

### Container Scanning (Future)

```yaml
- name: Scan backend image
  uses: aquasecurity/trivy-action@master
  with:
    image-ref: dashjump-backend:${{ github.sha }}
    format: 'sarif'
    output: 'trivy-results.sarif'

- name: Upload scan results
  uses: github/codeql-action/upload-sarif@v2
  with:
    sarif_file: 'trivy-results.sarif'
```

### Dependency Scanning

```yaml
# Python
- name: Check for vulnerabilities
  run: safety check

# Node.js
- name: Audit dependencies
  run: npm audit

# Rust
- name: Audit dependencies
  run: cargo audit
```

## Related Documentation

- [INFRA.md](INFRA.md) — Infrastructure overview
- [containers.md](containers.md) — Docker image details
- [deployment.md](deployment.md) — Production deployment
