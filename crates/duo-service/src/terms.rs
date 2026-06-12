// SPDX-License-Identifier: Apache-2.0

//! Compiled DUO term catalog loaded from build-time OWL parsing.

use std::collections::{HashMap, HashSet};

use ga4gh_types::DuoCode;
use serde::Deserialize;

use crate::error::DuoServiceError;

/// Category of a DUO term in the ontology.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DuoCategory {
    /// Primary data use permission.
    Permission,
    /// Additional data use modifier.
    Modifier,
}

/// A single DUO term with metadata compiled from the OWL source.
#[derive(Debug, Clone, Deserialize)]
pub struct DuoTerm {
    /// DUO shorthand code (e.g. `GRU`).
    pub code: String,
    /// OBO identifier (e.g. `DUO:0000042`).
    pub obo_id: String,
    /// Human-readable label.
    pub label: String,
    /// Term definition text.
    pub definition: String,
    /// Whether this term is a permission or modifier.
    pub category: DuoCategory,
    /// Direct parent OBO identifiers from the OWL `rdfs:subClassOf` graph.
    pub parents: Vec<String>,
    /// Whether the term is deprecated in the ontology.
    pub obsolete: bool,
}

#[derive(Debug, Deserialize)]
struct DuoTermsDocument {
    terms: Vec<DuoTerm>,
}

/// In-memory DUO catalog indexed by shorthand and OBO identifier.
#[derive(Debug, Clone)]
pub struct DuoCatalog {
    by_code: HashMap<String, DuoTerm>,
    by_obo_id: HashMap<String, DuoTerm>,
    ancestor_sets: HashMap<String, HashSet<String>>,
}

impl DuoCatalog {
    /// Load the compiled DUO catalog generated at build time.
    pub fn from_embedded() -> Result<Self, DuoServiceError> {
        static TERMS_JSON: &str = include_str!(concat!(env!("OUT_DIR"), "/duo_terms.json"));
        Self::from_json(TERMS_JSON)
    }

    fn from_json(json: &str) -> Result<Self, DuoServiceError> {
        let document: DuoTermsDocument = serde_json::from_str(json)
            .map_err(|err| DuoServiceError::Internal(format!("invalid DUO catalog: {err}")))?;

        let mut by_code = HashMap::new();
        let mut by_obo_id = HashMap::new();
        for term in document.terms {
            by_obo_id.insert(term.obo_id.clone(), term.clone());
            by_code.insert(term.code.clone(), term);
        }

        let mut ancestor_sets = HashMap::new();
        for obo_id in by_obo_id.keys() {
            ancestor_sets.insert(obo_id.clone(), collect_ancestors(obo_id, &by_obo_id));
        }

        Ok(Self {
            by_code,
            by_obo_id,
            ancestor_sets,
        })
    }

    /// Return all non-obsolete terms sorted by code.
    pub fn list_terms(&self) -> Vec<&DuoTerm> {
        let mut terms: Vec<_> = self
            .by_code
            .values()
            .filter(|term| !term.obsolete)
            .collect();
        terms.sort_by_key(|term| term.code.as_str());
        terms
    }

    /// Look up a term by shorthand code or OBO identifier.
    pub fn get(&self, code_or_id: &str) -> Option<&DuoTerm> {
        let normalized = normalize_lookup(code_or_id);
        self.by_code
            .get(normalized.as_str())
            .or_else(|| self.by_obo_id.get(normalized.as_str()))
    }

    /// Resolve a request code string to a catalog term.
    pub fn resolve(&self, raw: &str) -> Result<&DuoTerm, DuoServiceError> {
        self.get(raw)
            .ok_or_else(|| DuoServiceError::BadRequest(format!("unknown DUO code `{raw}`")))
    }

    /// Returns `true` when `researcher` satisfies the dataset requirement `required`.
    pub fn permission_satisfies(&self, researcher: &DuoTerm, required: &DuoTerm) -> bool {
        if required.code == "NRES" {
            return true;
        }
        if researcher.code == required.code {
            return true;
        }
        self.ancestor_sets
            .get(&researcher.obo_id)
            .is_some_and(|ancestors| ancestors.contains(&required.obo_id))
    }

    /// Parse a shorthand string into a [`DuoCode`] when recognized by ga4gh-types.
    pub fn parse_duo_code(raw: &str) -> Result<DuoCode, DuoServiceError> {
        let normalized = normalize_lookup(raw);
        normalized
            .parse::<DuoCode>()
            .map_err(|err| DuoServiceError::BadRequest(err.0))
    }
}

fn collect_ancestors(obo_id: &str, by_obo_id: &HashMap<String, DuoTerm>) -> HashSet<String> {
    let mut seen = HashSet::new();
    let mut stack = vec![obo_id.to_string()];
    while let Some(current) = stack.pop() {
        if !seen.insert(current.clone()) {
            continue;
        }
        if let Some(term) = by_obo_id.get(&current) {
            for parent in &term.parents {
                stack.push(parent.clone());
            }
        }
    }
    seen
}

fn normalize_lookup(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.starts_with("DUO:") || trimmed.starts_with("duo:") {
        trimmed.to_ascii_uppercase()
    } else if trimmed.starts_with("DUO_") || trimmed.starts_with("duo_") {
        trimmed.replace('_', ":").to_ascii_uppercase()
    } else {
        trimmed.to_ascii_uppercase()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_embedded_catalog_with_gru_and_hmb() {
        let catalog = DuoCatalog::from_embedded().expect("catalog");
        let gru = catalog.get("GRU").expect("gru");
        let hmb = catalog.get("HMB").expect("hmb");
        assert_eq!(gru.category, DuoCategory::Permission);
        assert!(catalog.permission_satisfies(hmb, gru));
        assert!(!catalog.permission_satisfies(gru, hmb));
    }

    #[test]
    fn resolves_obo_identifiers() {
        let catalog = DuoCatalog::from_embedded().expect("catalog");
        assert!(catalog.get("DUO:0000042").is_some());
        assert!(catalog.get("duo:0000042").is_some());
    }
}
