# ga4gh-infra

[![CI](https://github.com/SynapticFour/ga4gh-infra/actions/workflows/ci.yml/badge.svg)](https://github.com/SynapticFour/ga4gh-infra/actions/workflows/ci.yml)
[![codecov](https://codecov.io/gh/SynapticFour/ga4gh-infra/graph/badge.svg)](https://codecov.io/gh/SynapticFour/ga4gh-infra)
[![License](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](LICENSE)

Self-hostable Rust implementation of GA4GH infrastructure: OIDC brokering, Passport/Visa issuance, DUO matching, and a Service Registry.

> **Legal notice:** This repository documents technical capabilities and operating guidance. It is not legal advice and does not by itself provide regulatory certification or compliance guarantees. Compliance outcomes depend on operator configuration, contracts, and organisational controls. Passport and visa flows may involve identifiable researcher data — assess your legal basis before production use. See [docs/limitations.md](docs/limitations.md).

## Quick start

**Docker** (full stack):

```bash
just up
# or: docker compose -f docker/docker-compose.yml --env-file docker/.env.example up --build --wait
```

**Native binary** (all-in-one; ARM builds for Raspberry Pi 64-bit and 32-bit):

```bash
curl -sSL https://raw.githubusercontent.com/SynapticFour/ga4gh-infra/main/scripts/install.sh | sh
source ~/.config/ga4gh-infra/env
ga4gh-infra all-in-one --config ~/.config/ga4gh-infra/all-in-one.toml
```

See **[docs/getting-started.md](docs/getting-started.md)** for both paths, Pi/ARM notes, and what each command does.

## Documentation

| Topic | Link |
|-------|------|
| Getting started | [docs/getting-started.md](docs/getting-started.md) |
| Architecture | [docs/architecture.md](docs/architecture.md) |
| Configuration | [docs/configuration.md](docs/configuration.md) |
| Deployment | [docs/deployment-scenarios.md](docs/deployment-scenarios.md) |
| Production | [docs/production-deployment.md](docs/production-deployment.md) |
| Limitations | [docs/limitations.md](docs/limitations.md) |
| Contributing | [docs/contributing.md](docs/contributing.md) |
| Full index | [docs/README.md](docs/README.md) |

## Crates

| Crate | Role |
|-------|------|
| [`ga4gh-types`](crates/ga4gh-types) | Shared GA4GH types (crates.io) |
| [`ga4gh-clearinghouse`](crates/ga4gh-clearinghouse) | Passport/Visa validation library (crates.io) |
| [`aai-broker`](crates/aai-broker) | OIDC Relying Party; mints Passports |
| [`visa-registry`](crates/visa-registry) | Visa store + DAC API (Postgres or SQLite) |
| [`duo-service`](crates/duo-service) | DUO catalog and `/match` |
| [`service-registry`](crates/service-registry) | GA4GH Service Registry |
| [`ga4gh-infra-cli`](crates/ga4gh-infra-cli) | Combined `ga4gh-infra` binary |

The broker is an **OIDC Relying Party** — it does not replace your institute IdP. Resource services validate Passports with `ga4gh-clearinghouse`.

Related SynapticFour projects: [Ferrum](https://github.com/SynapticFour/Ferrum) (full GA4GH cloud stack), [Open-Source-GA4GH-Stack](https://github.com/SynapticFour/Open-Source-GA4GH-Stack) (curated upstream OSS kit).

## Development

```bash
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
just test-integration   # Docker + testcontainers (ignored tests)
just e2e    # Docker stack integration test
```

## Security

Report vulnerabilities privately — see [SECURITY.md](SECURITY.md). Questions: [contact@synapticfour.com](mailto:contact@synapticfour.com).

## License

Apache License 2.0 — see [LICENSE](LICENSE) and [NOTICE](NOTICE).
