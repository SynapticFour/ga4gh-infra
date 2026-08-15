# Changelog (workspace)

Identity-plane hardening for institute production. Application crates remain at `0.1.0` (library semver); the **stack** release tag is `ga4gh-infra-v0.2.3` and Compose/GHCR image tags are **`0.2.3`**. **Do not install `ga4gh-infra-v0.1.0`.**

## [Unreleased]

- **CI:** `cargo-audit` uses rustc stable so the auditor can compile; product crates stay on 1.91.1. Dev PEMs are mode 644 so visa-registry can read Compose bind-mounts (uid 1000).

## [ga4gh-infra-v0.2.3] - 2026-08-15

- **Stack Docker Release publishes every Compose service.** Tag `ga4gh-infra-v0.2.3` pushes broker, visa-registry, duo, service-registry, ADS, agreement-registry, admin-ui, sample-resource, mock-idp, register-tools, and all-in-one as `:0.2.3`. `v0.2.2` on origin only pushed all-in-one.
- **Dev PEMs are no longer in git.** `make prepare-secrets` generates PKCS#8 keys under `docker/secrets/` (gitignored). Keys that were previously committed are public; do not reuse them.

## [ga4gh-infra-v0.2.2] - 2026-08-15

- Docker Release vendors `docker/vendor` before `build-push` (`COPY docker/vendor` requires it; the tree is gitignored).

## [ga4gh-infra-v0.2.1] - 2026-08-15

Git tag on origin (changelog freeze of identity-plane work). Superseded by `ga4gh-infra-v0.2.2` for Docker Release.

### Security

- Allowlist `return_url`; reject open HTTPS redirects outside development.
- DAC operators are an explicit group allowlist; ADS enforces `dac_group`.
- Mock IdP issues no groups unless `MOCK_IDP_GROUPS` is set.
- Embedded Africa mock IdP requires `allow_insecure_demo` and development.
- Installers verify SHA-256 and refuse `v0.1.0` / unsigned assets.
- Visa-registry `GET /revoked-jtis`; broker `GET /revoked-passports` + `POST /revoke-passports`.
- HMAC-SHA256 API-key hashes when `GA4GH_API_KEY_PEPPER` is set (legacy SHA-256 still verified).
- Multi-key JWKS via `signing.previous_key_pems`.
- In-process login/callback rate limit; security headers on all HTTP services.
- Structured audit events (`audit=true`) on Passport issue, visa create/revoke, DAC actions.
- Documented bootstrap secrets rejected outside development.
- Passport JWTs verified; visa subjects bound; visa API keys required (Rust audit close-out).
- Identity-plane threat model addendum for Ferrum co-deploy.

### Operations

- Dependabot disabled; `cargo-audit` clear; compatible crate bumps.
- Docker e2e, coverage, and ARM moved off every PR (schedule/dispatch).
- Dependency-review workflow; CODEOWNERS; local hooks mirroring GitHub CI.
- Production Compose includes ADS + admin-ui and omits mock-idp.
- Starter Helm chart under `deploy/k8s/chart`.
- IdP wiring guide: Keycloak groups mapper, Entra, ELIXIR, Shibboleth OIDC.
- Governance, key-rotation, GA4GH self-assessment.

## [ga4gh-infra-v0.1.0] - 2026-08-01

Initial binary release. **Do not use in production** (unsigned, predates auth fixes).
