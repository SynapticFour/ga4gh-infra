# CONTINUITY — ga4gh-infra (GA4GH Infra)

Survivability notes. **True on `origin/main` today.** Not legal advice.

Checked on 2026-10-14. Release tags: `git tag` (do not copy a tag into this file). Stack image labels and crate versions: `VERSIONS.lock` and `docs/versioning.md`. The broker is an OIDC relying party, not an institute identity provider.

## What this project is

Self-hostable Rust **identity plane**: OIDC brokering, Passport/Visa issuance, access decisions (ADS), DUO matching, Service Registry. The broker is an OIDC Relying Party — it does **not** replace an institute IdP. License: **Apache-2.0**. Maturity: Active (beta) — **Compose is the supported deploy**; Helm under `deploy/k8s/chart` is a sketch.

## Build from a clean machine (from a given tag)

### Prerequisites

- Git, `make`, Docker Engine + Compose
- rustup toolchain **1.91.1** (`rust-toolchain.toml`) for native `make prove`
- Debian/Ubuntu native builds: `pkg-config`, `libssl-dev`
- Network: crates.io; container registries for base images used by Compose
- Optional: `cargo-audit` for `make verify-release`
- Low-memory hosts: `CARGO_BUILD_JOBS=1`

### Source

```bash
git clone https://github.com/SynapticFour/ga4gh-infra.git
cd ga4gh-infra
git tag -l 'ga4gh-infra-v*'
git checkout <tag>   # a tag from that list; do not assume a version number written here
```

### Proof (no Docker)

```bash
make prove    # cargo test --workspace
```

### Supported local stack

```bash
make up          # alias for make up-local (hyphen required)
# or lighter:
make up-sqlite
make down        # keep volumes
make destroy     # remove volumes
```

`make up-local` runs `prepare-secrets`, `prepare-vendor`, `prepare-admin-ui-static`, then Compose `--build --wait`. See `docs/GETTING-STARTED.md` and `docker/README.md`.

## How to run tests

| Command | Meaning |
|---------|---------|
| `make prove` / `make test` | Workspace cargo tests (no Docker) |
| `make verify-release` | fmt, clippy, tests, cargo-audit gate before tagging |
| Dispatch `.github/workflows/ci.yml` | Docker e2e / coverage / ARM when needed (`docs/CI.md`) — not assumed on every push |

## How to cut a release

Per `RELEASING.md`:

1. `make verify-release` on the commit to tag.
2. Tag `ga4gh-infra-vX.Y.Z` and push.
3. Same week: bump Ferrum `VERSIONS.lock` (`GA4GH_INFRA_REF` / SHA) and Ferrum-GA4GH-Demo `GA4GH-INFRA-git`.
4. Docker images: `.github/workflows/docker-release.yml` publishes service images to **GHCR** (aai-broker, visa-registry, duo-service, service-registry, …).

## Continuity

ga4gh-infra is maintained by a single steward (Synaptic Four). To reduce dependence on any one person, the build is documented from a clean machine in this file, the release steps are written down, and the source can be inspected and built without access to Synaptic Four systems (clean-room check dated 2026-10; workspace crate versions may remain `0.1.0` while stack tags `ga4gh-infra-v*` advance). Operational details such as accounts, publication rights and escrow arrangements are being documented in a non-public continuity plan.

## Minimal operator runbook

| Action | Command |
|--------|---------|
| Install / prove | Clone tag → rustup → `make prove` |
| Start (supported) | `make up` or `make up-sqlite` |
| Stop | `make down` |
| Wipe | `make destroy` |
| Back up | Back up Compose volumes (Postgres/SQLite paths per compose file) + `docker/secrets` material you retain |
| Restore | Restore volumes/secrets → `make up` |
| Upgrade | Checkout newer `ga4gh-infra-v*` → rebuild Compose → re-pin Ferrum `VERSIONS.lock` if co-deployed |

URLs after start: `make print-urls` / `make print-urls-sqlite`.
