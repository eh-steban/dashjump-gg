# Unified Development Container for dashjump.gg Monorepo
# Contains: Node.js 24, Python 3.13, Rust (latest)
# Note: Production containers (frontend/backend/parser/Dockerfile) remain separate

FROM ubuntu:22.04

ENV DEBIAN_FRONTEND=noninteractive

# Install common build tools and dependencies
RUN apt-get update && apt-get install -y \
    build-essential \
    curl \
    wget \
    git \
    iputils-ping \
    ca-certificates \
    software-properties-common \
    postgresql-client \
    protobuf-compiler \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Install Playwright system dependencies
# These are required for Chromium/Chrome to run in browser tests and playwright-mcp
RUN apt-get update && apt-get install -y \
    libnss3 \
    libnspr4 \
    libatk1.0-0 \
    libatk-bridge2.0-0 \
    libcups2 \
    libdrm2 \
    libdbus-1-3 \
    libxkbcommon0 \
    libxcomposite1 \
    libxdamage1 \
    libxfixes3 \
    libxrandr2 \
    libgbm1 \
    libasound2 \
    libpango-1.0-0 \
    libcairo2 \
    && rm -rf /var/lib/apt/lists/*

# Install Google Chrome stable
# playwright-mcp uses channel:'chrome' which looks for system Chrome at /opt/google/chrome/chrome
RUN curl -fsSL https://dl.google.com/linux/linux_signing_key.pub \
        | gpg --dearmor -o /usr/share/keyrings/google-chrome.gpg \
    && echo "deb [arch=amd64 signed-by=/usr/share/keyrings/google-chrome.gpg] \
        http://dl.google.com/linux/chrome/deb/ stable main" \
        > /etc/apt/sources.list.d/google-chrome.list \
    && apt-get update \
    && apt-get install -y google-chrome-stable \
    && rm -rf /var/lib/apt/lists/*

# === Node.js 24 ===
RUN curl -fsSL https://deb.nodesource.com/setup_24.x | bash - \
    && apt-get install -y nodejs \
    && rm -rf /var/lib/apt/lists/*

# === Python 3.13 ===
# Note: Don't install python3-pip from apt - it depends on distutils which was removed in Python 3.12+
# Use ensurepip (bundled with Python) to bootstrap pip instead
RUN add-apt-repository ppa:deadsnakes/ppa \
    && apt-get update \
    && apt-get install -y \
        python3.13 \
        python3.13-venv \
        python3.13-dev \
    && rm -rf /var/lib/apt/lists/* \
    && update-alternatives --install /usr/bin/python3 python3 /usr/bin/python3.13 1 \
    && update-alternatives --install /usr/bin/python python /usr/bin/python3.13 1 \
    && python3 -m ensurepip --upgrade

# === Create development user ===
# Use UID 1000 to match typical host user (avoids permission issues with mounted volumes)
# Create user early so Rust and pip install to user directories (avoids duplication)
RUN useradd -m -u 1000 -s /bin/bash lifted \
    && mkdir -p /home/lifted/.local/bin \
    && mkdir -p /home/lifted/.claude \
    && touch /home/lifted/.bashrc \
    && touch /home/lifted/.bash_history \
    && chown -R lifted:lifted /home/lifted

# Switch to user for remaining setup
USER lifted
WORKDIR /workspaces/dashjump-gg
ENV PATH="/home/lifted/.cargo/bin:/home/lifted/.local/bin:${PATH}"

# === Rust and pip for user ===
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y \
    && cargo install cargo-watch \
    && python3 -m pip install --user --upgrade pip \
    && python3 -m pip install --user pyright

# Set npm global prefix via environment so it persists across all RUN layers
ENV npm_config_prefix=/home/lifted/.local
RUN npm install -g typescript-language-server typescript
RUN curl -fsSL https://claude.ai/install.sh | bash

# Verification (visible in build output)
RUN echo "Node: $(node --version)" \
    && echo "npm: $(npm --version)" \
    && echo "Python: $(python3 --version)" \
    && echo "pip: $(pip --version)" \
    && echo "Rust: $(rustc --version)" \
    && echo "Cargo: $(cargo --version)"

CMD ["/bin/bash"]
