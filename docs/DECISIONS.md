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

### 2026-10-03 - ADR-004: Optional flat claim from signature-checked visas

- **Status:** Accepted
- **Context:** The broker copies visa JWTs from a visa source into `ga4gh_passport_v1` without checking their signatures, and it does not read the visa `jku` header. `PassportClaims.aud` is one optional string. Resource servers such as Solum read a flat `groups` claim. Copying the upstream ID token's `groups` into that claim mixes the hospital IdP's vocabulary with the broker token. An array `aud` would not decode in Ferrum, which stores audience as `Option<String>`.
- **Decision:** `[token_claims]` stays off by default. Off means the passport bytes stay as they are today: `groups` comes from the verified upstream ID token, `aud` is omitted, and visa strings are embedded without a signature check. On, the flat claim (default name `groups`, default visa type `AffiliationAndRole`) is filled only from visa JWTs this broker fetched and checked as RS256 against `jwks_file` or `jwks_url`. `jku` is ignored. The broker does not fetch a JWKS URL carried by the visa. Upstream ID token groups are not copied into that claim while the feature is on. `sub`, `email`, and `name` stay. A visa that fails the check, including a `sub` that is not the passport subject, contributes nothing. A missing verifier leaves the flat claim empty. `audiences` is a list. Empty omits `aud`. Exactly one value is emitted as a string. More than one value refuses startup even when the feature is off. The audience is written only when `token_claims.enabled` is true. One audience is shared by every resource server that accepts it, so a token valid for one of them is valid for the others. `verify_embedded_visas` is a separate option, default off, and example configs leave it off. The same verifier can drop visas before they are embedded. When that option is off, the original strings are embedded. When it is on and there is no verifier, nothing is embedded. When the flat claim is on and embedding verification is off, the claim still uses checked visas and the embedded strings stay unchecked. Startup loads the verifier when either option is on, and a JWKS URL that is down fails startup. When both are off, that JWKS is not fetched.
- **Consequences:** Operators who want Solum to read broker `groups` turn the feature on and accept that every resource server configured with that one audience will accept the same token. Downstream services that trust an embedded visa still have to verify it themselves until `verify_embedded_visas` is turned on. Array `aud` waits on a later type change.
- **Alternatives considered:** Copy the upstream ID token groups whenever visa verification fails (rejected: fail closed); follow `jku` (rejected: the visa would choose its own key); emit an audience array now (rejected: current decoders store one string); turn embedding verification on by default (rejected: that changes passport bytes for every deployment).

---
