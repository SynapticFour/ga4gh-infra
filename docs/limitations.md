# Limitations

An explicit list of what `ga4gh-infra` **does not** do today and what that means for production use.

## Authentication and identity

### No SAML2 upstream

Only **OIDC** upstream IdPs are supported (`openidconnect` crate). Institutes with SAML-only IdPs must use an OIDC-capable bridge (e.g. Keycloak SAML→OIDC, Shibboleth OIDC plugin, Entra federated OIDC).

### No multi-tenant WAYF / IdP discovery UI

Multiple upstream IdPs can be configured (`[[upstream_idps]]`, `/login/:idp_name`), but there is no built-in “where are you from?” UI or automatic IdP discovery. Operators must link users to the correct login entry point.

## Data and persistence

### DUO is a static snapshot

DUO terms are compiled into `duo-service` at **build time**. Live OWL updates from EBISPOT are not applied at runtime. Rebuild and redeploy to pick up ontology changes.

### SQLite is single-writer

SQLite mode (visa-registry) suits demo, desktop, and single-node edge use (including Raspberry Pi). It is **not** recommended for multi-writer production clusters. Use PostgreSQL for concurrent DAC/API load.

### Service-registry SQLite mode

`service-registry` supports **PostgreSQL** (recommended for multi-writer production) and **SQLite** (demo, desktop, single-node edge, and `docker-compose.sqlite.yml` / native all-in-one configs). SQLite is single-writer — not for concurrent multi-node clusters.

## Security and operations

### Rate limits and request size

Login and callback are rate-limited in-process (`login_rate_limit_per_minute`, default 20). Request bodies are capped at 1 MiB. Still put a reverse proxy or WAF in front for TLS, extra rate limits, and IP blocking.

### No formal security audit

This codebase has **not** undergone an independent third-party security audit. Do not assume GA4GH compliance or production readiness for sensitive human genomic data without your institute’s own risk assessment, penetration testing, and operational controls.

### Dev secrets in the repository

`docker/secrets/*.pem` are **generated on this machine** (`make prepare-secrets`) and gitignored. Documented default env values (`dev-*-api-key`) are for local development only. Never use either outside CI/dev machines. Private keys that appeared in git history are public.

## GA4GH behaviour

### Visa revocation vs issued Passports

Revoking a visa in the registry (`DELETE /visas/:id`) publishes the assertion id on `GET /revoked-jtis`. Clearinghouses drop matching embedded visas on extract. **Already-minted Passport JWTs remain valid until expiry unless revoked** via broker `POST /revoke-passports` (then `GET /revoked-passports`). Keep `passport_lifetime_seconds` short (900s in production examples) and persist the ledger on a shared volume when running more than one broker replica.

Revocation visibility at the broker depends on how often visa sources are queried during login (on each login today, not continuous polling).

### Limited demo visa/policy coverage

E2e and `sample-resource` focus on `ControlledAccessGrants` and DUO matching. Other visa types (`HasAffiliation`, `ResearcherStatus`, etc.) are supported in types/clearinghouse but not demonstrated end-to-end in the default stack.

### No Passport refresh / silent re-auth

Expired Passports require a full upstream IdP login. Refresh tokens and silent renewal are not implemented.

## Deployment

### TLS termination not included

Docker and native quick starts use plain HTTP. Production requires TLS at a reverse proxy or ingress; `external_url` in configs must match the public HTTPS URL used in JWT `iss` claims.

### Kubernetes / Helm is a broker sketch

**Supported deploy is Compose** (`make up` / `docker/docker-compose.yml`). A starter Helm chart lives in [`deploy/k8s/chart`](../deploy/k8s/chart) — broker-shaped, not a full identity-plane production chart. Institutes still own Ingress, secrets, Postgres, and TLS. Do not read the chart as Kubernetes-ready GA4GH Infra.

### Admin UI is English-only

The operations dashboard has no i18n or language switcher. All UI copy is English; timestamps are UTC with ISO tooltips.

See [roadmap.md](roadmap.md) for planned follow-on work.
