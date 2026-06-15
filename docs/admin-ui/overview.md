# Admin UI (Phase 9)

Server-rendered operations dashboard for GA4GH infrastructure deployments. Built with **Axum**, **Askama**, and **htmx** — no SPA build step.

## Pages

| Page | Path | Audience |
|------|------|----------|
| Dashboard | `/` | Operator |
| DAC Queue | `/dac` | Operator |
| Datasets | `/datasets` | Operator view; Admin create |
| Projects | `/projects` | Operator view; Admin create |
| Grants | `/grants` | Operator view; Admin revoke |
| Audit Log | `/audit` | Operator |
| Service Registry | `/services` | Operator view; Admin remove |
| Researchers | `/researchers` | Admin |
| Agreements | `/agreements` | Admin (stub until HTTP service) |
| System | `/system` | Admin |

## Authentication

1. Unauthenticated users → `/login`
2. Broker OIDC login → callback reads `#access_token` → `POST /auth/session`
3. Long-lived HMAC session cookie (`ga4gh_admin_session`)

See [roles.md](roles.md) and [configuration.md](configuration.md).

## Running

```bash
cargo run -p ga4gh-infra-cli -- admin-ui --config config/admin-ui.example.toml
```

Optional in all-in-one: add an `[admin_ui]` section to the combined TOML.

Docker: port **8095** (postgres compose) or **8195** (sqlite compose).

## Architecture

```
Browser → admin-ui (Askama + htmx)
            ├── AAI broker (login, JWKS read-only on System page)
            ├── ADS (DAC, datasets, projects, grants, audit, permissions)
            ├── DUO service (term picker)
            └── Service registry (list / admin delete)
```

Agreement Registry remains library-only until a dedicated HTTP service is added.
