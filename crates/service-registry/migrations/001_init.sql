-- SPDX-License-Identifier: Apache-2.0

CREATE TABLE IF NOT EXISTS registered_services (
    id TEXT PRIMARY KEY,
    url TEXT NOT NULL,
    service_info JSONB NOT NULL,
    registered_at BIGINT NOT NULL,
    updated_at BIGINT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_registered_services_type
    ON registered_services ((service_info->'type'->>'artifact'));
