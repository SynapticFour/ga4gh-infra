# GA4GH Infra — Project Status, Gaps, and Roadmap

This document records what has been built, what is intentionally out of scope, and what remains before `ga4gh-infra` is suitable for production institute deployment. It is the canonical reference for follow-on work (including large planning prompts).

Related docs:

- [architecture.md](architecture.md) — component layout, auth flow, docker/e2e instructions
- [README.md](../README.md) — quick start and crate overview

---

## Completed work (phases 1–9 + distribution A–G)

All items from the original implementation plan are done, plus publication/CI polish, a reference resource service, and distribution/deployment/testing (Sections A–G).

| Phase | Deliverable | Status |
|-------|-------------|--------|
| 1 | `ga4gh-types` — shared structs, serde, round-trip tests | Done |
| 2 | `ga4gh-clearinghouse` — JWKS cache, policy engine, optional `axum` extractor | Done |
| 3 | `aai-broker` — OIDC **Relying Party** (not Authorization Server), Passport minting | Done |
| 4 | `visa-registry` — PostgreSQL, DAC API, signed visa JWTs | Done |
| 5 | `duo-service` — OWL → static catalog, `/terms`, `/match` | Done |
| 6 | `service-registry` — GA4GH read APIs, internal write API | Done |
| 7 | Docker Compose stack, `ga4gh-e2e`, `docs/architecture.md` | Done |
| 8 | Root README, `.gitignore`, GitHub Actions CI, crates.io metadata for libraries | Done |
| 9 | `sample-resource` — clearinghouse `ExtractedPassport` reference API, e2e extension | Done |
| A | Combined `ga4gh-infra` CLI + `all-in-one` | Done |
| B | SQLite backend for `visa-registry` | Done |
| C | Independent crate versioning, CHANGELOGs, `docs/versioning.md` | Done |
| D | Per-service Dockerfiles, compose version pins, release workflow | Done |
| E | `justfile`, install scripts, binary release workflow, `keygen` | Done |
| F | Full `docs/` structure and getting-started guides | Done |
| G | Comprehensive test coverage, `tests/integration`, Codecov CI | Done |
| 10 | Production deployment guide (TLS, Keycloak, Postgres ops) | Done |

### Verification commands

```bash
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
just test-integration   # testcontainers (ignored tests)
./scripts/e2e.sh
```

CI (`.github/workflows/ci.yml`) runs fmt, clippy, workspace unit tests, library all-features tests, testcontainers integration job, coverage upload, and Docker e2e on push/PR.

---

## Architectural decisions (intentional)

These are **not** gaps — they reflect deliberate design choices documented during implementation.

### Broker is an OIDC Relying Party, not an Authorization Server

- The broker does **not** expose `/authorize`, `/token`, login forms, MFA, or consent screens.
- Researchers authenticate at the **institute IdP**; the broker completes the OIDC code + PKCE flow and mints GA4GH Passports.
- Downstream-facing endpoints: `/.well-known/openid-configuration`, `/jwks.json`, `/userinfo`, `/service-info`, plus RP flow `/login` and `/callback`.

### No `/introspect` on the broker

- Token introspection is an Authorization Server concern.
- Resource services validate Passports via `ga4gh-clearinghouse` (signature, issuer, expiry, embedded visas).

### Visa registry is a separate issuer

- Visas are stored unsigned; the registry signs JWTs when serving them.
- The broker embeds signed visa JWT strings in the Passport; clearinghouses validate each visa against trusted issuer JWKS.

### `mock-idp` is dev/CI only

- Minimal OIDC provider for Docker and e2e tests.
- Not intended for production authentication.

---

## Known implementation notes

### Axum path parameter syntax (`:param`, not `{param}`)

All services register parameterized routes using **colon syntax**:

```text
/datasets/:dataset_id
/terms/:code
/services/:serviceId
/login/:idp_name
/visas/:id
```

Using `{param}` (as in some axum 0.7 docs) **silently fails to match** in this workspace’s axum/matchit setup — requests return **404 with an empty body**. This was discovered during Phase 9 and fixed workspace-wide.

