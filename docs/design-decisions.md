# Design decisions

Architecture Decision Record (ADR) style log of key choices in `ga4gh-infra`.

## Apache 2.0 licensing

**Context:** GA4GH infrastructure is deployed at institutes, integrated into existing systems, and extended by third parties.

**Decision:** Release the workspace under the Apache License 2.0.

**Consequences:** Permissive use in commercial and academic settings; patent grant included; downstream modifications do not need to be open-sourced. Each crate carries SPDX headers and the root `NOTICE` file lists attribution.

---

## Broker as OIDC Relying Party, not Authorization Server

**Context:** GA4GH AAI expects researchers to authenticate at their home institute IdP. Some early designs conflate “broker” with hosting login pages, token endpoints, and consent UI.

**Decision:** `aai-broker` is an **OIDC Relying Party** only. It initiates authorization code + PKCE flows against configured upstream IdPs, exchanges codes for ID tokens, collects visas, and mints GA4GH Passport JWTs. It does **not** implement `/authorize`, `/token`, login forms, MFA, or `/introspect`.

**Consequences:**

- Institutes keep their existing IdP (Keycloak, Entra ID, ELIXIR AAI, etc.).
- The broker exposes RP endpoints (`/login`, `/callback`) and GA4GH-facing surfaces (`/userinfo`, `/jwks.json`, `/.well-known/openid-configuration`).
- Resource services validate Passports via `ga4gh-clearinghouse`, not broker introspection.
- SAML2 upstream is **not** implemented; institutes can front SAML with an OIDC-capable proxy (see [limitations.md](limitations.md)).

---

## SQLite and PostgreSQL dual support (visa-registry)

**Context:** Production deployments use PostgreSQL; desktop, demo, and edge devices (e.g. Raspberry Pi) benefit from embedded SQLite.

**Decision:** `visa-registry` supports `driver = "postgres" | "sqlite"` with portable migrations (TEXT ids/JSON, INTEGER timestamps). SQLite auto-creates files and runs migrations; PostgreSQL uses explicit or `auto_migrate` startup migration.

**Consequences:** Same API and schema on both backends; service-registry remains PostgreSQL-only for now. All-in-one native installs default to SQLite for visas.

---

## Workspace structure and published crates

**Context:** Binary services and reusable libraries serve different consumers.

**Decision:**

- **Published to crates.io (semver-strict):** `ga4gh-types`, `ga4gh-clearinghouse`
- **Deployable binaries (independent versions):** broker, visa-registry, duo-service, service-registry, `ga4gh-infra` CLI
- **Dev/CI only:** `mock-idp`, `ga4gh-e2e`, `sample-resource`

**Consequences:** Resource services depend on clearinghouse without pulling in HTTP service crates. Docker image tags and git tags follow per-crate versions ([versioning.md](versioning.md)).

---

## DUO terms compiled statically

**Context:** DUO is an OWL ontology; runtime OWL reasoning is heavy and complicates deployment.

**Decision:** `duo-service` embeds a **static catalog** generated at build time from DUO OWL (via `build.rs`). Matching uses precomputed hierarchy, not live reasoning.

**Consequences:** Fast, deterministic `/match` responses; ontology updates require rebuilding/redeploying `duo-service`. Documented in [limitations.md](limitations.md).

---

## Axum route syntax (`:param`)

**Context:** Axum 0.7 docs show `{param}` in some examples; in this workspace that syntax fails to match routes (404 with empty body).

**Decision:** All parameterized routes use **colon syntax** (`/visas/:id`, `/datasets/:dataset_id`).

**Consequences:** Integration tests must use `:param` when adding routes. Documented in [architecture.md](architecture.md) and [roadmap.md](roadmap.md).
