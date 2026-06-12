# Contributing

Thank you for contributing to `ga4gh-infra`. This guide covers common extension points, testing, and review expectations.

## Development setup

```bash
git clone https://github.com/SynapticFour/ga4gh-infra.git
cd ga4gh-infra
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

Optional: install [just](https://github.com/casey/just) for Docker workflows (`just up`, `just test`, `just test-integration`, `just e2e`).

## Code style

- Rust 2021 edition; `cargo fmt` and `cargo clippy -D warnings` enforced in CI.
- SPDX license headers on source files.
- Public library APIs (`ga4gh-types`, `ga4gh-clearinghouse`) require doc comments (`#![deny(missing_docs)]`).
- Axum routes use **`:param`** syntax, not `{param}`.

## Adding a visa source type

1. Extend types in `ga4gh-types` if the GA4GH spec requires new visa types or claim shapes (semver-major if published).
2. Update `visa-registry` handlers/store if persistence or API changes are needed.
3. Update `ga4gh-clearinghouse` policy checks if resource services must enforce new visa semantics.
4. Add broker integration tests if collection or embedding behaviour changes.
5. Document config in [configuration.md](configuration.md).

## Adding DUO terms

1. DUO OWL is fetched/processed in `duo-service/build.rs` at compile time.
2. Rebuild `duo-service` after ontology updates; add unit tests in `matcher` and `terms` modules.
3. Document that runtime OWL updates are not supported ([limitations.md](limitations.md)).

## Testing strategy

| Layer | Command | Docker required? |
|-------|---------|------------------|
| Unit tests | `cargo test --workspace` or `just test` | No |
| Clearinghouse integration | `cargo test -p ga4gh-clearinghouse` | No (wiremock JWKS) |
| Testcontainers integration | `just test-integration` / `cargo test -p ga4gh-integration -- --ignored` | Yes |
| Docker e2e | `just e2e` / `./scripts/e2e.sh` | Yes |
| Ignored stack test | `cargo test -p ga4gh-e2e -- --ignored` | Yes (stack running) |

**Coverage expectations:** CI uploads workspace coverage to Codecov. Target **>80%** for `ga4gh-types` and `ga4gh-clearinghouse`; service crates rely more on integration and e2e tests and have a lower unit-test bar.

Shared fixtures live under [`tests/fixtures/`](../tests/fixtures/) at the workspace root. Integration tests are in [`tests/integration/`](../tests/integration/) (`ga4gh-integration` crate). Mark Docker-dependent tests with `#[ignore]` and run them via `just test-integration`.

When adding tests:

- **Types:** round-trips plus edge cases (optional field omission, malformed timestamps, custom visa types).
- **Clearinghouse:** passport/visa validation, JWKS cache behaviour (wiremock), policy combinators.
- **Broker:** session cookie signing, PKCE verifier round-trip, CSRF state mismatch.
- **Visa registry:** SQLite in-memory tests in-crate; Postgres via testcontainers in `ga4gh-integration`.
- **DUO / service-registry:** matcher edge cases, service-info GA4GH shape compliance.

## Pull requests

1. One logical change per PR when possible.
2. Include tests for behaviour changes.
3. Update relevant docs (`configuration.md`, `architecture.md`, etc.).
4. CI must pass: fmt, clippy, workspace unit tests, library all-features tests, integration job (testcontainers), coverage upload, Docker e2e on PRs.

## Releases

Published libraries and service binaries version independently — see [versioning.md](versioning.md). Use `cargo release -p <crate>` with the workspace `release.toml` config.

## Questions

Open a GitHub issue for design questions before large refactors. See [design-decisions.md](design-decisions.md) for rationale behind major choices.
