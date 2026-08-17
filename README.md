# ga4gh-infra (GA4GH Infra)

[![CI](https://github.com/SynapticFour/ga4gh-infra/actions/workflows/ci.yml/badge.svg)](https://github.com/SynapticFour/ga4gh-infra/actions/workflows/ci.yml)
[![codecov](https://codecov.io/gh/SynapticFour/ga4gh-infra/graph/badge.svg)](https://codecov.io/gh/SynapticFour/ga4gh-infra)
[![License](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](LICENSE)

Self-hostable Rust identity plane: OIDC brokering, Passport/Visa issuance, access decisions (ADS), DUO matching, and a Service Registry. The broker is an OIDC Relying Party — it does not replace an institute IdP.

**Maturity: Active (beta) — Compose path.** Helm under `deploy/k8s/chart` is a **sketch**, not the supported production path. No third-party security audit. Not GA4GH certification.

> This README describes technical capabilities, not legal advice. Passport and visa flows may involve identifiable researcher data. See [docs/limitations.md](docs/limitations.md).

## Ferrum / GA4GH suite

These ten public repositories are from the same organisation and can be composed. They are not a fifth product and not a bundle SKU. Each repository keeps its own version and license. Roles, maturity, and who consumes whom: [SUITE-OVERVIEW](https://github.com/SynapticFour/Ferrum/blob/main/docs/SUITE-OVERVIEW.md).

## Quick start

```bash
make prove    # workspace tests (no Docker)
make up       # Compose stack (supported deploy). Alias: make up-local
```

Stop: `make down` (keep volumes) or `make destroy` (remove volumes). Lighter SQLite stack: `make up-sqlite`.

## Documentation

- [Getting started](docs/GETTING-STARTED.md)
- [Architecture](docs/ARCHITECTURE.md)
- [For evaluators](docs/FOR-EVALUATORS.md)
- [Limitations](docs/limitations.md) · [Documentation index](docs/README.md)

Crate versions in this workspace are **0.1.0**. Stack tags are `ga4gh-infra-v*`.

## License

Apache License 2.0 — see [LICENSE](LICENSE) and [NOTICE](NOTICE).
