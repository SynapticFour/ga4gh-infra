# Security model

This document is the operator-facing security contract for `ga4gh-infra`. Read it before connecting the stack to a real identity provider or to controlled-access genomic data.

Private vulnerability reports: see [SECURITY.md](../SECURITY.md).

## What this stack is responsible for

`ga4gh-infra` is an **identity and access plane**, not a data archive. It:

- Delegates authentication to an institute OIDC IdP (the broker is a Relying Party).
- Mints short-lived GA4GH Passport JWTs after visa collection.
- Stores unsigned visa assertions and signs visa JWTs at the visa registry.
- Lets a Data Access Committee approve or reject access requests (ADS + admin-ui).
- Lets resource services validate Passports with `ga4gh-clearinghouse`.

It does **not** replace your IdP, your TLS termination, or your dataset storage ACLs.

## Production requirements (non-negotiable)

1. **Do not run `mock-idp`.** It is a test fixture. Docker Compose sets `MOCK_IDP_GROUPS` only so end-to-end tests can exercise admin-ui. Production compose must omit the service.
2. **Do not use committed secrets.** `docker/secrets/*.pem`, `dev-broker-cookie-secret`, `dev-ads-api-key`, and similar values are blocked outside development unless `GA4GH_ALLOW_DEV_SECRETS` is set (that override is for CI, not production).
3. **Pin a signed release newer than `ga4gh-infra-v0.1.0`.** `v0.1.0` shipped without checksums and without later authentication fixes. `scripts/install.sh` and `scripts/install.ps1` refuse that tag and refuse any asset without a matching `.sha256` file.
4. **Allowlist `return_url`.** Set `server.allowed_return_url_origins` to the admin-ui (and any other post-login origin). An empty list is rejected outside development. Passports are returned in the URL fragment; an open redirect would leak them to a third party.
5. **Scope DAC operators.** Configure `dac_operator_groups` on admin-ui. An empty list means **only** the admin claim may approve or reject. ADS checks that scoped operators belong to the request's `dac_group`. The ADS API key remains break-glass; treat it as a secret, not as a user role.
6. **Keep Passport TTL short and use revocation lists.** Production examples use `passport_lifetime_seconds = 900`. Visa revocation (`GET /revoked-jtis`) drops revoked visas on extract. Stolen or withdrawn Passports: `POST /revoke-passports` then `GET /revoked-passports`. Persist `passport_ledger_path` when running more than one broker replica.
7. **Terminate TLS at a reverse proxy.** Enable `Secure` cookies via `https://` public URLs. Rate-limit `/login` and `/callback` at the proxy **and** keep the broker `login_rate_limit_per_minute` (default 20).
8. **Issue admin groups from the real IdP.** Default mock tokens carry **no** groups. Admin membership is `admin_claim_value` (default `ga4gh-infra-admins`).
9. **Ship a signed release.** Tag `ga4gh-infra-v0.2.0` (or later) so installers and GHCR pins are not `v0.1.0`. See [governance.md](governance.md).

## Authentication and authorization

| Control | Where |
|---------|--------|
| Researcher login | Institute IdP → broker `/login` + `/callback` (OIDC code + PKCE) |
| Post-login redirect | `allowed_return_url_origins` (scheme + host + port; userinfo rejected) |
| Admin-ui session | HMAC cookie; Passport verified against broker JWKS |
| DAC actions | Admin claim **or** intersection with `dac_operator_groups`; ADS re-checks `dac_group` |
| Visa API | `X-API-Key` on visa-registry |
| Passport validation | `iss` + RS256 signature + expiry. `aud` is **not** enforced: GA4GH Passports are presented to many resource services and are often minted without `aud`. |

## Visa revocation

Visa JWT `jti` equals the assertion UUID. `DELETE /visas/:id` sets `revoked_at`. Clearinghouses fetch `{jwks origin}/revoked-jtis` (inferred from `jwks_uri`). A 404 is treated as an empty list. Resource services must re-validate Passports (or re-extract visas) rather than cache grants for longer than the Passport TTL.

## Supply chain

- Dependabot updates Cargo and GitHub Actions monthly.
- CI runs `cargo-audit` and GitHub Dependency Review (blocking on pull requests).
- Docker e2e runs on pull requests, not only on a weekly schedule.
- Binary releases publish a sibling `.sha256` file; installers verify it before extracting.

## Africa / offline mode

`embedded_mock_idp` requires `allow_insecure_demo = true` **and** a development environment. `--africa` no longer starts a mock IdP by default. Field labs authenticating real researchers must use an institute IdP.

## Residual risk (honest)

- Bus factor is one upstream CODEOWNER. This repo stays pushable to `main` during development. Follow [governance.md](governance.md) on **your fork** before go-live: add your team to CODEOWNERS and enable required reviews there.
- There is no formal third-party security audit. Use [THREAT_MODEL.md](THREAT_MODEL.md) and [ga4gh-compliance.md](ga4gh-compliance.md) as the internal self-assessment.
- Passport-level denylist: broker `GET /revoked-passports` / `POST /revoke-passports`; persist `passport_ledger_path` on a shared volume for replicas.
- AdsStore is split by entity (`store/*.rs`); DAC *authorization* lives in `handlers/dac.rs`.
- Multi-key JWKS: set `signing.previous_key_pems` so one process publishes the old `kid` during overlap.

A research organization can operate this stack in good conscience when the production requirements above are met, a deputy can ship a signed release, secrets are institute-controlled, and a second operator reviews identity/DAC changes.
