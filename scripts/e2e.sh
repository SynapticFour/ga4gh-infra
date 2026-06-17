#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
COMPOSE_FILE="${ROOT}/docker/docker-compose.yml"
COMPOSE_ENV="${ROOT}/docker/.env.example"

echo "Starting ga4gh-infra stack..."
docker compose -f "${COMPOSE_FILE}" --env-file "${COMPOSE_ENV}" up --build --wait

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
