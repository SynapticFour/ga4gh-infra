-- SPDX-License-Identifier: Apache-2.0
-- Portable schema for PostgreSQL and SQLite (TEXT ids, TEXT JSON, INTEGER timestamps).

CREATE TABLE IF NOT EXISTS visa_assertions (
    id TEXT PRIMARY KEY,
    sub TEXT NOT NULL,
    visa_type TEXT NOT NULL,
    value TEXT NOT NULL,
    source TEXT NOT NULL,
    by_authority TEXT,
    conditions TEXT,
    asserted BIGINT NOT NULL,
    created_at BIGINT NOT NULL,
    revoked_at BIGINT,
    expires_at BIGINT
);

CREATE INDEX IF NOT EXISTS idx_visa_assertions_sub_active
    ON visa_assertions (sub)
    WHERE revoked_at IS NULL;

CREATE TABLE IF NOT EXISTS api_keys (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    key_hash TEXT NOT NULL UNIQUE,
    created_at BIGINT NOT NULL,
    revoked_at BIGINT
);
