#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
COMPOSE_FILE="${ROOT}/docker/docker-compose.yml"
COMPOSE_ENV="${ROOT}/docker/.env.example"

echo "Preparing dev RSA keys (gitignored)..."
make -C "${ROOT}" prepare-secrets

echo "Starting ga4gh-infra stack..."
if ! docker compose -f "${COMPOSE_FILE}" --env-file "${COMPOSE_ENV}" up --build --wait; then
  echo "e2e: compose up failed — visa-registry / mock-idp logs:" >&2
  docker compose -f "${COMPOSE_FILE}" --env-file "${COMPOSE_ENV}" logs --no-color visa-registry mock-idp postgres || true
  exit 1
fi

echo "Seeding demo data..."
GA4GH_SEED_PROFILE=postgres "${ROOT}/scripts/seed-dev-stack.sh" postgres

echo "Running end-to-end tests..."
(
  cd "${ROOT}"
  GA4GH_VISA_API_KEY=dev-visa-api-key \
  GA4GH_ADS_API_KEY=dev-ads-api-key \
  GA4GH_ADMIN_UI_URL=http://localhost:8095 \
    cargo test -p ga4gh-e2e -- --ignored --test-threads=1
)

echo "E2E stack test completed successfully."
