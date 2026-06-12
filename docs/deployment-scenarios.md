# Deployment scenarios

How to run `ga4gh-infra` in common environments.

## Single desktop / demo (all-in-one + SQLite)

**Goal:** One binary, minimal dependencies, local testing.

**Approach:**

1. Install via [`getting-started.md`](getting-started.md) native path, or build from source:
   ```bash
   cargo build --release -p ga4gh-infra-cli
   ```
2. Use [`config/all-in-one.native.toml.example`](../config/all-in-one.native.toml.example) with SQLite for visa-registry.
3. Generate keys: `ga4gh-infra keygen --output-dir ~/.config/ga4gh-infra/secrets`
4. Run: `ga4gh-infra all-in-one --config ~/.config/ga4gh-infra/all-in-one.toml`

**Raspberry Pi:** Same flow on 64-bit or 32-bit Raspberry Pi OS using prebuilt ARM binaries (see [getting-started.md](getting-started.md)).

**Limitations:** Service-registry still needs PostgreSQL unless you use Docker `just up-sqlite` instead. Not suitable for multi-user production load on SQLite.

---

## Single institute production (Docker Compose + PostgreSQL)

**Goal:** Separate services, production-like persistence, institute IdP integration.

**Full guide:** [production-deployment.md](production-deployment.md) — TLS, Keycloak, secrets, Postgres backups, URL matrix, go-live checklist.

**Approach:**

1. Copy and edit configs under `config/` or `docker/config/`.
2. Set real `external_url` values (public HTTPS URLs).
3. Mount production PEM keys; never use `docker/secrets/` keys.
4. Pin image versions in [`docker/.env.example`](../docker/.env.example):
   ```env
   GA4GH_IMAGE_PREFIX=ghcr.io/SynapticFour
   AAI_BROKER_VERSION=0.3.0
   VISA_REGISTRY_VERSION=0.1.5
   ```
5. Start: `just up` or `docker compose -f docker/docker-compose.yml --env-file docker/.env.example up -d`
6. Put **TLS termination** in front (nginx, Caddy, Traefik) — see [`docker/reverse-proxy/`](../docker/reverse-proxy/) and [production-deployment.md](production-deployment.md).
7. Set `read_only = true` on public service-registry; register services from an internal network only.

**Database:** Run PostgreSQL with backups; visa-registry and service-registry use separate databases (see `docker/postgres/init.sql`).

**IdP:** Configure `[[upstream_idps]]` with your institute OIDC metadata (Keycloak, Entra ID, ELIXIR AAI, etc.) — see [limitations.md](limitations.md) for SAML.

---

## Lighter local / edge (Docker + SQLite visas)

**Goal:** Avoid Postgres for visa storage; still use Compose for full multi-container demo.

```bash
just up-sqlite
```

Visa-registry persists to a Docker volume; service-registry still uses the Compose Postgres service.

---

## Multi-institute federation (future)

**Goal:** Multiple institutes, shared or federated visa sources, researcher home-org brokering.

**Today:** Each institute typically runs its own broker + visa-registry. Visa sources can point to remote registries via `[[visa_sources]]`. There is **no** built-in WAYF UI or central passport issuer.

**Future work (not implemented):**

- Institute picker / IdP discovery UX
- Federated trust policies between brokers
- Shared clearinghouse trust configuration across resource services

Document operational patterns here as they emerge; see [roadmap.md](roadmap.md) Phase 11+.

---

## URL matrix (Docker vs JWT claims)

| Context | Example |
|---------|---------|
| Browser / JWT `iss` | `https://aai.example.org` |
| Host e2e tests | `http://localhost:8080` |
| Container-to-container | `http://aai-broker:8080`, `http://visa-registry:8081` |
| Mock IdP in browser | Rewrite `mock-idp:9000` → `localhost:9000` on host |

Mismatch between `external_url` and what clients use breaks JWT validation in clearinghouse.

---

## ARM / Raspberry Pi deployment

| Method | Notes |
|--------|-------|
| Native binary | `install.sh` on Pi OS; prefer 64-bit OS |
| Docker | Build or pull `linux/arm64` / `linux/arm/v7` images when publishing multi-arch (see [docker/README.md](../docker/README.md)) |
| Source build | `cargo build --release -p ga4gh-infra-cli` on the Pi directly (slow but simple) |

Release binaries: `aarch64-unknown-linux-gnu`, `armv7-unknown-linux-gnueabihf` ([versioning.md](versioning.md)).
