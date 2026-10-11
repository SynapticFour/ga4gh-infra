// SPDX-License-Identifier: Apache-2.0

//! Token endpoint for confidential BFF clients (`response_mode=query`).

use std::sync::Arc;

use axum::extract::State;
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Form;
use axum::Json;
use serde::Deserialize;
use tracing::instrument;

use crate::app::AppState;
use crate::auth_codes::verify_s256_challenge;
use crate::error::BrokerError;
use crate::session::unix_now;

#[derive(Debug, Deserialize)]
pub struct TokenForm {
    pub grant_type: Option<String>,
    pub code: Option<String>,
    pub redirect_uri: Option<String>,
    pub code_verifier: Option<String>,
}

/// Exchange a one-time authorization code for a GA4GH Passport.
#[instrument(skip(state, headers, form))]
pub async fn oauth_token(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Form(form): Form<TokenForm>,
) -> Result<Response, BrokerError> {
    if !state
        .login_limiter
        .allow(&ga4gh_http::client_key(&headers))
        .await
    {
        return Err(BrokerError::TooManyRequests);
    }
    if form.grant_type.as_deref() != Some("authorization_code") {
        return Err(BrokerError::BadRequest("unsupported grant_type".into()));
    }
    let code = form
        .code
        .filter(|c| !c.is_empty())
        .ok_or_else(|| BrokerError::BadRequest("code required".into()))?;
    let redirect_uri = form
        .redirect_uri
        .filter(|u| !u.is_empty())
        .ok_or_else(|| BrokerError::BadRequest("redirect_uri required".into()))?;
    let verifier = form
        .code_verifier
        .filter(|v| !v.is_empty() && v.len() >= 43 && v.len() <= 128)
        .ok_or_else(|| BrokerError::BadRequest("code_verifier required".into()))?;

    let pending = state.auth_codes.take(&code)?;
    if pending.redirect_uri != redirect_uri {
        return Err(BrokerError::AuthenticationFailed);
    }
    if !verify_s256_challenge(&verifier, &pending.code_challenge) {
        return Err(BrokerError::AuthenticationFailed);
    }

    let expires_in = (pending.passport_exp - unix_now()).max(0);
    let json = serde_json::json!({
        "access_token": pending.passport_jwt,
        "token_type": "Bearer",
        "expires_in": expires_in,
        "scope": "openid ga4gh_passport_v1",
        "id_token": pending.id_token,
        "nonce": pending.client_nonce,
    });

    let mut response = (StatusCode::OK, Json(json)).into_response();
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        header::HeaderValue::from_static("no-store"),
    );
    Ok(response)
}
