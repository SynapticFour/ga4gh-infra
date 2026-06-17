-- SPDX-License-Identifier: Apache-2.0
-- Dataset visibility lifecycle and resource typing (datasets vs compute pools).

ALTER TABLE datasets ADD COLUMN visibility TEXT NOT NULL DEFAULT 'institute';
ALTER TABLE datasets ADD COLUMN resource_type TEXT NOT NULL DEFAULT 'dataset';

CREATE INDEX IF NOT EXISTS idx_datasets_visibility ON datasets (visibility);
CREATE INDEX IF NOT EXISTS idx_datasets_resource_type ON datasets (resource_type);
