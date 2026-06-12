# Configuration reference

All services load TOML configuration files. Environment variables override or supplement config via `*_env` fields or the `REGISTRY__` / service-specific prefixes documented in each crate.

Example files live in [`config/`](../config/). Docker uses [`docker/config/`](../docker/config/).

---

## aai-broker

File: `broker.toml` — see [`config/broker.example.toml`](../config/broker.example.toml)

### `[server]`

| Field | Type | Description |
|-------|------|-------------|
| `host` | string | Bind address (e.g. `0.0.0.0`) |
| `port` | u16 | Listen port (default stack: `8080`) |
| `external_url` | string | Public base URL, no trailing slash; used as Passport JWT `iss` |
| `environment` | string | `prod`, `test`, `dev`, `staging`, or `development` — controls trace logging policy |

### `[signing]`

| Field | Type | Description |
|-------|------|-------------|
| `private_key_pem` | path | RS256 PKCS#8 PEM for Passport signing |
| `passport_lifetime_seconds` | u64 | Passport JWT `exp` offset from issuance |
| `token_lifetime_seconds` | u64 | OAuth access token lifetime for `/userinfo` |

### `[session]`

| Field | Type | Description |
|-------|------|-------------|
| `cookie_secret_env` | string | Env var holding HMAC secret for RP session cookies |
| `session_lifetime_seconds` | u64 | RP session cookie TTL |

### `[[upstream_idps]]`

| Field | Type | Description |
|-------|------|-------------|
| `name` | string | Short name for `/login/:idp_name` |
| `issuer` | URL | Upstream OIDC issuer (`iss`) |
| `client_id` | string | OAuth client ID registered at the IdP |
| `client_secret_env` | string | Env var for client secret |
| `scopes` | string[] | OIDC scopes (must include `openid`) |

#### `[upstream_idps.claim_mapping]`

Maps upstream JWT claim names to GA4GH identity fields (`sub`, `email`, `affiliation`, etc.).

### `[[visa_sources]]`

| Field | Type | Description |
|-------|------|-------------|
| `name` | string | Label for logs |
| `url` | URL | Visa registry base URL (`GET /visas?sub=`) |

---

## visa-registry

File: `visa-registry.toml` — see [`config/visa-registry.example.toml`](../config/visa-registry.example.toml)

### `[server]`

Same shape as broker (`host`, `port`, `external_url`, `environment`).

### `[signing]`

| Field | Type | Description |
|-------|------|-------------|
| `private_key_pem` | path | RS256 PEM for visa JWT signing |
| `visa_lifetime_seconds` | u64 | Default visa JWT lifetime |

### `[database]`

| Field | Type | Description |
|-------|------|-------------|
| `driver` | string | `postgres` (default) or `sqlite` |
| `url` | string | Optional inline connection URL |
| `url_env` | string | Env var for URL when `url` omitted (default: `REGISTRY_DATABASE_URL`) |
| `auto_migrate` | bool | Run migrations on startup (PostgreSQL only; SQLite always migrates) |

**SQLite example:**

```toml
[database]
driver = "sqlite"
url = "sqlite:///var/lib/ga4gh/visa_registry.sqlite"
```

**PostgreSQL example:**

```toml
[database]
driver = "postgres"
url_env = "REGISTRY_DATABASE_URL"
auto_migrate = true
```

### `[auth]`

| Field | Type | Description |
|-------|------|-------------|
| `bootstrap_api_key_env` | string | Env var; registered on first startup if no API keys exist |

DAC requests use header `X-API-Key`.

---

## duo-service

File: `duo-service.toml` — see [`config/duo-service.example.toml`](../config/duo-service.example.toml)

### `[server]`

`host`, `port`, `external_url`, `environment` only. DUO catalog is embedded at build time.

---

## service-registry

File: `service-registry.toml` — see [`config/service-registry.example.toml`](../config/service-registry.example.toml)

### `[server]`

| Field | Type | Description |
|-------|------|-------------|
| `host`, `port`, `external_url`, `environment` | | Same as other services |
| `read_only` | bool | When `true`, reject registration writes (public production mode) |

### `[database]`

| Field | Type | Description |
|-------|------|-------------|
| `url_env` | string | PostgreSQL URL env (default: `SERVICE_REGISTRY_DATABASE_URL`) |

### `[auth]`

| Field | Type | Description |
|-------|------|-------------|
| `registration_api_key_env` | string | Env var for write API key |

---

## sample-resource

File: `sample-resource.toml` — see [`config/sample-resource.example.toml`](../config/sample-resource.example.toml)

### `[server]`

Standard server block.

### `[clearinghouse]`

| Field | Type | Description |
|-------|------|-------------|
| `jwks_cache_ttl_seconds` | u64 | JWKS cache TTL for passport/visa validation |

### `[[clearinghouse.trusted_issuers]]`

| Field | Type | Description |
|-------|------|-------------|
| `issuer` | URL | Expected JWT `iss` |
| `jwks_uri` | URL | JWKS document URL |

### `[duo_service]`

| Field | Type | Description |
|-------|------|-------------|
| `url` | URL | Base URL of duo-service for `/match` calls |

### `[[datasets]]`

| Field | Type | Description |
|-------|------|-------------|
| `id` | string | Dataset identifier in URL paths |
| `name`, `description` | string | Display metadata |
| `duo` | string[] | Required DUO codes for the dataset |
| `default_intended_use` | string[] | Default intended-use codes when header absent |

---

## ga4gh-infra all-in-one

File: `all-in-one.toml` — see [`config/all-in-one.example.toml`](../config/all-in-one.example.toml)

Nested sections mirror standalone configs:

- `[broker]` — full broker config
- `[visa_registry]`
- `[duo_service]`
- `[service_registry]`

Native install template: [`config/all-in-one.native.toml.example`](../config/all-in-one.native.toml.example) (SQLite visas, `{{CONFIG_DIR}}` placeholders).

Environment template: [`config/env.native.example`](../config/env.native.example).

---

## Environment variables (development defaults)

| Variable | Used by | Dev default |
|----------|---------|-------------|
| `BROKER_COOKIE_SECRET` | broker | `dev-broker-cookie-secret` |
| `MOCK_IDP_CLIENT_SECRET` | broker → mock-idp | `mock-client-secret` |
| `REGISTRY_DATABASE_URL` | visa-registry | Postgres in Docker stack |
| `REGISTRY_BOOTSTRAP_API_KEY` | visa-registry | `dev-visa-api-key` |
| `SERVICE_REGISTRY_DATABASE_URL` | service-registry | Postgres in Docker stack |
| `SERVICE_REGISTRY_REGISTRATION_KEY` | service-registry | `dev-service-registry-key` |

Override via `REGISTRY__` prefix for visa-registry config (double underscore nesting).

See [deployment-scenarios.md](deployment-scenarios.md) for production guidance on `external_url` vs internal Docker hostnames.
