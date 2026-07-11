#!/bin/sh
set -e

# Default reproduces docker-compose's network exactly (service name
# "backend", resolved by docker's own embedded DNS) - Cloud Run deployments
# override this with the backend service's own HTTPS URL (see
# docs/CLOUD_RUN_DEPLOY.md). nginx.conf substitutes this into a static
# proxy_pass target, resolved once at nginx startup via the OS resolver -
# no separate resolver configuration needed.
export BACKEND_UPSTREAM="${BACKEND_UPSTREAM:-http://backend:3000}"
# Cloud Run injects PORT and requires the container to listen on it;
# docker-compose never sets it, so this defaults to nginx.conf's original
# port 80 (see docker-compose.yml's `ports: - "8880:80"` mapping).
export PORT="${PORT:-80}"

# Writing to a separate generated file (rather than overwriting the template
# in place) keeps this working even when nginx.conf.template is a read-only
# bind mount (docker-compose's local dev setup).
envsubst '${BACKEND_UPSTREAM} ${PORT}' \
    < /etc/nginx/nginx.conf.template > /etc/nginx/nginx.generated.conf

exec nginx -c /etc/nginx/nginx.generated.conf -g 'daemon off;'