When adding new routes, always use `:param` and add a router integration test if the route is security-critical.

### Docker internal vs external URLs

Several services use different URLs depending on caller context:

| Purpose | URL pattern |
|---------|-------------|
| JWT `iss` claim, clearinghouse trust | `http://localhost:8080` (host/e2e) |
| Container-to-container HTTP | `http://aai-broker:8080`, `http://visa-registry:8081`, etc. |
| Browser/e2e authorize step | Rewrite `mock-idp:9000` → `localhost:9000` on the host |

See `docker/config/*.toml` and `docs/architecture.md` for the current dev layout.

### JSON login sets session cookie

`GET /login` with `Accept: application/json` must return the RP session cookie alongside `authorization_url` (fixed in Phase 7). E2e tests pass the cookie manually to `/callback` (no `reqwest` cookie store — avoids `cookie`/`time` crate conflict).

---

## Gaps — original goals not yet done

| Gap | Priority | Description |
|-----|----------|-------------|
| **SAML2 upstream IdP** | High | Original project goal: connect institutes with SAML2 IdPs. Only OIDC upstream (via `openidconnect`) is implemented. Requires SAML SP library, metadata, assertion mapping to broker identity, and likely separate config section. |
| **crates.io publish** | Medium | `ga4gh-types` and `ga4gh-clearinghouse` have README, keywords, and `#![deny(missing_docs)]`. Not yet published; version is `0.1.0`, repository URL is a placeholder. |
| **Real repository metadata** | Done | `SynapticFour/ga4gh-infra` on GitHub; GHCR prefix `ghcr.io/SynapticFour` |

---

## Gaps — production and operations

| Gap | Priority | Description |
|-----|----------|-------------|
| **TLS termination** | High | Docker stack uses plain HTTP. Production needs reverse proxy (nginx, Caddy, Traefik) or ingress TLS, plus docs for `external_url` vs internal service URLs. |
| **Secrets management** | High | Dev keys live in `docker/secrets/` with documented “do not use in production.” Need guidance for PEM storage (KMS, mounted secrets, rotation). |
| **Structured audit logging** | Medium | Original broker spec: “Audit log every token issuance as structured JSON via tracing.” Today: general `tracing` on handlers; no dedicated audit event schema (subject, visa count, upstream IdP, client IP, etc.). |
| **Automatic service registration** | Medium | `docker/scripts/register-service.sh` and e2e registration exist. Binaries do not self-register with `service-registry` on startup. |
| **Kubernetes / Helm** | Medium | Single Compose stack only. No manifests, health probe conventions beyond `/service-info`, or horizontal scaling guidance. |
| **Operational hardening** | Medium | Not implemented: rate limiting, CORS policy, security headers, request size limits, graceful shutdown docs, backup/restore for PostgreSQL. |
| **Key rotation runbooks** | Medium | Broker and visa-registry sign with RS256 PEM files. No documented zero-downtime key rotation (multi-key JWKS, overlap period). |
| **CI PostgreSQL integration tests** | Done | `tests/integration` (`ga4gh-integration`) runs Postgres CRUD via testcontainers; `just test-integration` |

---

## Gaps — GA4GH and product completeness

| Gap | Priority | Description |
|-----|----------|-------------|
| **Real IdP integration guide** | High | No step-by-step for Keycloak, Microsoft Entra ID, Shibboleth OIDC, ELIXIR AAI, etc. Only `config/broker.example.toml` and docker mock-idp. |
| **Passport refresh / renewal** | Medium | No refresh token or silent re-auth flow. Expired Passports require full upstream login again. |
| **Visa revocation propagation** | Medium | DAC can `DELETE /visas/:id`, but already-issued Passports remain valid until expiry. No short Passport TTL + re-fetch pattern documented; no denylist. |
| **Broader visa/policy coverage in demos** | Low | E2e and `sample-resource` demonstrate `ControlledAccessGrants` and DUO matching. Not demonstrated: `HasAffiliation`, `AcceptedTermsAndPolicies`, `ResearcherStatus`, `LinkedIdentities`, `HasDuoPermission` in a live flow. |
| **GA4GH compliance audit** | Low | Types and flows align with specs, but no formal checklist pass against AAI OIDC Profile v1.2, Passport v1.2, Service Info, Service Registry, and DUO integration expectations. |
| **Multi-upstream IdP selection UX** | Low | Config supports `[[upstream_idps]]` and `/login/:idp_name`, but no UI or documented institute onboarding for multiple IdPs. |

