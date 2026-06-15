-- SPDX-License-Identifier: Apache-2.0
-- DAC group scoping for datasets and access requests.

ALTER TABLE datasets ADD COLUMN dac_group TEXT;
ALTER TABLE access_requests ADD COLUMN dac_group TEXT;

CREATE INDEX IF NOT EXISTS idx_datasets_dac_group ON datasets (dac_group);
CREATE INDEX IF NOT EXISTS idx_access_requests_dac_group ON access_requests (dac_group);
