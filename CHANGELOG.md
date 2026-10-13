# Changelog (workspace)

Identity-plane hardening for institute production. Application crates remain at `0.1.0` (library semver); the **stack** release tag is `ga4gh-infra-v0.2.4` and Compose/GHCR image tags are **`0.2.4`**. **Do not install `ga4gh-infra-v0.1.0`.**

## [Unreleased]

## [ga4gh-infra-v0.2.7] - 2026-10-13

Image tags are `:0.2.7`. Crate `version` stays `0.1.0`. **Do not create the git tag until `make verify-release` is green on the release commit.** Do **not** move `ga4gh-infra-v0.2.6` (still peels to `f2ac411`, no Visa document store).

### Clearinghouse UserInfo + passport size

- **`ga4gh-clearinghouse` UserInfo Visas** — when `ga4gh_passport_v1` is empty after Passport validation, fetch Visas from OIDC discovery of the trusted Passport `iss` only (issuer match, scheme match), forward Bearer, cache by SHA-256(token) until `exp` (max 1024). Fail closed on errors/timeouts/401. ADR-006 (Ferrum ADR-034 rules). `extract_visas(passport, raw_jwt)` now requires the raw Bearer.
- **sample-resource** — passes Bearer into `extract_visas` (works with `embed_visas = false`).
- **Consumer matrix** — **admin-ui** supported (identity claims). **ADS** / **duo-service** N/A (not Passport Clearinghouses). **ga4gh-clearinghouse** / **sample-resource** **supported** with `embed_visas = false` in this tag.
- Includes ADR-005 broker work already on `main` (not in `v0.2.6`): `[passport] embed_visas`, in-memory Visa document store (TTL = `exp`; max 10 000 / 64 MiB), `/userinfo` 401 on restart miss. See [docs/PASSPORT-SIZE.md](docs/PASSPORT-SIZE.md).

**Runner minutes (public repo, rough):** push/PR → secret-scan + dependency-review ≈ **2–4**. Before tag: `workflow_dispatch` `ci.yml` ≈ **40–90**. Tag `ga4gh-infra-v0.2.7` → `docker-release.yml` ≈ **20–40** + `release-binaries.yml` ≈ **15–40**. Total if you run full product CI then tag: about **75–170** minutes.

**Owner manual checks before/after tag:** `make verify-release` on the tip; confirm `git rev-parse ga4gh-infra-v0.2.6^{}` still `f2ac411`; after images publish, smoke `embed_visas=false` login → sample-resource controlled-access with small Authorization; broker restart → UserInfo 401 until re-login; admin-ui still opens; ADS/duo unchanged.

**s4-showcase (after `0.2.7` images exist):** pin suite/broker images to `:0.2.7` digests; set `[passport] embed_visas = false` in `deploy/ga4gh/broker.toml.tpl`; step Caddy `:443` `max_header_size` from **64KB → 16KB** (and update `check.sh` / `render-config.sh` assertions); re-login after broker restart; Ferrum already has ADR-034 on `v0.3.5`.

### Passport size (on main; tag `ga4gh-infra-v0.2.6` does **not** include this)

Published tag `ga4gh-infra-v0.2.6` / image `:0.2.6` peels to `f2ac411` and **lacks** the Visa document store. The broker commits below are on `main` and ship in **`ga4gh-infra-v0.2.7`**.

- **`[passport] embed_visas`** — default **`true`**. When **`false`**, empty `ga4gh_passport_v1` + Visas from `GET /userinfo`. ADR-005.
- **In-memory Visa document store bounds** — as above.

## [aai-broker-v0.2.5] - 2026-10-11

Image tag `:0.2.5` (crate tag `aai-broker-v0.2.5`). Crate `version` stays `0.1.0`.

- **BFF authorization code flow:** `response_mode=query` with PKCE S256, client `state`/`nonce`, one-time `/oauth/token` exchange, and exact `redirect_uri` match. Optional `login_hint` forwarded to Keycloak.

## [ga4gh-infra-v0.2.4] - 2026-10-03

Image tags are `:0.2.4`. Crate `version` stays `0.1.0`. The git tag is not created in this commit.

- **Upstream TLS** — optional `tls.extra_ca_bundle` adds a PEM CA to the broker's bundled webpki roots for OIDC discovery, token exchange, userinfo, and JWKS. Unset is unchanged. `SSL_CERT_FILE` is not read. ADR-003.
- **Service registry liveness** — `GET /services` returns the server `updatedAt` and a `stale` flag. `stale_after_seconds` unset means nothing is stale. Set it to at least twice a registrant's heartbeat. Rows are not deleted. `?type=` matches `type.artifact`. `POST` stays an upsert. `DELETE` stays key-authenticated.
- **Passport token claims** — `[token_claims]` stays off. On, the flat claim (default `groups`, default visa type `AffiliationAndRole`) comes only from RS256-checked visas, and upstream ID token groups are not copied. One audience is a string `aud` shared by every resource server that accepts it. `verify_embedded_visas` can drop unchecked visas before embedding and stays off. ADR-004.
- **Docs** — Passport `audit=true` JSON is implemented. SQLite Compose does not use Postgres for service-registry. Keycloak `sub` mapping, the three-service sqlite set, and that audit rows are not a hash chain.
- **Field board** — Africa and ARM docs name the same board as Ferrum ADR-026: Raspberry Pi 5, 8 GB, 64-bit. The `armv7` binary stays a compile target, not that board.
- **cargo-audit:** `h2` 0.4.19 (RUSTSEC-2026-0258), `event-listener` 5.4.2 (RUSTSEC-2026-0221), `rustls` 0.23.45 (RUSTSEC-2026-0285). `rustls-pemfile` unmaintained (RUSTSEC-2025-0134) is transitive via bollard → testcontainers (dev/integration only); ignored like tokio-tar.
- **CI** — push to `main` and pull requests run secret scan and dependency review. Product CI is `workflow_dispatch`. `make verify-release` is the local gate before `ga4gh-infra-v*`.
- HelixTest pin **v0.1.3** (suite tag; SHA in `VERSIONS.lock`). Helix is the VERIFY brand around that CLI (not a third SKU). Helm chart labelled **SKETCH**.
- Historical `docker/secrets/*.pem` remain in git objects. **History rewrite is not planned** (accepted residual risk, 2026-08-17). `make prepare-secrets` per clone; never reuse old PEMs.
- **Release Binaries:** install the 1.91.1 target (not `@stable` vs `rust-toolchain.toml`), cross-compile Intel macOS on `macos-latest`, and allow `workflow_dispatch` to attach assets to an existing `ga4gh-infra-v*` tag.
- **`make prove`** — workspace tests without Docker (same command as CI unit tests). Live stack remains `make up`.
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
