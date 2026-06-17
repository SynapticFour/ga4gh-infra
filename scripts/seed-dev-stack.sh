#!/usr/bin/env bash
# Load idempotent demo data into a running local/test GA4GH stack.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PROFILE="${GA4GH_SEED_PROFILE:-${1:-postgres}}"

cd "${ROOT}"

if command -v seed-dev-stack >/dev/null 2>&1; then
  GA4GH_SEED_PROFILE="${PROFILE}" seed-dev-stack --profile "${PROFILE}"
elif command -v ga4gh-infra >/dev/null 2>&1 && ga4gh-infra seed-dev-stack --help >/dev/null 2>&1; then
  GA4GH_SEED_PROFILE="${PROFILE}" ga4gh-infra seed-dev-stack --profile "${PROFILE}"
else
  GA4GH_SEED_PROFILE="${PROFILE}" cargo run -q -p ga4gh-dev-seed -- --profile "${PROFILE}"
fi
