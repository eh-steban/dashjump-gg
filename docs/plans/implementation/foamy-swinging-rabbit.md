# Neovim in Devcontainer Plan

## Context

Developers want neovim available in the devcontainer for terminal-based code editing (paired with tmux in WSL2). Each developer should bring their own neovim config from the host, while plugin data stays isolated per-container to avoid binary compatibility issues between host and container neovim versions.

**Goals:**
- `nvim` available inside the devcontainer with a recent version (0.10+)
- Developer's `~/.config/nvim/` from the host is mounted and usable immediately
- Plugin data (treesitter parsers, lazy.nvim cache, Mason binaries) persists across container rebuilds without conflicting with host neovim
- Developers without neovim configured locally are unaffected (empty config dir is fine)

**Branch:** `chore/neovim-devcontainer` (or quick-fix directly on main -- this is infra-only, no app code)

---

## Scope

Only the devcontainer is in scope. Service containers (backend, frontend, parser) are slim runtime containers, not interactive terminals -- no changes there.

---

## Design Decision: Named Volume vs. Bind Mount for Plugin Data

`~/.config/nvim/` (the user's config) is a bind mount from the host -- same pattern as `~/.claude`.

`~/.local/share/nvim/` (plugin data -- treesitter `.so` files, lazy.nvim cache, Mason-installed LSP binaries) is a **named Docker volume**, NOT a host bind mount.

**Why:** Even on WSL2 where host and container are both Linux, treesitter compiled parsers and Mason binaries are tied to the neovim version that compiled them. If the host has nvim 0.9 and the container has nvim 0.10, sharing the same `~/.local/share/nvim` causes "API level mismatch" errors on treesitter parsers. A named volume gives the container its own isolated plugin data that exactly matches the container's neovim version.

---

## Critical Files

| File | Change |
|------|--------|
| `.devcontainer/Dockerfile` | Install neovim tarball (root phase) + add `pynvim` + `rust-analyzer` + pre-create user dirs |
| `.devcontainer/docker-compose.yml` | Add config bind mount + `nvim-data` named volume |
| `.devcontainer/setup.sh` | Guard: ensure `~/.config/nvim` exists on host before bind-mount |
| `backend/Dockerfile` | Add `vim` to existing apt-get block |
| `frontend/Dockerfile` | Add new apt-get block with `vim` |
| `parser/Dockerfile` | Add `vim` to existing apt-get block |

---

## Phase A -- Devcontainer Dockerfile

### A1. Install neovim (root phase)

After the Google Chrome install block and before the Node.js block, add a new root-phase `RUN` that downloads the neovim pre-built tarball from GitHub releases and installs it to `/opt/nvim`.

- Download `nvim-linux-x86_64.tar.gz` for a pinned stable release (e.g. `v0.10.4`) from `https://github.com/neovim/neovim/releases/download/<version>/nvim-linux-x86_64.tar.gz`
- Unpack to `/opt/nvim-linux-x86_64`
- Symlink: `ln -s /opt/nvim-linux-x86_64/bin/nvim /usr/local/bin/nvim`
- Clean up the tarball

Why not the Ubuntu PPA: `ppa:neovim-ppa/stable` currently ships 0.9.x and `ppa:neovim-ppa/unstable` is genuinely unstable. The GitHub tarball gives a pinned, known-good build with no extra apt layers.

### A2. Install pynvim and rust-analyzer (user phase)

Both additions go in the user phase (after `USER lifted` at line 86), so no root permissions are involved.

**pynvim:** Extend the existing pip install line (`.devcontainer/Dockerfile:93`):
```
python3 -m pip install --user pyright pynvim
```

**Why not `apt-get install python3-neovim`:** That package installs pynvim for the Ubuntu system Python (python3.10 on 22.04), not our python3.13. Neovim would find the wrong Python host and fail to load Python plugins.

**rust-analyzer:** Add to the existing `cargo install` line (`.devcontainer/Dockerfile:91-92`):
```
rustup component add rust-analyzer
```

This is the only LSP binary not already present in the container. `pyright` and `typescript-language-server` are already in PATH. Neovim's `nvim-lspconfig` will find all three automatically -- no Mason needed for the core languages.

### A3. Pre-create neovim user directories (user-setup block)

In the existing `RUN useradd ...` block (`.devcontainer/Dockerfile:78-83`), add two `mkdir` lines before the `chown`:

```
mkdir -p /home/lifted/.config/nvim
mkdir -p /home/lifted/.local/share/nvim
```

The existing `chown -R lifted:lifted /home/lifted` already covers these. This prevents Docker from creating them as root-owned when the volume/mount first attaches -- the same trap documented in `MEMORY.md` for `.claude`.

---

## Phase B -- Service Containers (vim)

Basic `vim` in the three service containers for production/ops use. No config mounting needed -- just the binary.

### B1. backend/Dockerfile

`backend/Dockerfile:8` already has an apt install block. Add `vim` to it:

```dockerfile
RUN apt-get update && apt-get install -y postgresql-client vim
```

### B2. parser/Dockerfile

`parser/Dockerfile:5-7` already has an apt install block. Add `vim`:

```dockerfile
RUN apt-get update && \
    apt-get install -y protobuf-compiler vim && \
    rm -rf /var/lib/apt/lists/*
```

### B3. frontend/Dockerfile

No apt block exists. Add one in the root phase before `USER lifted` (currently line 15):

```dockerfile
RUN apt-get update && apt-get install -y vim && rm -rf /var/lib/apt/lists/*
```

---

## Phase D -- docker-compose.yml

### B1. Add config bind mount

Under the `devcontainer` service `volumes:` block, add after the `~/.claude` mount:

```yaml
# Developer's neovim config from host -- empty/missing is fine
- ~/.config/nvim:/home/lifted/.config/nvim
```

### B2. Add plugin data named volume

Also under `volumes:`:

```yaml
# Isolated plugin data -- avoids nvim version conflicts with host binaries
- nvim-data:/home/lifted/.local/share/nvim
```

### B3. Declare named volume

At the top-level `volumes:` block (currently only has `cargo-cache:`), add:

```yaml
volumes:
  cargo-cache:
  nvim-data:
```

---

## Phase E -- setup.sh

### C1. Guard against missing config dir

Docker bind-mounting a missing source directory creates it as a root-owned empty directory on the host. Add a guard to `setup.sh` to ensure `~/.config/nvim` exists before the container starts:

```bash
# Ensure ~/.config/nvim exists on the host before the container bind-mounts it.
# Docker creates missing bind-mount source directories as root -- harmless but surprising.
if [ ! -d "$HOME/.config/nvim" ]; then
  mkdir -p "$HOME/.config/nvim"
  echo "Created empty ~/.config/nvim -- add your neovim config or init.lua here."
fi
```

---

## Verification

After rebuilding the devcontainer (`Dev Containers: Rebuild Container Without Cache`):

| Check | Where | Command | Expected |
|-------|-------|---------|----------|
| nvim available | devcontainer | `nvim --version` | `NVIM v0.10.x` |
| Config visible | devcontainer | `ls ~/.config/nvim/` | Shows host init.lua (or empty) |
| Plugin data writable | devcontainer | `nvim +":checkhealth" +qa` | No permission errors |
| pynvim present | devcontainer | `python3 -c "import pynvim"` | No error |
| rust-analyzer present | devcontainer | `rust-analyzer --version` | Prints version |
| Plugin data persists | devcontainer | Rebuild container, re-open nvim | Plugins still installed |
| vim in backend | backend container | `docker compose exec dashjump-backend vim --version` | Prints vim version |
| vim in frontend | frontend container | `docker compose exec dashjump-frontend vim --version` | Prints vim version |
| vim in parser | parser container | `docker compose exec dashjump-parser vim --version` | Prints vim version |

---

## Notes

- Developers who already have a neovim config on the host get it automatically on next rebuild
- Developers without neovim installed locally will see an empty `~/.config/nvim` created on their host by `setup.sh` -- a clean starting point
- Plugin managers (lazy.nvim, packer) will install plugins into the named volume on first neovim open after a fresh container build -- this is expected behavior
- If a developer wants shada history (marks, search history) to also persist, add a second named volume `nvim-state` at `~/.local/state/nvim` -- not included here since it's optional

### LSP and Linters for Neovim

The devcontainer already has the core LSP binaries installed at the Docker level -- the same ones VSCode uses via `devcontainer.json` extensions:

| Language | Binary | Where installed |
|----------|--------|-----------------|
| Python | `pyright` | pip user install (already present) |
| TypeScript/JS | `typescript-language-server` | npm global (already present) |
| Rust | `rust-analyzer` | rustup component (added in this plan) |

**No Mason needed for these.** Neovim's `nvim-lspconfig` will find all three in PATH automatically. If a developer's personal config uses Mason, it will install its own copies into the named volume -- that's fine, just redundant.

**Linters** (flake8, ruff, eslint) live in project dependencies and are available when the project is running. No Dockerfile changes needed -- they're already in PATH inside the container.

**VSCode extensions vs. neovim:** The `devcontainer.json` extensions (Pylance, rust-analyzer, etc.) are VSCode-specific and irrelevant to neovim. Neovim's LSP setup lives entirely in the developer's `~/.config/nvim/`.
