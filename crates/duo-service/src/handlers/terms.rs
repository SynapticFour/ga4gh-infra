// SPDX-License-Identifier: Apache-2.0

//! DUO term listing and lookup handlers.

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::Json;
use serde::Serialize;
use tracing::instrument;

use crate::app::AppState;
use crate::error::DuoServiceError;
use crate::terms::DuoTerm;

/// Public DUO term representation returned by the API.
#[derive(Debug, Serialize)]
pub struct TermResponse {
    /// DUO shorthand code.
    pub code: String,
    /// OBO identifier.
    pub obo_id: String,
    /// Human-readable label.
    pub label: String,
    /// Term definition.
    pub definition: String,
    /// Permission or modifier category.
    pub category: String,
    /// Whether the term is obsolete in the ontology.
    pub obsolete: bool,
}

/// Response body for `GET /terms`.
#[derive(Debug, Serialize)]
pub struct TermsListResponse {
    /// All non-obsolete DUO terms.
    pub terms: Vec<TermResponse>,
}

/// List all non-obsolete DUO terms.
#[instrument(skip(state))]
pub async fn list_terms(State(state): State<Arc<AppState>>) -> Json<TermsListResponse> {
    let terms = state
        .catalog
        .list_terms()
        .into_iter()
        .map(term_to_response)
        .collect();
    Json(TermsListResponse { terms })
}

/// Return metadata for a single DUO term.
#[instrument(skip(state))]
pub async fn get_term(
    State(state): State<Arc<AppState>>,
    Path(code): Path<String>,
) -> Result<Json<TermResponse>, DuoServiceError> {
    let term = state.catalog.get(&code).ok_or(DuoServiceError::NotFound)?;
    Ok(Json(term_to_response(term)))
}

fn term_to_response(term: &DuoTerm) -> TermResponse {
    TermResponse {
        code: term.code.clone(),
        obo_id: term.obo_id.clone(),
        label: term.label.clone(),
        definition: term.definition.clone(),
        category: format!("{:?}", term.category).to_ascii_lowercase(),
        obsolete: term.obsolete,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{DuoServiceConfig, ServerConfig};
    use crate::terms::DuoCatalog;

    fn state() -> Arc<AppState> {
        Arc::new(AppState {
            config: DuoServiceConfig {
                server: ServerConfig {
                    host: "127.0.0.1".to_string(),
                    port: 8082,
                    external_url: "https://duo.example.org".to_string(),
                    environment: "test".to_string(),
                },
            },
            catalog: DuoCatalog::from_embedded().expect("catalog"),
        })
    }

    #[tokio::test]
    async fn lists_terms_and_fetches_single_code() {
        let state = state();
        let list = list_terms(State(state.clone())).await;
        assert!(list.terms.iter().any(|term| term.code == "GRU"));

        let term = get_term(State(state), Path("GRU".to_string()))
            .await
            .expect("get term");
        assert_eq!(term.code, "GRU");
    }
}
