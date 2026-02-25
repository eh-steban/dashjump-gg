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
cd backend && pip3 install --user -r requirements.txt

# Install frontend dependencies and Playwright browser
cd ../frontend && npm install && npx playwright install chromium
