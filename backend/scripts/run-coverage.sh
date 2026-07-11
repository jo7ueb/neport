#!/bin/sh
set -e

echo "==> installing coverage instrumentation env"
eval "$(cargo llvm-cov show-env --export-prefix)"

echo "==> building the instrumented server binary"
cargo build --bin neport-backend

echo "==> starting the instrumented server in the background"
./target/debug/neport-backend &
SERVER_PID=$!
trap 'kill -TERM "$SERVER_PID" 2>/dev/null || true' EXIT

echo "==> waiting for the server to become healthy"
i=0
until curl -sf http://localhost:3000/health > /dev/null 2>&1; do
  i=$((i + 1))
  if [ "$i" -ge 30 ]; then
    echo "server did not become healthy in time" >&2
    exit 1
  fi
  sleep 1
done

echo "==> running unit tests + integration tests (backend/tests/*.rs) together"
TEST_BASE_URL=http://localhost:3000 cargo test

echo "==> stopping the instrumented server so profile data flushes"
kill -TERM "$SERVER_PID"
wait "$SERVER_PID" 2>/dev/null || true
trap - EXIT

echo "==> generating coverage report"
mkdir -p /app/coverage-report
cargo llvm-cov report --summary-only
cargo llvm-cov report --html --output-dir /app/coverage-report
cargo llvm-cov report --lcov --output-path /app/coverage-report/lcov.info

echo "==> HTML report written to backend/coverage/html/index.html (host)"
