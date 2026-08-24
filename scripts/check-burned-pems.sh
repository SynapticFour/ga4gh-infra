#!/usr/bin/env bash
# Refuse local docker/secrets PEMs that match keys committed in e669a58
# (removed from HEAD in 591739b; still public in git history).
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BURNED_COMMIT="${GA4GH_BURNED_PEM_COMMIT:-e669a589aef45c89a59b07bc53e162e671018c78}"

sha256() {
  if command -v shasum >/dev/null 2>&1; then
    shasum -a 256 | awk '{print $1}'
  else
    sha256sum | awk '{print $1}'
  fi
}

failed=0
for name in broker_rs256.pem registry_rs256.pem mock_idp_rs256.pem; do
  path="$ROOT/docker/secrets/$name"
  [[ -f "$path" ]] || continue
  local_hash="$(sha256 <"$path")"
  if ! hist_hash="$(git -C "$ROOT" show "$BURNED_COMMIT:docker/secrets/$name" 2>/dev/null | sha256)"; then
    continue
  fi
  if [[ -n "$hist_hash" && "$local_hash" == "$hist_hash" ]]; then
    echo "check-burned-pems: $name matches a historical git object (burned). Generate a new key with make prepare-secrets after deleting $path." >&2
    failed=1
  fi
done

if [[ "$failed" -ne 0 ]]; then
  exit 1
fi
