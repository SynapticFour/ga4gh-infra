# ADS database schema

Source: `crates/access-decision-service/migrations/001_init.sql`

Portable **PostgreSQL / SQLite** schema. Timestamps are Unix epoch **BIGINT** seconds.

## Tables

### researchers

| Column | Type | Notes |
|--------|------|-------|
| id | TEXT PK | OIDC `sub` |
| display_name | TEXT | optional |
| email | TEXT | optional |
| affiliations | TEXT | JSON array |
| created_at, updated_at | BIGINT | |

### datasets

| Column | Type | Notes |
|--------|------|-------|
| id | TEXT PK | UUID |
| name | TEXT | |
| description | TEXT | |
| duo_codes | TEXT | JSON array of DUO codes |
| external_id | TEXT | DRS / Beacon id |
| auto_approve_enabled | INTEGER | 0/1 |
| auto_approve_threshold | INTEGER | 0–100 |
| created_at, updated_at | BIGINT | |

### research_projects

| Column | Type |
|--------|------|
| id | TEXT PK |
| researcher_id | TEXT FK → researchers |
| name, description | TEXT |
| duo_codes | TEXT JSON |
| created_at, updated_at | BIGINT |

### access_requests

| Column | Type |
|--------|------|
| id | TEXT PK |
| researcher_id, dataset_id, project_id | TEXT FKs |
| status | TEXT | pending / approved / rejected / escalated |
| justification | TEXT |
| duo_evaluation | TEXT | JSON snapshot |
| created_at, updated_at | BIGINT |

### access_decisions

Immutable DAC audit trail.

| Column | Type |
|--------|------|
| id | TEXT PK |
| request_id | TEXT FK |
| outcome | TEXT |
| actor | TEXT |
| reason | TEXT |
| decided_at | BIGINT |

### grants

| Column | Type |
|--------|------|
| id | TEXT PK |
| researcher_id, dataset_id | TEXT |
| request_id | TEXT nullable |
| source | TEXT | dac_approval / duo_auto_approval / institutional_mapping |
| duo_codes | TEXT JSON |
| resource_scope | TEXT |
| expires_at, revoked_at | BIGINT nullable |
| created_at | BIGINT |

Index: `idx_grants_researcher_active` (non-revoked grants by researcher).

### visa_sources, permission_sources, permission_mappings

Configuration for visa export and institutional mappings.

### audit_events

| Column | Type |
|--------|------|
| id | TEXT PK |
| event_type | TEXT |
| payload | TEXT JSON |
| occurred_at | BIGINT |

### api_keys

DAC API keys (SHA-256 hash only).

## Migrations

Embedded via `sqlx::migrate!()`. PostgreSQL runs migrations when `database.auto_migrate = true`;
SQLite always migrates on connect.
