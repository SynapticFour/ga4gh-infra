# Production deployment guide

This guide consolidates TLS termination, secrets, URL configuration, upstream IdP wiring, and PostgreSQL operations for a single-institute production deployment. It builds on [deployment-scenarios.md](deployment-scenarios.md) and [configuration.md](configuration.md).

**Prerequisites:** Docker Compose (or equivalent orchestration), a public DNS name, TLS certificates, PostgreSQL 16+, and an OIDC-capable institute IdP (Keycloak, Microsoft Entra ID, ELIXIR AAI, etc.). SAML-only IdPs require an OIDC front (e.g. Keycloak) — see [limitations.md](limitations.md).

---

## Architecture overview

```text
Internet (HTTPS)
       │
       ▼
┌──────────────────┐
│ Reverse proxy    │  nginx / Caddy / Traefik — TLS termination
│ (443 → HTTP)     │
└────────┬─────────┘
         │ plain HTTP on internal network
         ├─► aai-broker:8080      (public: /login, /callback, /userinfo, /jwks.json)
         ├─► visa-registry:8081   (DAC API — restrict to internal network)
         ├─► duo-service:8082     (optional public read; /match often internal)
         ├─► service-registry:8083 (public read; writes internal only)
         └─► resource services     (your DRS/TES/etc.)
```

Services **must not** be exposed on plain HTTP to the public internet without a reverse proxy. JWT `iss` claims and clearinghouse trust depend on **`external_url`** matching what clients and resource services use over HTTPS.

---

## Step 1 — DNS and TLS

| Hostname | Service | Notes |
|----------|---------|-------|
| `aai.example.org` | aai-broker | Passport issuer; OIDC discovery for downstream |
| `visas.example.org` | visa-registry | Visa JWT issuer (optional separate hostname) |
| `registry.example.org` | service-registry | GA4GH Service Registry |
| `duo.example.org` | duo-service | Optional; can stay internal |
| `data.example.org` | resource APIs | Your datasets / DRS endpoints |

Obtain certificates via Let's Encrypt (Caddy auto-HTTPS) or your PKI. Example reverse-proxy configs live in [`docker/reverse-proxy/`](../docker/reverse-proxy/).

**Caddy (simplest):** copy [`docker/reverse-proxy/Caddyfile.example`](../docker/reverse-proxy/Caddyfile.example), set your domain, run Caddy on the host or as a container in front of Compose.

**nginx:** copy [`docker/reverse-proxy/nginx.conf.example`](../docker/reverse-proxy/nginx.conf.example), point `proxy_pass` at Compose service ports or an internal Docker network.

Ensure proxy headers preserve the public URL:

```nginx
proxy_set_header Host $host;
proxy_set_header X-Forwarded-Proto https;
proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
```

The broker sets `Secure` cookies; browsers require HTTPS on the public URL.

---

## Step 2 — `external_url` and JWT trust matrix

Every service's `server.external_url` must be the **public HTTPS base URL** (no trailing slash). Internal container URLs are only for service-to-service HTTP.

| Setting | Wrong (breaks validation) | Correct |
|---------|---------------------------|---------|
| Broker `external_url` | `http://aai-broker:8080` | `https://aai.example.org` |
| Passport JWT `iss` | mismatched host/scheme | same as broker `external_url` |
| Clearinghouse `trusted_issuers[].issuer` | internal Docker hostname | `https://aai.example.org` |
| Clearinghouse `jwks_uri` | `https://aai.example.org/jwks.json` via public URL **or** internal `http://aai-broker:8080/jwks.json` | Either works if issuer string matches JWT `iss` |
| Visa registry `external_url` | `http://visa-registry:8081` | `https://visas.example.org` |
| Broker `[[visa_sources]].url` | public URL | internal: `http://visa-registry:8081` |
| Broker `[[upstream_idps]].issuer` | broker's own URL | IdP issuer from discovery (Keycloak realm URL) |

Resource services validate Passports against **`iss`**, not against how they fetch JWKS. Fetch JWKS from an internal URL when the issuer string is the public HTTPS URL.

Example resource service clearinghouse config:

```toml
[[clearinghouse.trusted_issuers]]
issuer = "https://aai.example.org"
jwks_uri = "http://aai-broker:8080/jwks.json"

[[clearinghouse.trusted_issuers]]
issuer = "https://visas.example.org"
jwks_uri = "http://visa-registry:8081/jwks.json"
```

---

