#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# Preflight checks for ga4gh-infra Africa-mode / edge deployment.

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ERR=0

info() { printf '[preflight] %s\n' "$*"; }
warn() { printf '[preflight] WARN: %s\n' "$*" >&2; ERR=1; }
ok()   { printf '[preflight] OK: %s\n' "$*"; }

# Architecture
ARCH="$(uname -m)"
case "$ARCH" in
  aarch64|arm64|armv7l|x86_64) ok "architecture $ARCH supported" ;;
  *) warn "architecture $ARCH may not have prebuilt binaries" ;;
esac

# Disk (need ~500 MB for SQLite + keys)
if command -v df >/dev/null 2>&1; then
  AVAIL_KB="$(df -k "${HOME}" | awk 'NR==2 {print $4}')"
  if [[ "${AVAIL_KB:-0}" -lt 524288 ]]; then
    warn "less than 512 MB free disk under ${HOME}"
  else
    ok "disk space sufficient"
  fi
fi

# Co-deploy ports (8180 block)
for PORT in 8180 8181 8182 8183 8190 9100; do
  if command -v lsof >/dev/null 2>&1 && lsof -iTCP:"$PORT" -sTCP:LISTEN >/dev/null 2>&1; then
    warn "port $PORT already in use"
  fi
done
ok "co-deploy port check complete"

# SQLite write test
DATA_DIR="${GA4GH_DATA_DIR:-${HOME}/.config/ga4gh-infra/data}"
mkdir -p "$DATA_DIR"
TEST_FILE="${DATA_DIR}/.preflight-write"
if echo test >"$TEST_FILE" 2>/dev/null; then
  rm -f "$TEST_FILE"
  ok "SQLite data directory writable: $DATA_DIR"
else
  warn "cannot write to $DATA_DIR"
fi

# Binary
if command -v ga4gh-infra >/dev/null 2>&1; then
  ok "ga4gh-infra binary found: $(command -v ga4gh-infra)"
else
  warn "ga4gh-infra binary not in PATH (run install.sh or cargo build -p ga4gh-infra-cli)"
fi

if [[ "$ERR" -ne 0 ]]; then
  info "preflight completed with warnings"
  exit 1
fi

info "preflight passed"
exit 0
