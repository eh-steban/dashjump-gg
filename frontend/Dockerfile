FROM node:24
WORKDIR /app

# Rename built-in node user to lifted (UID 1000) to match other services.
# -d/-m relocates the home directory so Playwright's browser cache path is consistent.
RUN usermod -l lifted -d /home/lifted -m node && groupmod -n lifted node

COPY package*.json ./
RUN npm ci

# Install Playwright system dependencies as root, then browser binary as lifted user.
# --with-deps requires apt; the browser binary must be owned by the runtime user.
RUN npx playwright install-deps chromium
RUN chown -R lifted:lifted /app
USER lifted
RUN npx playwright install chromium

COPY --chown=lifted:lifted . .
RUN npm run build

CMD ["npm", "run", "dev", "--", "--host", "0.0.0.0"]
