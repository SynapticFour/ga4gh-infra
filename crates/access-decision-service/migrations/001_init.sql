-- SPDX-License-Identifier: Apache-2.0
-- Portable schema for PostgreSQL and SQLite.

CREATE TABLE IF NOT EXISTS researchers (
    id TEXT PRIMARY KEY,
    display_name TEXT,
    email TEXT,
    affiliations TEXT NOT NULL DEFAULT '[]',
    created_at BIGINT NOT NULL,
    updated_at BIGINT NOT NULL
);

CREATE TABLE IF NOT EXISTS datasets (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    duo_codes TEXT NOT NULL,
    external_id TEXT,
    auto_approve_enabled INTEGER NOT NULL DEFAULT 0,
    auto_approve_threshold INTEGER NOT NULL DEFAULT 100,
    created_at BIGINT NOT NULL,
    updated_at BIGINT NOT NULL
);

CREATE TABLE IF NOT EXISTS research_projects (
    id TEXT PRIMARY KEY,
    researcher_id TEXT NOT NULL,
    name TEXT NOT NULL,
    description TEXT,
    duo_codes TEXT NOT NULL,
    created_at BIGINT NOT NULL,
    updated_at BIGINT NOT NULL,
    FOREIGN KEY (researcher_id) REFERENCES researchers(id)
);

CREATE INDEX IF NOT EXISTS idx_projects_researcher ON research_projects (researcher_id);

CREATE TABLE IF NOT EXISTS access_requests (
    id TEXT PRIMARY KEY,
    researcher_id TEXT NOT NULL,
    dataset_id TEXT NOT NULL,
    project_id TEXT NOT NULL,
    status TEXT NOT NULL,
    justification TEXT,
    duo_evaluation TEXT,
    created_at BIGINT NOT NULL,
    updated_at BIGINT NOT NULL,
    FOREIGN KEY (researcher_id) REFERENCES researchers(id),
    FOREIGN KEY (dataset_id) REFERENCES datasets(id),
    FOREIGN KEY (project_id) REFERENCES research_projects(id)
);

CREATE INDEX IF NOT EXISTS idx_access_requests_status ON access_requests (status);

CREATE TABLE IF NOT EXISTS access_decisions (
    id TEXT PRIMARY KEY,
    request_id TEXT NOT NULL,
    outcome TEXT NOT NULL,
    actor TEXT NOT NULL,
    reason TEXT,
    decided_at BIGINT NOT NULL,
    FOREIGN KEY (request_id) REFERENCES access_requests(id)
);

CREATE INDEX IF NOT EXISTS idx_access_decisions_request ON access_decisions (request_id);

CREATE TABLE IF NOT EXISTS grants (
    id TEXT PRIMARY KEY,
    researcher_id TEXT NOT NULL,
    dataset_id TEXT NOT NULL,
    request_id TEXT,
    source TEXT NOT NULL,
    duo_codes TEXT NOT NULL,
    resource_scope TEXT,
    expires_at BIGINT,
    revoked_at BIGINT,
    created_at BIGINT NOT NULL,
    FOREIGN KEY (researcher_id) REFERENCES researchers(id),
    FOREIGN KEY (dataset_id) REFERENCES datasets(id)
);

CREATE INDEX IF NOT EXISTS idx_grants_researcher_active
    ON grants (researcher_id)
    WHERE revoked_at IS NULL;

CREATE TABLE IF NOT EXISTS visa_sources (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    issuer_url TEXT NOT NULL,
    visa_type TEXT NOT NULL,
    enabled INTEGER NOT NULL DEFAULT 1,
    created_at BIGINT NOT NULL
);

CREATE TABLE IF NOT EXISTS permission_sources (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    oidc_issuer TEXT NOT NULL,
    claim_path TEXT NOT NULL,
    created_at BIGINT NOT NULL
);

CREATE TABLE IF NOT EXISTS permission_mappings (
    id TEXT PRIMARY KEY,
    source_id TEXT NOT NULL,
    claim_value TEXT NOT NULL,
    dataset_id TEXT NOT NULL,
    grant_lifetime_seconds BIGINT,
    created_at BIGINT NOT NULL,
    FOREIGN KEY (source_id) REFERENCES permission_sources(id),
    FOREIGN KEY (dataset_id) REFERENCES datasets(id)
);

CREATE TABLE IF NOT EXISTS audit_events (
    id TEXT PRIMARY KEY,
    event_type TEXT NOT NULL,
    payload TEXT NOT NULL,
    occurred_at BIGINT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_audit_events_type ON audit_events (event_type);

CREATE TABLE IF NOT EXISTS api_keys (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    key_hash TEXT NOT NULL UNIQUE,
    created_at BIGINT NOT NULL,
    revoked_at BIGINT
);
