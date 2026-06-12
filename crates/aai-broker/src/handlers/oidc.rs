// SPDX-License-Identifier: Apache-2.0

//! Downstream-facing OIDC metadata, JWKS, and userinfo handlers.

use std::sync::Arc;

use axum::extract::State;
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use ga4gh_types::PassportClaims;
use jsonwebtoken::{decode, Validation};
use serde::Serialize;
use tracing::instrument;

use crate::app::AppState;
use crate::error::BrokerError;
use crate::session::unix_now;

/// Broker OIDC discovery document served to downstream GA4GH services.
#[derive(Debug, Serialize)]
pub struct OpenIdConfiguration {
    /// Broker issuer URL.
    pub issuer: String,
    /// Broker JWKS URL.
    pub jwks_uri: String,
    /// Broker userinfo endpoint.
    pub userinfo_endpoint: String,
    /// Supported OAuth scopes.
    pub scopes_supported: Vec<String>,
    /// Supported response types (broker does not expose its own `/authorize`).
    pub response_types_supported: Vec<String>,
    /// Supported subject identifier types.
    pub subject_types_supported: Vec<String>,
    /// Supported signing algorithms for broker-issued tokens.
    pub id_token_signing_alg_values_supported: Vec<String>,
}

/// Userinfo response including GA4GH Passport visas.
#[derive(Debug, Serialize)]
pub struct UserinfoResponse {
    /// Researcher subject identifier.
    pub sub: String,
    /// Email address when known from upstream authentication.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    /// Affiliation when known from upstream authentication.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub affiliation: Option<String>,
    /// Visa JWT strings from the validated Passport.
    #[serde(rename = "ga4gh_passport_v1")]
    pub ga4gh_passport_v1: Vec<String>,
}

/// Serve the broker's OIDC discovery document.
#[instrument(skip(state))]
pub async fn openid_configuration(State(state): State<Arc<AppState>>) -> Json<OpenIdConfiguration> {
    let issuer = state.config.issuer_url().to_string();
    Json(OpenIdConfiguration {
        jwks_uri: format!("{issuer}/jwks.json"),
        userinfo_endpoint: format!("{issuer}/userinfo"),
        issuer,
        scopes_supported: vec![
            "openid".to_string(),
            "profile".to_string(),
            "email".to_string(),
            "ga4gh_passport_v1".to_string(),
        ],
        response_types_supported: vec![],
        subject_types_supported: vec!["public".to_string()],
        id_token_signing_alg_values_supported: vec!["RS256".to_string()],
    })
}

/// Serve the broker's JSON Web Key Set for downstream Clearinghouses.
#[instrument(skip(state))]
pub async fn jwks(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    Json(state.keys.jwks().clone())
}

/// Return standard claims and `ga4gh_passport_v1` for a valid broker Passport JWT.
#[instrument(skip(state, headers))]
pub async fn userinfo(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Response, BrokerError> {
    let token = bearer_token(&headers).ok_or(BrokerError::InvalidAccessToken)?;
    let claims = decode_passport(&state, token)?;

    let profile = state.profiles.get(&claims.sub, unix_now());
    let response = UserinfoResponse {
        sub: claims.sub,
        email: profile.as_ref().and_then(|value| value.email.clone()),
        affiliation: profile.as_ref().and_then(|value| value.affiliation.clone()),
        ga4gh_passport_v1: claims.ga4gh_passport_v1,
    };

    Ok((StatusCode::OK, Json(response)).into_response())
}

fn bearer_token(headers: &HeaderMap) -> Option<&str> {
    headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
}

fn decode_passport(state: &AppState, token: &str) -> Result<PassportClaims, BrokerError> {
    let mut validation = Validation::new(jsonwebtoken::Algorithm::RS256);
    validation.set_issuer(&[state.config.issuer_url()]);
    validation.validate_aud = false;

    decode::<PassportClaims>(token, state.keys.decoding_key(), &validation)
        .map(|data| data.claims)
        .map_err(|_| BrokerError::InvalidAccessToken)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::ResearcherIdentity;
    use crate::passport::mint_passport_jwt;
    use crate::test_support::test_signing_keys;
    use axum::http::HeaderValue;

    #[test]
    fn extracts_bearer_token() {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::AUTHORIZATION,
            HeaderValue::from_static("Bearer passport-token"),
        );
        assert_eq!(bearer_token(&headers), Some("passport-token"));
    }

    #[test]
    fn decodes_minted_passport_for_userinfo() {
        let keys = test_signing_keys();
        let identity = ResearcherIdentity {
            sub: "researcher@example.org".to_string(),
            email: Some("researcher@example.org".to_string()),
            affiliation: None,
            extra: Default::default(),
        };
        let token = mint_passport_jwt(
            keys,
            "https://broker.example.org",
            &identity,
            &["visa-jwt".to_string()],
            3600,
        )
        .expect("mint");

        let mut validation = Validation::new(jsonwebtoken::Algorithm::RS256);
        validation.set_issuer(&["https://broker.example.org"]);
        validation.validate_aud = false;
        let claims = decode::<PassportClaims>(&token, keys.decoding_key(), &validation)
            .expect("decode")
            .claims;
        assert_eq!(claims.ga4gh_passport_v1, vec!["visa-jwt".to_string()]);
    }
}
