#!/bin/bash

# Ensure ~/.claude/settings.json exists on the host before the container bind-mounts it.
# Docker bind-mounting a file that doesn't exist on the host creates a directory instead,
# which breaks Claude Code's settings entirely.
SETTINGS_FILE="$HOME/.claude/settings.json"

if [ ! -f "$SETTINGS_FILE" ]; then
  mkdir -p "$(dirname "$SETTINGS_FILE")"
  echo '{}' > "$SETTINGS_FILE"
  echo "Created empty $SETTINGS_FILE — configure your Claude Code settings here."
fi

# Install backend dependencies
cd /workspaces/dashjump-gg/backend && pip3 install --user -r requirements.txt

# Install dev utilities (image processing for tooling scripts)
pip3 install --user Pillow

# Install frontend dependencies and Playwright browser
# chromium system deps are pre-installed in the Dockerfile image
cd /workspaces/dashjump-gg/frontend && npm install && npx playwright install chromium
