# GIT_COMMIT/GIT_DIRTY are baked into the frontend footer (see
# frontend/Dockerfile, frontend/src/components/Layout.tsx). `docker compose
# build`/`up --build` invoked directly (without going through this Makefile)
# still work fine - the frontend just falls back to showing "dev" with no
# dirty indicator instead of a real hash.
GIT_COMMIT := $(shell git rev-parse --short HEAD 2>/dev/null || echo dev)
GIT_DIRTY := $(shell git status --porcelain 2>/dev/null | grep -q . && echo 1 || echo 0)

.PHONY: up down build migrate logs ps test test-up test-down cov site site-serve

## Build images (frontend gets the current commit hash) and start the dev stack.
up: build
	docker compose up -d

## Stop the dev stack (containers only, volumes are left alone).
down:
	docker compose down

## Build images without starting anything.
build:
	docker compose build --build-arg GIT_COMMIT=$(GIT_COMMIT) --build-arg GIT_DIRTY=$(GIT_DIRTY) frontend
	docker compose build backend migrator

## Run pending DB migrations against the dev DB.
migrate:
	docker compose run --rm migrator

logs:
	docker compose logs -f

ps:
	docker compose ps

## Start the integration-test stack (separate test-db, relaxed rate limits).
test-up:
	docker compose -f docker-compose.yml -f docker-compose.test.yml build migrator
	docker compose -f docker-compose.yml -f docker-compose.test.yml run --rm migrator
	docker compose -f docker-compose.yml -f docker-compose.test.yml up -d

## Run the backend integration test suite against the running test stack
## (run `make test-up` first).
test:
	cd backend && TEST_BASE_URL=http://localhost:3000 cargo test

## Stop the integration-test stack (containers only, volumes are left alone).
test-down:
	docker compose -f docker-compose.yml -f docker-compose.test.yml down

## Run the backend unit tests + integration tests together with code
## coverage measurement (cargo-llvm-cov), in a throwaway container that
## builds and runs its own instrumented server against test-db (see
## docker-compose.coverage.yml, backend/scripts/run-coverage.sh). Report is
## written to backend/coverage/ (html/index.html + lcov.info) on the host.
## Starts and tears down its own test-db - no need to run make test-up first.
cov:
	docker compose -f docker-compose.yml -f docker-compose.test.yml build migrator
	docker compose -f docker-compose.yml -f docker-compose.test.yml run --rm migrator
	docker compose -f docker-compose.yml -f docker-compose.test.yml up -d test-db
	docker compose -f docker-compose.yml -f docker-compose.test.yml -f docker-compose.coverage.yml build coverage
	docker compose -f docker-compose.yml -f docker-compose.test.yml -f docker-compose.coverage.yml run --rm coverage
	docker compose -f docker-compose.yml -f docker-compose.test.yml down

## Generate the multilingual Hugo site and its static OpenAPI/Redoc reference.
site:
	mkdir -p site/static/ja/developer/api site/static/en/developer/api
	cargo run --manifest-path backend/Cargo.toml -- --export-openapi=site/static/ja/developer/api/openapi.json
	cp site/static/ja/developer/api/openapi.json site/static/en/developer/api/openapi.json
	npx --yes @redocly/cli@2.36.0 lint --config=site/redocly.yaml site/static/ja/developer/api/openapi.json
	npx --yes @redocly/cli@2.36.0 lint --config=site/redocly.yaml site/static/en/developer/api/openapi.json
	npx --yes @redocly/cli@2.36.0 build-docs site/static/ja/developer/api/openapi.json --output=site/static/ja/developer/api/index.html --title='Neport API (日本語)'
	npx --yes @redocly/cli@2.36.0 build-docs site/static/en/developer/api/openapi.json --output=site/static/en/developer/api/index.html --title='Neport API (English)'
	hugo --source site --destination "$(CURDIR)/site/public" --minify --baseURL='https://jo7ueb.github.io/neport/'

## Serve the Hugo site locally (run `make site` first for API pages).
site-serve:
	hugo server --source site --baseURL='http://localhost:1313/' --buildDrafts
