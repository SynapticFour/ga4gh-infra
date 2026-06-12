# Versioning and releases

The `ga4gh-infra` workspace uses **independent versions per crate**. There is no single unified release number for the whole repository. Components can be on different versions at the same time (for example `aai-broker` `0.3.0` with `ga4gh-types` `0.2.1`).

Each crate declares its own `version` in `crates/<name>/Cargo.toml`. Workspace inheritance is used for `edition`, `license`, and shared dependency versions — **not** for crate versions.

## Semver-strict libraries (crates.io)

These crates are intended for publication on [crates.io](https://crates.io/) and follow [Semantic Versioning](https://semver.org/) strictly:

| Crate | Current role |
|-------|----------------|
| [`ga4gh-types`](../crates/ga4gh-types) | Shared Passport, Visa, DUO, and Service Info types |
| [`ga4gh-clearinghouse`](../crates/ga4gh-clearinghouse) | Passport/Visa validation for resource services |

**Rules:**

- Breaking public API changes → **major** bump (`1.0.0` → `2.0.0`).
- Backward-compatible features → **minor** bump.
- Bug fixes only → **patch** bump.

Each published crate maintains a [Keep a Changelog](https://keepachangelog.com/) file:

- [`crates/ga4gh-types/CHANGELOG.md`](../crates/ga4gh-types/CHANGELOG.md)
- [`crates/ga4gh-clearinghouse/CHANGELOG.md`](../crates/ga4gh-clearinghouse/CHANGELOG.md)

When releasing `ga4gh-clearinghouse`, bump the `ga4gh-types` dependency constraint if the types crate also released a new compatible version (`cargo release` can do this with `dependent-version = "upgrade"`).

## Application / service crates (looser versioning)

Service binaries and internal tools use the same semver *syntax* but are **not** published to crates.io. Version bumps track deployable artifacts (Docker images, GitHub release binaries) rather than a stable library API:

| Crate | Deployed artifact |
|-------|-------------------|
| `aai-broker` | Docker image `…/aai-broker:<version>` |
| `visa-registry` | Docker image `…/visa-registry:<version>` |
| `duo-service` | Docker image `…/duo-service:<version>` |
| `service-registry` | Docker image `…/service-registry:<version>` |
| `ga4gh-infra-cli` (`ga4gh-infra` binary) | Docker all-in-one image + GitHub release binaries |
| `sample-resource` | Reference deployment only |
| `mock-idp` | Dev/CI only |

Breaking changes to HTTP APIs or config file shapes should still be called out in release notes, but these crates do not guarantee the same API stability bar as the published libraries.

Crates marked `publish = false` in `Cargo.toml` are excluded from `cargo publish`.

## Git tags

Tags identify **one crate version**, not the whole monorepo:

```text
ga4gh-types-v0.1.0
ga4gh-clearinghouse-v0.2.0
aai-broker-v0.3.0
visa-registry-v0.1.5
ga4gh-infra-v0.4.0          # combined CLI / all-in-one binary
```

Tag names are configured via `[package.metadata.release]` in each crate (see [`release.toml`](../release.toml) for workspace defaults).

## Docker image tags

Docker image tags mirror **the crate version of that component**, not a workspace-wide release:

```text
ghcr.io/<org>/aai-broker:0.3.0
ghcr.io/<org>/visa-registry:0.1.5
ghcr.io/<org>/ga4gh-infra:0.4.0
```

A compose stack can mix versions via environment pins in `docker/.env.example` (copy to `docker/.env` locally): `AAI_BROKER_VERSION=0.3.0`, etc. Upgrading one service does not require bumping every other service.

GitHub release tag `ga4gh-infra-v*` also publishes prebuilt binaries via `.github/workflows/release-binaries.yml`:

| Target | Typical hardware |
|--------|------------------|
| `x86_64-unknown-linux-gnu` | Linux PCs, servers |
| `aarch64-unknown-linux-gnu` | Raspberry Pi 4/5 (64-bit OS), ARM64 SBCs |
| `armv7-unknown-linux-gnueabihf` | Raspberry Pi 2/3/4 (32-bit OS) |
| `x86_64-apple-darwin` / `aarch64-apple-darwin` | macOS |
| `x86_64-pc-windows-msvc` | Windows |

## Releasing with cargo-release

Install [cargo-release](https://github.com/crate-ci/cargo-release):

```bash
cargo install cargo-release
```

**Dry run** (shows planned version bump, tag, and commits):

```bash
cargo release -p ga4gh-types
cargo release -p aai-broker
```

**Execute** a release:

```bash
cargo release -p ga4gh-types --execute
cargo release -p ga4gh-clearinghouse --execute
```

Published libraries also publish to crates.io when `publish = true` in their release metadata:

```bash
cargo release -p ga4gh-types --execute
# bumps version, updates CHANGELOG, commits, tags ga4gh-types-vX.Y.Z, pushes, cargo publish
```

Service crates set `publish = false`; their releases create git tags only (Docker/binary workflows in CI consume those tags).

Test-only crates (`ga4gh-e2e`, `mock-idp`) set `release = false` and are skipped by release automation.

## Dependency versions inside the workspace

Path dependencies in the root `Cargo.toml` include a semver version for packaging:

```toml
ga4gh-types = { path = "crates/ga4gh-types", version = "0.1.0" }
```

When a library crate releases, downstream workspace crates should update their dependency version constraints to match (automated by `cargo release` for dependents when configured).
