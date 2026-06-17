# Admin UI (Phase 9)

Server-rendered operations dashboard for GA4GH infrastructure deployments. Built with **Axum**, **Askama**, and **htmx** — no SPA build step, no npm dependency.

## Layout

```
┌─────────────────────────────────────────────────────────────┐
│ Sidebar (nav)          │ Main content                       │
│ · Dashboard            │ Page title + tables/forms/cards    │
│ · DAC Queue            │                                    │
│ · Datasets / Projects  │ Degraded sections inline when a    │
│ · Grants / Audit       │ backend service is unavailable     │
│ · Service Registry     │                                    │
│ ─ Admin ─              │                                    │
│ · Researchers          │                                    │
│ · Agreements           │                                    │
│ · System               │                                    │
│ [user / role / logout] │                                    │
└─────────────────────────────────────────────────────────────┘
```

Dark ops-dashboard styling (`admin.css`); responsive down to ~900px for tablet DAC review.

## Language and locale

The admin UI is **English only**. There is no language switcher or i18n layer — all labels, status badges, and messages are hardcoded in English.

Timestamps use **UTC** with a consistent `YYYY-MM-DD HH:MM UTC` display format and ISO 8601 `<time datetime="…">` tooltips (locale-neutral for international research teams).

Status labels follow Title Case where shown to operators: e.g. `Pending`, `Approved`, `Active`, `Revoked`.

## Pages

| Page | Path | Audience |
|------|------|----------|
| Dashboard | `/` | Operator; signing-key panel for Admin |
| DAC Queue | `/dac` | Operator (htmx partial refresh + row swap on action) |
| Datasets | `/datasets` | Operator view; Admin create/edit |
| Projects | `/projects` | Operator view; Admin create/edit |
| Grants | `/grants` | Operator scoped; Admin all + revoke + CSV |
| Audit Log | `/audit` | Operator scoped; Admin full + filters + CSV |
| Service Registry | `/services` | Operator view; Admin register/delete |
| Researchers | `/researchers` | Admin |
| Agreements | `/agreements` | Admin (templates, profiles, compatibility check) |
| System | `/system` | Admin (IdP read-only, JWKS, permission mappings) |

See [roles.md](roles.md) for the Operator vs Admin rationale per page.

## UX details

- **Entity names** — Dataset and project UUIDs are resolved to human-readable names (with external-id tooltips) across DAC queue, grants, audit log, and dashboard.
- **Activity feed** — Dashboard shows recent ADS audit events with plain-language labels (e.g. “Grant issued to … for Heidelberg Cancer Cohort”).
- **DAC actions** — Approve, reject, and escalate update the queue row in place via htmx; the UI uses the ADS response body so approved requests no longer disappear as empty rows.
- **DUO** — Inline compatibility hints on the DAC queue and dataset detail pages call duo-service for term labels.

Researchers submit access requests **outside** the admin UI — via ADS `POST /access-requests` with a Passport Bearer token (see [ads/integration.md](../ads/integration.md)). Operators review those requests in the DAC Queue.

## Authentication

1. Unauthenticated users → `/login`
2. Broker OIDC login → callback reads `#access_token` → `POST /auth/session`
3. Long-lived HMAC session cookie (`ga4gh_admin_session`)

This is a **real user session** for the dashboard. It is separate from the broker's short-lived RP login session used during OIDC code flow.

## Running

### Docker (recommended)

```bash
make up-local
# Admin UI: http://localhost:8095
# Log in as researcher@uni-heidelberg.de (Operator) or admin@uni-heidelberg.de (Admin)
```

Demo seed data is loaded automatically. Re-seed with `make seed`.

### Native

```bash
cargo run -p ga4gh-infra-cli -- admin-ui --config config/admin-ui.example.toml
```

Optional in all-in-one: add an `[admin_ui]` section to the combined TOML.

Docker ports: **8095** (postgres compose) or **8195** (sqlite compose).

## Architecture

```
Browser → admin-ui (Askama + htmx)
            ├── AAI broker (OIDC login, JWKS read-only on System page)
            ├── ADS (DAC queue, datasets, projects, grants, audit, permissions)
            ├── DUO service (term labels + picker)
            ├── Visa registry (health)
            ├── Service registry (list / admin CRUD)
            └── Agreement registry (templates, profiles, compatibility check)
```

Admin-ui has **no database** and **no domain logic** — it aggregates existing REST APIs as an authenticated client.

See [configuration.md](configuration.md) and [roles.md](roles.md).
