# Security policy

## Reporting vulnerabilities

Please report security issues **privately** — do not open a public GitHub issue.

- Email: [contact@synapticfour.com](mailto:contact@synapticfour.com)
- Or use GitHub **Security** → **Report a vulnerability** on this repository

Include steps to reproduce, affected components, and impact. We will acknowledge receipt and coordinate a fix and disclosure timeline.

## Supported versions

| Version | Supported |
|---------|-----------|
| Latest release on `main` | Yes |
| Older tags | Best effort |

## Scope

This repository provides GA4GH identity and access services (AAI broker, visa registry, ADS, service registry). Production deployments must generate signing keys (`ga4gh-infra keygen` / `make prepare-secrets`) onto a secret volume; `docker/secrets/*.pem` are local-only and gitignored. Keys that were previously committed are public. **History rewrite is not planned** (accepted residual risk, 2026-08-17) — treat historical PEMs as public and never reuse them.

See [docs/production-deployment.md](docs/production-deployment.md) and [docs/security.md](docs/security.md).
