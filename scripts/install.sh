#!/usr/bin/env bash
# Install the ga4gh-infra all-in-one binary and default native configuration.
#
#   curl -sSL https://raw.githubusercontent.com/<org>/ga4gh-infra/main/scripts/install.sh | sh
#
# Environment:
#   GA4GH_INFRA_REPO   GitHub repo (default: SynapticFour/ga4gh-infra)
#   GA4GH_INFRA_VERSION  Release tag version without v prefix (default: latest)
#   GA4GH_INFRA_INSTALL_DIR  Binary install directory (default: ~/.local/bin)
#   GA4GH_INFRA_CONFIG_DIR   Config directory (default: ~/.config/ga4gh-infra)

set -euo pipefail

REPO="${GA4GH_INFRA_REPO:-SynapticFour/ga4gh-infra}"
VERSION="${GA4GH_INFRA_VERSION:-}"
INSTALL_DIR="${GA4GH_INFRA_INSTALL_DIR:-${HOME}/.local/bin}"
CONFIG_DIR="${GA4GH_INFRA_CONFIG_DIR:-${HOME}/.config/ga4gh-infra}"
RAW_BASE="https://raw.githubusercontent.com/${REPO}/main"

log() {
    printf '==> %s\n' "$*"
}

err() {
    printf 'install.sh: %s\n' "$*" >&2
    exit 1
}

need_cmd() {
    command -v "$1" >/dev/null 2>&1 || err "required command not found: $1"
}

detect_target() {
    local os arch
    os="$(uname -s)"
    arch="$(uname -m)"
    case "${os}" in
        Linux) os=unknown-linux-gnu ;;
        Darwin) os=apple-darwin ;;
        *) err "unsupported OS: ${os}" ;;
    esac
    case "${arch}" in
        x86_64 | amd64) arch=x86_64 ;;
        aarch64 | arm64) arch=aarch64 ;;
        armv7l | armv6l) arch=armv7 ;;
        armv7) arch=armv7 ;;
        *) err "unsupported architecture: ${arch} (supported: x86_64, aarch64/arm64, armv7l for Raspberry Pi)" ;;
    esac
    case "${arch}" in
        x86_64 | aarch64) printf '%s-%s\n' "${arch}" "${os}" ;;
        armv7) printf 'armv7-unknown-linux-gnueabihf\n' ;;
    esac
}

resolve_version() {
    if [ -n "${VERSION}" ]; then
        printf 'ga4gh-infra-v%s\n' "${VERSION}"
        return
    fi
    need_cmd curl
    curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" \
        | sed -n 's/.*"tag_name"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' \
        | head -n1
}

download_binary() {
    local tag="$1"
    local target="$2"
    local asset="ga4gh-infra-${target}"
    local url="https://github.com/${REPO}/releases/download/${tag}/${asset}.tar.gz"
    local tmp
    tmp="$(mktemp -d)"
    trap 'rm -rf "${tmp}"' EXIT

    log "Downloading ${asset} from ${tag}"
    curl -fsSL "${url}" -o "${tmp}/archive.tar.gz"
    tar -xzf "${tmp}/archive.tar.gz" -C "${tmp}"
    mkdir -p "${INSTALL_DIR}"
    install -m 755 "${tmp}/${asset}" "${INSTALL_DIR}/ga4gh-infra"
}

write_config() {
    local template_url="${RAW_BASE}/config/all-in-one.native.toml.example"
    local env_url="${RAW_BASE}/config/env.native.example"
    mkdir -p "${CONFIG_DIR}/secrets"

    if [ ! -f "${CONFIG_DIR}/all-in-one.toml" ]; then
        log "Writing ${CONFIG_DIR}/all-in-one.toml"
        curl -fsSL "${template_url}" \
            | sed "s|{{CONFIG_DIR}}|${CONFIG_DIR}|g" \
            > "${CONFIG_DIR}/all-in-one.toml"
    else
        log "Keeping existing ${CONFIG_DIR}/all-in-one.toml"
    fi

    if [ ! -f "${CONFIG_DIR}/env" ]; then
        log "Writing ${CONFIG_DIR}/env"
        curl -fsSL "${env_url}" > "${CONFIG_DIR}/env"
    fi
}

generate_keys() {
    log "Generating signing keys (when missing)"
    "${INSTALL_DIR}/ga4gh-infra" keygen --output-dir "${CONFIG_DIR}/secrets"
}

main() {
    need_cmd curl
    need_cmd tar
    need_cmd install
    need_cmd sed

    local tag target
    tag="$(resolve_version)"
    [ -n "${tag}" ] || err "could not resolve latest ga4gh-infra release tag"
    target="$(detect_target)"

    download_binary "${tag}" "${target}"
    write_config
    generate_keys

    cat <<EOF

ga4gh-infra installed to ${INSTALL_DIR}/ga4gh-infra
Configuration directory: ${CONFIG_DIR}

Next steps:
  1. Add ${INSTALL_DIR} to your PATH if needed.
  2. Edit ${CONFIG_DIR}/env (secrets and SERVICE_REGISTRY_DATABASE_URL).
  3. source ${CONFIG_DIR}/env
  4. ga4gh-infra all-in-one --config ${CONFIG_DIR}/all-in-one.toml

For a PostgreSQL-free demo stack, use Docker instead:
  just up-sqlite

Raspberry Pi: use 64-bit Raspberry Pi OS (aarch64) or 32-bit armv7 — install.sh selects the matching release asset automatically.

EOF
}

main "$@"
