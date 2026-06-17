#!/usr/bin/env bash
# Vendor crates.io dependencies for offline/reliable Docker image builds.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
VENDOR_DIR="$ROOT/docker/vendor"
STAMP="$VENDOR_DIR/.vendor-stamp"
LOCK="$ROOT/Cargo.lock"

if [[ ! -f "$LOCK" ]]; then
  echo "error: Cargo.lock not found at repo root" >&2
  exit 1
fi

refresh=false
if [[ ! -d "$VENDOR_DIR" ]] || [[ ! -f "$STAMP" ]] || [[ "$LOCK" -nt "$STAMP" ]]; then
  refresh=true
fi

if $refresh; then
  echo "Vendoring crates.io dependencies to docker/vendor (one-time network fetch on host) …"
  rm -rf "$VENDOR_DIR"
  mkdir -p "$VENDOR_DIR"
  (cd "$ROOT" && cargo vendor "$VENDOR_DIR" >/dev/null)
  touch "$STAMP"
  echo "Vendor tree ready ($(du -sh "$VENDOR_DIR" | cut -f1))."
else
  echo "docker/vendor is up to date (Cargo.lock unchanged)."
fi

if [[ ! -f "$ROOT/docker/.cargo/config.toml" ]]; then
  echo "error: missing docker/.cargo/config.toml" >&2
  exit 1
fi

if [[ ! -d "$VENDOR_DIR" ]] || [[ -z "$(ls -A "$VENDOR_DIR" 2>/dev/null | grep -v '^\.vendor-stamp$' || true)" ]]; then
  echo "error: docker/vendor is empty — run: make prepare-vendor" >&2
  exit 1
fi
