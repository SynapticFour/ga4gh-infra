#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
COMPOSE_FILE="${ROOT}/docker/docker-compose.yml"
COMPOSE_ENV="${ROOT}/docker/.env.example"

register_service() {
  local id="$1"
  local name="$2"
  local artifact="$3"
  local url="$4"
  local spec_version="${5:-1.0.0}"

  GA4GH_SERVICE_ID="$id" \
  GA4GH_SERVICE_NAME="$name" \
  GA4GH_SERVICE_ARTIFACT="$artifact" \
  GA4GH_SERVICE_URL="$url" \
  GA4GH_SERVICE_SPEC_VERSION="$spec_version" \
    "${ROOT}/docker/scripts/register-service.sh"
}

echo "Starting ga4gh-infra stack..."
docker compose -f "${COMPOSE_FILE}" --env-file "${COMPOSE_ENV}" up --build --wait

echo "Registering services with the service registry..."
export SERVICE_REGISTRY_URL=http://localhost:8083
export SERVICE_REGISTRY_REGISTRATION_KEY=dev-service-registry-key
register_service "org.localhost.aai-broker" "GA4GH AAI Broker" "passport" "http://localhost:8080" "1.2"
register_service "org.localhost.visa-registry" "GA4GH Visa Registry" "visa" "http://localhost:8081" "1.0"
register_service "org.localhost.duo-service" "GA4GH DUO Service" "duo" "http://localhost:8082" "1.0"
register_service "org.localhost.access-decision-service" "GA4GH Access Decision Service" "ads" "http://localhost:8090" "1.0"
register_service "org.localhost.sample-resource" "GA4GH Sample Resource" "resource" "http://localhost:8084" "1.0"

echo "Running end-to-end tests..."
(
  cd "${ROOT}"
  GA4GH_VISA_API_KEY=dev-visa-api-key \
  GA4GH_ADS_API_KEY=dev-ads-api-key \
    cargo test -p ga4gh-e2e -- --ignored --test-threads=1
)

echo "E2E stack test completed successfully."
