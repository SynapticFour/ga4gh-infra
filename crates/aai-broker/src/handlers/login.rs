// SPDX-License-Identifier: Apache-2.0

//! Upstream login redirect handlers.

use axum::extract::{Path, Query, State};
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::{IntoResponse, Redirect, Response};
use serde::Deserialize;
use std::sync::Arc;
use tracing::instrument;

use crate::app::AppState;
use crate::error::BrokerError;
use crate::session::{unix_now, RpSession};

#[derive(Debug, Deserialize)]
pub struct LoginQuery {
    /// HTTPS (or localhost HTTP) URL to return to after passport issuance.
    pub return_url: Option<String>,
}

/// Start the upstream OIDC flow using the default configured IdP.
#[instrument(skip(state, headers))]
pub async fn login_default(
    State(state): State<Arc<AppState>>,
    Query(query): Query<LoginQuery>,
    headers: HeaderMap,
) -> Result<Response, BrokerError> {
    start_login(state, None, query.return_url, headers).await
}

/// Start the upstream OIDC flow for a named IdP.
#[instrument(skip(state, headers))]
pub async fn login_named(
    State(state): State<Arc<AppState>>,
    Path(idp_name): Path<String>,
    Query(query): Query<LoginQuery>,
    headers: HeaderMap,
) -> Result<Response, BrokerError> {
    start_login(state, Some(idp_name), query.return_url, headers).await
}

fn validate_return_url(raw: &str) -> Result<String, BrokerError> {
    let parsed = url::Url::parse(raw).map_err(|_| BrokerError::AuthenticationFailed)?;
    let host = parsed.host_str().ok_or(BrokerError::AuthenticationFailed)?;
    let ok_scheme = parsed.scheme() == "https"
        || (parsed.scheme() == "http"
            && (host == "localhost" || host == "127.0.0.1" || host.ends_with(".localhost")));
    if !ok_scheme {
        return Err(BrokerError::AuthenticationFailed);
    }
    Ok(raw.to_string())
}

async fn start_login(
    state: Arc<AppState>,
    idp_name: Option<String>,
    return_url: Option<String>,
    headers: HeaderMap,
) -> Result<Response, BrokerError> {
    let idp = match idp_name {
        Some(name) => state.upstream.get(&name)?,
        None => state.upstream.default()?,
    };

    let auth = idp.authorization_request()?;
    let return_url = return_url.as_deref().map(validate_return_url).transpose()?;
    let session = RpSession {
        idp_name: idp.name.clone(),
        csrf_state: auth.csrf_state,
        pkce_verifier: auth.pkce_verifier,
        nonce: Some(auth.nonce),
        created_at: unix_now(),
        return_url,
    };

    let cookie = state.sessions.create_set_cookie(&session)?;
    let set_cookie = cookie
        .parse()
        .map_err(|err| BrokerError::Internal(format!("invalid Set-Cookie header: {err}")))?;

    if headers
        .get(header::ACCEPT)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value.contains("application/json"))
    {
        let mut response = (
            StatusCode::OK,
            axum::Json(serde_json::json!({
                "authorization_url": auth.auth_url,
            })),
        )
            .into_response();
        response
            .headers_mut()
            .append(header::SET_COOKIE, set_cookie);
        return Ok(response);
    }

    let mut response = Redirect::temporary(&auth.auth_url).into_response();
    response
        .headers_mut()
        .append(header::SET_COOKIE, set_cookie);

    Ok(response)
}
