-- SPDX-License-Identifier: Apache-2.0
-- Peer DRS service base URL for federated dataset access (GA4GH DRS 1.x path prefix).

ALTER TABLE datasets ADD COLUMN remote_drs_base_url TEXT;

CREATE INDEX IF NOT EXISTS idx_datasets_remote_drs ON datasets (remote_drs_base_url)
    WHERE remote_drs_base_url IS NOT NULL;
