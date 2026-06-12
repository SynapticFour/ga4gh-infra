#!/bin/sh
set -eu

REGISTRY_URL="${SERVICE_REGISTRY_URL:-http://service-registry:8083}"
REGISTRY_KEY="${SERVICE_REGISTRY_REGISTRATION_KEY:-dev-service-registry-key}"
SERVICE_ID="${GA4GH_SERVICE_ID:?GA4GH_SERVICE_ID is required}"
SERVICE_NAME="${GA4GH_SERVICE_NAME:?GA4GH_SERVICE_NAME is required}"
SERVICE_URL="${GA4GH_SERVICE_URL:?GA4GH_SERVICE_URL is required}"
SERVICE_ARTIFACT="${GA4GH_SERVICE_ARTIFACT:?GA4GH_SERVICE_ARTIFACT is required}"
SERVICE_VERSION="${GA4GH_SERVICE_VERSION:-0.1.0}"
SERVICE_SPEC_VERSION="${GA4GH_SERVICE_SPEC_VERSION:-1.0.0}"

until curl -fsS "${REGISTRY_URL}/service-info" >/dev/null; do
  echo "waiting for service registry..."
  sleep 2
done

curl -fsS -X POST "${REGISTRY_URL}/services" \
  -H "Content-Type: application/json" \
  -H "X-API-Key: ${REGISTRY_KEY}" \
  -d "{
    \"id\": \"${SERVICE_ID}\",
    \"name\": \"${SERVICE_NAME}\",
    \"type\": {
      \"group\": \"org.ga4gh\",
      \"artifact\": \"${SERVICE_ARTIFACT}\",
      \"version\": \"${SERVICE_SPEC_VERSION}\"
    },
    \"organization\": {
      \"name\": \"GA4GH Infra\",
      \"url\": \"https://ga4gh.org\"
    },
    \"version\": \"${SERVICE_VERSION}\",
    \"url\": \"${SERVICE_URL}\",
    \"environment\": \"development\"
  }"

echo "registered ${SERVICE_ID}"
