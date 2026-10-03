#!/usr/bin/env bash
# Local gate before a ga4gh-infra-v* tag.
# Covers the fmt, clippy, test, and cargo-audit checks that no longer run on
# push to main or on pull_request. Docker e2e, coverage, and ARM stay
# workflow_dispatch (see docs/CI.md).
set -euo pipefail
# shellcheck disable=SC1091
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/hooks/macos-sdk.sh"
use_linkable_macos_sdk
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

echo "verify-release: cargo fmt --check"
cargo fmt --all -- --check

echo "verify-release: cargo clippy"
cargo clippy --workspace --all-targets -- -D warnings

echo "verify-release: workspace tests"
cargo test --workspace

echo "verify-release: library tests (all features)"
cargo test -p ga4gh-types -p ga4gh-clearinghouse --all-features

if cargo audit --version >/dev/null 2>&1; then
  echo "verify-release: cargo audit"
  cargo audit \
    --ignore RUSTSEC-2023-0071 \
    --ignore RUSTSEC-2025-0111 \
    --ignore RUSTSEC-2025-0134
else
  echo "verify-release: cargo-audit not installed; skipped (cargo install cargo-audit)"
fi

echo "verify-release: OK"
