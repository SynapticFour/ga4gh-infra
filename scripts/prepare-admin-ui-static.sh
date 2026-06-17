#!/usr/bin/env bash
# Ensure admin-ui ships a real htmx bundle (not the repo placeholder).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
HTMX="$ROOT/crates/admin-ui/static/htmx.min.js"
HTMX_VERSION="2.0.4"
HTMX_URL="https://unpkg.com/htmx.org@${HTMX_VERSION}/dist/htmx.min.js"
MIN_BYTES=10000

needs_fetch=false
if [[ ! -f "$HTMX" ]]; then
  needs_fetch=true
elif [[ $(wc -c <"$HTMX" | tr -d ' ') -lt $MIN_BYTES ]]; then
  needs_fetch=true
elif head -n 1 "$HTMX" | grep -q 'htmx placeholder'; then
  needs_fetch=true
fi

if $needs_fetch; then
  echo "Fetching htmx.org@${HTMX_VERSION} for admin-ui …"
  curl -fsSL "$HTMX_URL" -o "$HTMX"
  echo "Wrote $(wc -c <"$HTMX" | tr -d ' ') bytes to crates/admin-ui/static/htmx.min.js"
else
  echo "admin-ui static assets OK (htmx present)."
fi
