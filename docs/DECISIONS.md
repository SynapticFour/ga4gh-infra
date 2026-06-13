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
