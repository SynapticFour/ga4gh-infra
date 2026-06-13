-- SPDX-License-Identifier: Apache-2.0
-- Portable schema for PostgreSQL and SQLite (TEXT ids, TEXT JSON, INTEGER timestamps).

CREATE TABLE IF NOT EXISTS registered_services (
    id TEXT PRIMARY KEY,
    url TEXT NOT NULL,
    service_info TEXT NOT NULL,
    registered_at BIGINT NOT NULL,
    updated_at BIGINT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_registered_services_id ON registered_services (id);
