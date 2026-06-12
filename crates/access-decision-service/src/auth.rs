// SPDX-License-Identifier: Apache-2.0

//! OAuth2/OIDC and DAC API key authentication helpers.

use async_trait::async_trait;
use axum::extract::{FromRef, FromRequestParts};
use axum::http::request::Parts;
use axum::http::{header, HeaderMap};
use ga4gh_clearinghouse::JwksCache;
use ga4gh_types::PassportClaims;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::sync::Arc;

use crate::app::AppState;
use crate::error::AdsError;

/// Minimal JWT claims for researcher identity extraction.
#[derive(Debug, Deserialize)]
pub struct ResearcherTokenClaims {
    pub sub: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub email: Option<String>,
}

/// Authenticated researcher context from a Bearer JWT.
#[derive(Debug, Clone)]
pub struct AuthenticatedResearcher {
    pub sub: String,
    pub display_name: Option<String>,
    pub email: Option<String>,
}

/// DAC operator authenticated via API key.
#[derive(Debug, Clone)]
pub struct DacOperator {
    pub name: String,
}

pub fn hash_api_key(raw: &str) -> String {
    let digest = Sha256::digest(raw.as_bytes());
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

pub fn verify_api_key(raw: &str, stored_hash: &str) -> bool {
    let candidate = hash_api_key(raw);
    constant_time_eq(candidate.as_bytes(), stored_hash.as_bytes())
}

fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    left.iter()
        .zip(right.iter())
        .fold(0u8, |acc, (a, b)| acc | (a ^ b))
        == 0
}

pub fn extract_bearer(headers: &HeaderMap) -> Result<&str, AdsError> {
    let value = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .ok_or(AdsError::Unauthorized)?;
    value.strip_prefix("Bearer ").ok_or(AdsError::Unauthorized)
}

pub fn extract_api_key(headers: &HeaderMap) -> Result<&str, AdsError> {
    headers
        .get("X-API-Key")
        .and_then(|v| v.to_str().ok())
        .ok_or(AdsError::Unauthorized)
}

impl AuthenticatedResearcher {
    pub async fn from_headers(state: &AppState, headers: &HeaderMap) -> Result<Self, AdsError> {
        let token = extract_bearer(headers)?;
        Self::from_token(&state.jwks, token).await
    }

    pub async fn from_token(jwks: &Arc<JwksCache>, token: &str) -> Result<Self, AdsError> {
        if let Ok(claims) = jwks.verify_and_decode::<ResearcherTokenClaims>(token).await {
            return Ok(Self {
                sub: claims.sub,
                display_name: claims.name,
                email: claims.email,
            });
        }

        let passport: PassportClaims = jwks.verify_and_decode(token).await?;
        Ok(Self {
            sub: passport.sub,
            display_name: None,
            email: None,
        })
    }
}

impl DacOperator {
    pub async fn from_headers(state: &AppState, headers: &HeaderMap) -> Result<Self, AdsError> {
        let key = extract_api_key(headers)?;
        let name = state
            .store
            .verify_api_key(key)
            .await?
            .ok_or(AdsError::Unauthorized)?;
        Ok(Self { name })
    }
}

/// Axum extractor for DAC-protected routes.
pub struct RequireDac(pub DacOperator);

#[async_trait]
impl<S> FromRequestParts<S> for RequireDac
where
    S: Send + Sync,
    Arc<AppState>: FromRef<S>,
{
    type Rejection = AdsError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let app = Arc::<AppState>::from_ref(state);
        let operator = DacOperator::from_headers(&app, &parts.headers).await?;
        Ok(Self(operator))
    }
}

/// Axum extractor for researcher Bearer JWT.
pub struct RequireResearcher(pub AuthenticatedResearcher);

#[async_trait]
impl<S> FromRequestParts<S> for RequireResearcher
where
    S: Send + Sync,
    Arc<AppState>: FromRef<S>,
{
    type Rejection = AdsError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let app = Arc::<AppState>::from_ref(state);
        let researcher = AuthenticatedResearcher::from_headers(&app, &parts.headers).await?;
        Ok(Self(researcher))
    }
}

/// Bearer JWT or DAC API key for introspection from resource services.
pub struct RequireServiceAuth {
    pub sub: Option<String>,
    pub dac_name: Option<String>,
}

impl RequireServiceAuth {
    pub async fn from_headers(state: &AppState, headers: &HeaderMap) -> Result<Self, AdsError> {
        if let Ok(key) = extract_api_key(headers) {
            if let Some(name) = state.store.verify_api_key(key).await? {
                return Ok(Self {
                    sub: None,
                    dac_name: Some(name),
                });
            }
        }
        let researcher = AuthenticatedResearcher::from_headers(state, headers).await?;
        Ok(Self {
            sub: Some(researcher.sub),
            dac_name: None,
        })
    }
}

#[async_trait]
impl<S> FromRequestParts<S> for RequireServiceAuth
where
    S: Send + Sync,
    Arc<AppState>: FromRef<S>,
{
    type Rejection = AdsError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let app = Arc::<AppState>::from_ref(state);
        Self::from_headers(&app, &parts.headers).await
    }
}
