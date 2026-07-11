# Build stage
# Pinned to -trixie (not the floating `rust:latest`, whose Debian base can
# move out from under this pin) so the builder's glibc always matches the
# debian:trixie-slim runtime stage below - a mismatch there makes the
# built binary fail to even start ("GLIBC_x.xx not found").
FROM rust:1-trixie as builder

WORKDIR /app

# Copy project files
COPY Cargo.toml ./
COPY src ./src
COPY migrations ./migrations

# Build release binary
RUN cargo build --release 2>&1

# Runtime stage
FROM debian:trixie-slim

# Install runtime dependencies. postgresql-client-16 (trixie's own default
# is 17) is pinned via the official PGDG apt repo so pg_dump/pg_restore
# match the db/test-db server's major version exactly (see backend/src/backup)
# - used for the backup/restore feature.
RUN apt-get update && apt-get install -y \
    ca-certificates \
    curl \
    libpq5 \
    wget \
    gnupg \
    && wget -qO- https://www.postgresql.org/media/keys/ACCC4CF8.asc | gpg --dearmor -o /etc/apt/trusted.gpg.d/postgresql.gpg \
    && echo "deb http://apt.postgresql.org/pub/repos/apt trixie-pgdg main" > /etc/apt/sources.list.d/pgdg.list \
    && apt-get update \
    && apt-get install -y postgresql-client-16 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy binary from builder
COPY --from=builder /app/target/release/neport-backend .
COPY migrations ./migrations

# Health check endpoint
HEALTHCHECK --interval=5s --timeout=5s --retries=5 \
    CMD curl -f http://localhost:3000/health || exit 1

# Entrypoint: run migrations then start server
COPY entrypoint.sh /app/entrypoint.sh
RUN chmod +x /app/entrypoint.sh

ENTRYPOINT ["/app/entrypoint.sh"]
