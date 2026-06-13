# GA4GH Infra — common development and deployment commands.
# Requires: https://github.com/casey/just

compose := "docker compose -f docker/docker-compose.yml --env-file docker/.env.example"
compose_sqlite := "docker compose -f docker/docker-compose.sqlite.yml --env-file docker/.env.example"
secrets_dir := "docker/secrets"

default:
    @just --list

# Generate dev RSA keys under docker/secrets/ when missing.
prepare-secrets:
    #!/usr/bin/env bash
    set -euo pipefail
    mkdir -p "{{secrets_dir}}"
    keygen() {
        if command -v ga4gh-infra >/dev/null 2>&1; then
            ga4gh-infra keygen --output "$1"
        else
            cargo run -q -p ga4gh-infra-cli -- keygen --output "$1"
        fi
    }
    for name in broker_rs256.pem registry_rs256.pem mock_idp_rs256.pem; do
        path="{{secrets_dir}}/$name"
        if [ ! -f "$path" ]; then
            keygen "$path"
        fi
    done

# Start the full Docker stack (PostgreSQL).
up: prepare-secrets
    {{compose}} up --build --wait

# Start the lighter stack (SQLite visa-registry).
up-sqlite: prepare-secrets
    {{compose_sqlite}} up --build --wait

down:
    {{compose}} down

# Stop stack and remove volumes (database, registry data, etc.).
destroy:
    {{compose}} down -v --remove-orphans

logs *args:
    {{compose}} logs -f {{args}}

# Run workspace unit tests (no Docker required).
test:
    cargo test --workspace

# Run Docker-backed integration tests (testcontainers, ignored by default).
test-integration:
    cargo test -p ga4gh-integration -- --ignored --test-threads=1

# Run Docker stack end-to-end test.
e2e:
    ./scripts/e2e.sh