---

## Gaps — documentation and polish

| Gap | Priority | Description |
|-----|----------|-------------|
| **Production deployment guide** | Done | [production-deployment.md](production-deployment.md), reverse-proxy examples, Keycloak template |
| **Config drift** | Low | Each service has `config/*.example.toml` plus `docker/config/*.toml`. Consider documenting which is canonical or generating docker configs from examples. |
| **CHANGELOG / releases** | Done | Per-crate CHANGELOGs, `cargo release`, Docker/binary workflows — see [versioning.md](versioning.md). |
| **Contributor guide** | Done | [contributing.md](contributing.md) |

---

## Suggested next phases

Ordered by impact for “real institute deployment”:

### Phase 11 — SAML2 upstream (optional institute path)

- SAML SP crate integration in `aai-broker`.
- Map SAML attributes → `ResearcherIdentity` (same path as OIDC callback).
- Config shape: `[[upstream_idps]]` with `protocol = "saml"` vs `"oidc"`.
- Docker test IdP or fixture-based integration tests.

### Phase 12 — crates.io publish

- Set real `repository`, `documentation`, and `homepage` in library `Cargo.toml` files.
- Publish `ga4gh-types` then `ga4gh-clearinghouse`.
- Pin versions in workspace for binaries.

### Phase 13 — Audit and hardening

- Structured JSON audit events on Passport issuance (and optionally visa grant/revoke).
- Rate limits on public broker endpoints.
- Optional Passport TTL tuning and refresh documentation.

### Phase 14 — Operational automation

- Self-registration hook in each binary (optional, config-gated).
- Helm chart or Compose production profile with `read_only` registry and no mock-idp.

---

## Crate inventory (current)

| Crate | Published? | Deployed in Docker? |
|-------|------------|---------------------|
| `ga4gh-types` | Intended (crates.io) | No (library) |
| `ga4gh-clearinghouse` | Intended (crates.io) | No (library) |
| `aai-broker` | No | Yes (:8080) |
| `visa-registry` | No | Yes (:8081) |
| `duo-service` | No | Yes (:8082) |
| `service-registry` | No | Yes (:8083) |
| `sample-resource` | No | Yes (:8084) |
| `mock-idp` | No | Yes (:9000, dev only) |
| `ga4gh-e2e` | No | No (host test against stack) |

---

## Dev secrets reference (development only)

| Variable / secret | Default | Used by |
|-------------------|---------|---------|
| `BROKER_COOKIE_SECRET` | `dev-broker-cookie-secret` | aai-broker |
| `MOCK_IDP_CLIENT_SECRET` | `mock-client-secret` | aai-broker → mock-idp |
| `REGISTRY_BOOTSTRAP_API_KEY` | `dev-visa-api-key` | visa-registry DAC API |
| `SERVICE_REGISTRY_REGISTRATION_KEY` | `dev-service-registry-key` | service-registry writes |
| PEM files in `docker/secrets/` | test RSA keys | broker, registry, mock-idp signing |

Never use these outside local development.

---

## Standards referenced

- [GA4GH AAI OIDC Profile v1.2](https://ga4gh.github.io/data-security/aai-openid-connect-profile)
- [GA4GH Passport & Visa specification](https://github.com/ga4gh-duri/ga4gh-duri.github.io/blob/master/researcher_ids/ga4gh_passport_v1.md)
- [GA4GH Service Info](https://github.com/ga4gh-discovery/ga4gh-service-info)
- [GA4GH Service Registry](https://github.com/ga4gh-discovery/ga4gh-service-registry)
- [Data Use Ontology (DUO)](https://github.com/EBISPOT/DUO)

---

*Last updated: June 2026 — after Phase 10 (production deployment guide).*