## Step 3 — Secrets and signing keys

**Never** use keys from [`docker/secrets/`](../docker/secrets/) in production.

| Secret | Purpose | How to provision |
|--------|---------|------------------|
| Broker RS256 PEM | Passport signing | `ga4gh-infra keygen --output /run/secrets/broker_rs256.pem` |
| Visa-registry RS256 PEM | Visa JWT signing | separate keypair |
| `BROKER_COOKIE_SECRET` | HMAC for RP session cookies | `openssl rand -base64 32` |
| `MY_INSTITUTE_CLIENT_SECRET` | OIDC client secret at IdP | from IdP admin console |
| `REGISTRY_BOOTSTRAP_API_KEY` | DAC visa API (visa-registry) | strong random; rotate after bootstrap |
| `SERVICE_REGISTRY_REGISTRATION_KEY` | Internal service registration | strong random; not exposed publicly |
| Postgres passwords | DB access | managed by DBA / secret store |

**Mounting keys in Docker:**

```yaml
volumes:
  - /etc/ga4gh/secrets/broker_rs256.pem:/secrets/broker_rs256.pem:ro
```

Or Docker Swarm/Kubernetes secrets mounted read-only at a fixed path referenced in TOML (`signing.private_key_pem`).

**Rotation:** publish new keys in JWKS with a new `kid` before retiring the old key. Overlap both keys in JWKS during rotation; see [limitations.md](limitations.md) for current gaps in automated rotation runbooks.

Template environment file: [`config/production.env.example`](../config/production.env.example).

---

## Step 4 — PostgreSQL

The stack uses **two logical databases** on one Postgres instance (see [`docker/postgres/init.sql`](../docker/postgres/init.sql)):

| Database | Service | Connection env |
|----------|---------|----------------|
| `visa_registry` | visa-registry | `REGISTRY_DATABASE_URL` |
| `service_registry` | service-registry | `SERVICE_REGISTRY_DATABASE_URL` |

Example URLs:

```text
postgres://ga4gh_app:SECRET@postgres.internal:5432/visa_registry
postgres://ga4gh_app:SECRET@postgres.internal:5432/service_registry
```

### Migrations

- **visa-registry:** migrations run on startup when `auto_migrate = true` (Postgres and SQLite). For production, consider running migrations in a controlled job and setting `auto_migrate = false` after initial deploy.
- **service-registry:** migrations run on every connect via `sqlx::migrate!()` in `ServiceStore::connect`.

Migration files:

- `crates/visa-registry/migrations/`
- `crates/service-registry/migrations/`

Manual migrate (visa-registry CLI pattern):

```bash
# From a machine with DATABASE_URL set and the visa-registry binary:
export REGISTRY_DATABASE_URL='postgres://...'
visa-registry --config visa-registry.toml
# Migrations run on startup when auto_migrate is enabled
```

### Backups

**Logical backup (recommended for portability):**

```bash
pg_dump -Fc -h postgres.internal -U ga4gh_app -d visa_registry \
  -f "visa_registry_$(date +%Y%m%d).dump"
pg_dump -Fc -h postgres.internal -U ga4gh_app -d service_registry \
  -f "service_registry_$(date +%Y%m%d).dump"
```

**Restore:**

```bash
pg_restore -h postgres.internal -U ga4gh_app -d visa_registry --clean --if-exists visa_registry_YYYYMMDD.dump
```

Schedule daily backups with retention; test restore quarterly. Store backups encrypted; visa data may identify researchers.

**Managed Postgres:** RDS, Cloud SQL, Azure Database for PostgreSQL work the same way — point `REGISTRY_DATABASE_URL` / `SERVICE_REGISTRY_DATABASE_URL` at the managed endpoint and run `init.sql` equivalents once to create both databases.

---

## Step 5 — Wire the institute IdP (Keycloak)

Full example broker config: [`config/broker.keycloak.example.toml`](../config/broker.keycloak.example.toml).

### Keycloak client setup

1. Create realm (or use existing institute realm).
2. **Clients → Create client**
   - Client ID: `ga4gh-broker`
   - Client authentication: **On** (confidential client)
   - Standard flow: **On**
   - Direct access grants: **Off** (recommended)
3. **Valid redirect URIs:** `https://aai.example.org/callback`
4. **Web origins:** `https://aai.example.org` (or `+` for same-origin)
5. Copy **client secret** → `MY_INSTITUTE_CLIENT_SECRET` env var.
6. Note **issuer** URL: `https://idp.example.org/realms/your-realm` (must match broker TOML exactly).

