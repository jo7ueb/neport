# Build stage
FROM node:24-slim AS builder

WORKDIR /app

# Baked into the footer as the displayed app version. Vite exposes
# VITE_-prefixed env vars to client code as import.meta.env.VITE_GIT_COMMIT
# automatically; no vite.config.ts change needed. Pass the real hash with
# `docker compose build --build-arg GIT_COMMIT=$(git rev-parse --short HEAD) frontend`;
# omitted, it just shows as "dev". GIT_DIRTY=1 additionally flags the footer
# when the working tree had uncommitted changes at build time (see Makefile,
# which sets both from `git status --porcelain`).
ARG GIT_COMMIT=dev
ARG GIT_DIRTY=0
ENV VITE_GIT_COMMIT=$GIT_COMMIT
ENV VITE_GIT_DIRTY=$GIT_DIRTY

# Install dependencies first for better layer caching
COPY frontend/package.json frontend/package-lock.json* ./
RUN npm install

# Copy project files and build
COPY frontend/. .
RUN npm run build

# Runtime stage
FROM nginx:alpine

COPY --from=builder /app/dist /usr/share/nginx/html

# nginx.conf is a template (see its own comments): ${BACKEND_UPSTREAM} and
# ${NGINX_RESOLVER} are filled in at container start by docker-entrypoint.sh,
# defaulting to docker-compose's exact current values when unset. The build
# context is the repo root (see docker-compose.yml) so this can reach the
# root-level nginx.conf, which both docker-compose (bind mount) and Cloud Run
# (this baked-in copy) need to share.
COPY nginx.conf /etc/nginx/nginx.conf.template
COPY frontend/docker-entrypoint.sh /docker-entrypoint.sh
RUN chmod +x /docker-entrypoint.sh

HEALTHCHECK --interval=5s --timeout=5s --retries=5 \
    CMD wget --quiet --tries=1 --spider http://localhost/health || exit 1

ENTRYPOINT ["/docker-entrypoint.sh"]
