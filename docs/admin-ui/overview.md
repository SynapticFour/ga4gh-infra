# Admin UI (Phase 9)

Server-rendered operations dashboard for GA4GH infrastructure deployments. Built with **Axum**, **Askama**, and **htmx** — no SPA build step.

## Scope (pages 1–3)

| Page | Path | Description |
|------|------|-------------|
| Dashboard | `/` | Service health, pending DAC count, dataset count |
| DAC Queue | `/dac` | Live queue with htmx approve/reject/escalate |
| Datasets | `/datasets` | List, detail, admin-only registration form |

Pages 4–10 (grants, researchers, agreements, audit, settings, etc.) are planned for a follow-up phase.

## Authentication

1. Unauthenticated users are redirected to `/login`.
2. Login sends the browser to the **AAI broker** (`/login?return_url=…`).
3. After broker login, the callback page reads `#access_token=…` from the URL fragment and POSTs to `/auth/session`.
4. Admin-ui stores a **long-lived HMAC session cookie** (`ga4gh_admin_session`), separate from the broker’s short RP session.

### Roles

- **Operator** — view dashboard, DAC queue, datasets; perform DAC actions.
- **Admin** — operator capabilities plus **register datasets**.

Role is derived from an OIDC claim (default: `groups` contains `ga4gh-infra-admins`). See [roles.md](roles.md).

ADS mutations use the configured **`ads_dac_api_key`** server-side; the browser never sees the DAC key.

## Running locally

```bash
# With the docker stack up (broker :8080, ADS :8090, DUO :8082):
cargo run -p ga4gh-infra-cli -- admin-ui --config config/admin-ui.example.toml
```

Open http://localhost:8095 — sign in via broker, then use the sidebar.

## Docker

The `admin-ui` service is included in `docker/docker-compose.yml` on port **8095**.

## Configuration

See [configuration.md](configuration.md) and `config/admin-ui.example.toml`.

## Architecture

```
Browser ──► admin-ui (Askama + htmx)
              ├──► AAI broker (OIDC login)
              ├──► ADS (DAC queue, datasets)  [X-API-Key]
              ├──► DUO service (term picker)
              └──► service-info probes (health panel)
```

Agreement Registry has no HTTP endpoint yet; the dashboard shows it as unavailable.
