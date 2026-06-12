// SPDX-License-Identifier: Apache-2.0

//! Curated seed agreement templates (see `docs/agreement-registry/templates/`).

use ga4gh_types::AgreementTemplate;

use crate::error::AgreementRegistryError;

/// Load bundled seed templates from JSON files.
pub fn seed_templates() -> Result<Vec<AgreementTemplate>, AgreementRegistryError> {
    const SEEDS: &[&str] = &[
        include_str!("../seeds/ega-general-research-use-v1.json"),
        include_str!("../seeds/ega-health-medical-biomedical-v1.json"),
        include_str!("../seeds/duos-dbgap-gru-ncu-v1.json"),
        include_str!("../seeds/ega-disease-specific-mondo-v1.json"),
        include_str!("../seeds/bbmri-registered-access-illustrative-v1.json"),
    ];

    SEEDS
        .iter()
        .map(|json| {
            serde_json::from_str(json).map_err(|err| AgreementRegistryError::Parse(err.to_string()))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_all_seed_templates() {
        let templates = seed_templates().expect("seeds");
        assert_eq!(templates.len(), 5);
        assert!(templates
            .iter()
            .any(|t| t.id == "ega-general-research-use-v1"));
        assert!(templates
            .iter()
            .any(|t| t.id == "bbmri-registered-access-illustrative-v1" && t.is_illustrative));
    }
}
