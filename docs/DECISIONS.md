# Engineering Decisions (ADR-lite)

Track important architectural and operational decisions for ga4gh-infra.

## Template

### YYYY-MM-DD - Decision title

- **Context:** Why this decision was needed.
- **Decision:** What was chosen.
- **Consequences:** Trade-offs, risks, and follow-up actions.

---

### 2026-06-12 - ADR-001: Africa-Mode for resource-constrained identity plane

- **Status:** Accepted
- **Context:** Ferrum's Africa/Laptop Mode covers the data plane (SQLite, local storage, offline DRS/Beacon). ga4gh-infra had ARM binaries but still required PostgreSQL for service-registry, blocking true zero-dependency edge auth stacks.
- **Decision:** Add `[africa]` profile to all-in-one config: SQLite for visa-registry, service-registry, and ADS; optional embedded mock-idp; co-deploy port block 8180–8190; `ga4gh-infra all-in-one --africa` and `GA4GH_OFFLINE=1` shortcut.
- **Consequences:** Single-process auth stack runs on Pi-class hardware without Postgres. Service-registry `list_types` deduplicates in application code for SQLite/Postgres parity. Not a substitute for production multi-user IdP integration.
- **Alternatives considered:** Require Postgres even on Pi (rejected: operational burden); fold auth into Ferrum (rejected: see ADR-002).

---

### 2026-06-12 - ADR-002: Identity/access plane boundary with Ferrum co-deploy

- **Status:** Accepted
- **Context:** Ferrum shipped built-in Passport broker and visa tables, overlapping ga4gh-infra's broker, visa-registry, DUO, and ADS. Operators needed both stacks on one host without port or auth conflicts.
- **Decision:** ga4gh-infra owns **identity and access** (AAI broker, visa-registry, DUO, ADS, service-registry, clearinghouse library). Ferrum owns **data and compute** (DRS, WES, TES, TRS, Beacon, htsget, Crypt4GH, Africa genomics features). Co-deploy uses port block 8180–8190; Ferrum stays on 8080. Ferrum registers data services in ga4gh-infra service-registry; Ferrum validates Passports via clearinghouse when `auth.mode = external`.
- **Consequences:** Both stacks remain independently deployable. Co-deploy requires documented port matrix and monorepo Docker build for Ferrum (`Ferrum/deploy/Dockerfile.gateway-monorepo`). agreement-registry HTTP service remains future work; opaque DTA refs to Ferrum are extension points only.
- **Alternatives considered:** Merge broker into Ferrum gateway (rejected: blurs layers); shared Postgres for both (rejected: unnecessary coupling on edge).

---

### 2026-10-03 - ADR-003: Extra CA bundle for upstream OIDC

- **Status:** Accepted
- **Context:** The broker's reqwest client is built with `rustls-tls` and bundled webpki roots. It does not load the operating-system trust store and it does not read `SSL_CERT_FILE`. An on-prem IdP whose discovery and JWKS URLs are signed by a private CA fails TLS verification. Disabling verification, or switching the default to native roots, would change trust for every deployment.
- **Decision:** Optional `tls.extra_ca_bundle` is a PEM file of extra CA certificates added on top of the bundled webpki roots. Unset means webpki roots only. A set path that is missing, unreadable, or has no certificate fails startup. `SSL_CERT_FILE` stays unread. Visa-source and ADS HTTP clients are separate builders and are not changed.
- **Consequences:** An institute can trust one private CA for upstream OIDC discovery, token exchange, userinfo, and the JWKS fetch those calls make. Public IdPs keep the previous trust set. Operators must not point this at a bundle that replaces or disables the webpki roots.
- **Alternatives considered:** Honor `SSL_CERT_FILE` whenever it is set (rejected: ambient env would change trust); `danger_accept_invalid_certs` (rejected); rustls-native-certs as the default (rejected: changes the roots this binary already ships).

---