### Broker TOML (Keycloak)

```toml
[server]
host = "0.0.0.0"
port = 8080
external_url = "https://aai.example.org"
environment = "prod"

[[upstream_idps]]
name = "keycloak"
issuer = "https://idp.example.org/realms/your-realm"
client_id = "ga4gh-broker"
client_secret_env = "MY_INSTITUTE_CLIENT_SECRET"
scopes = ["openid", "profile", "email"]

[upstream_idps.claim_mapping]
sub = "sub"
email = "email"
affiliation = "eduperson_scoped_affiliation"
```

Adjust `claim_mapping` to match attributes your IdP releases (eduPerson, VO entitlements, etc.). The broker maps upstream claims to `ResearcherIdentity` before collecting visas.

### Microsoft Entra ID (brief)

1. App registration → **Web** redirect URI: `https://aai.example.org/callback`
2. Create client secret; set `MY_INSTITUTE_CLIENT_SECRET`.
3. Issuer: `https://login.microsoftonline.com/{tenant-id}/v2.0`
4. Enable ID tokens; grant `openid`, `profile`, `email` scopes.
5. Map optional claims (e.g. `preferred_username`) in `[upstream_idps.claim_mapping]`.

Test the flow: browser → `https://aai.example.org/login/keycloak` → IdP → callback → Bearer token from JSON response or `/userinfo`.

---

## Step 6 — Production Compose profile

Reference layout: [`docker/docker-compose.prod.example.yml`](../docker/docker-compose.prod.example.yml).

Changes from dev stack:

| Dev | Production |
|-----|------------|
| `mock-idp` | Remove; use real IdP |
| `environment = "development"` | `environment = "prod"` |
| Published ports on all services | Only reverse proxy on 443; internal network for services |
| `read_only = false` on service-registry | `read_only = true`; register via internal CI/job |
| Dev API keys in compose | Secrets from env file / secret manager |
| `external_url = http://localhost:...` | Public `https://...` URLs |

Pin image versions in `.env`:

```env
GA4GH_IMAGE_PREFIX=ghcr.io/synapticfour
AAI_BROKER_VERSION=0.1.0
VISA_REGISTRY_VERSION=0.1.0
```

Start internal stack, then reverse proxy:

```bash
docker compose -f docker/docker-compose.prod.example.yml --env-file config/production.env up -d
# Caddy/nginx terminates TLS → localhost:8080-8084 or docker network
```

---

## Step 7 — Service registry hardening

```toml
[server]
read_only = true
external_url = "https://registry.example.org"
environment = "prod"
```

Register services from an internal network only (CI job, `register-service.sh` behind VPN). Public clients use `GET /services` and `GET /service-info`; block `POST /services` at the reverse proxy if not using `read_only`.

---

## Step 8 — Health checks and monitoring

| Service | Health endpoint |
|---------|-----------------|
| aai-broker | `GET /service-info` |
| visa-registry | `GET /service-info` |
| duo-service | `GET /service-info` |
| service-registry | `GET /service-info` |

Monitor: HTTP 5xx rates, Postgres connection errors, JWKS fetch failures in resource service logs, disk usage on Postgres volumes.

Structured audit logging on Passport issuance is **not** yet implemented — see [roadmap.md](roadmap.md) Phase 13.

---

## Step 9 — Pre-go-live checklist

- [ ] TLS on all public hostnames; HSTS enabled at proxy
- [ ] Production RS256 keys generated and dev keys removed
- [ ] All `external_url` values are public HTTPS URLs
- [ ] Clearinghouse `trusted_issuers[].issuer` matches JWT `iss` on resource services
- [ ] IdP redirect URI registered for `/callback`
- [ ] Postgres backups scheduled and restore tested
- [ ] `read_only = true` on service-registry (or proxy blocks writes)
- [ ] DAC API (`visa-registry`) not exposed to internet without auth
- [ ] Image versions pinned; `mock-idp` not deployed
- [ ] Security review: [limitations.md](limitations.md) — no formal third-party audit yet

---

## Related documents

- [getting-started.md](getting-started.md) — local demo paths
- [deployment-scenarios.md](deployment-scenarios.md) — scenario overview
- [configuration.md](configuration.md) — all config fields
- [architecture.md](architecture.md) — component interactions
- [docker/reverse-proxy/README.md](../docker/reverse-proxy/README.md) — proxy config usage
