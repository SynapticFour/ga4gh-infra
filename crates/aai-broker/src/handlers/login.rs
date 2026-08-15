// SPDX-License-Identifier: Apache-2.0

//! Upstream login redirect handlers.

use axum::extract::{Path, Query, State};
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::{IntoResponse, Redirect, Response};
use serde::Deserialize;
use std::sync::Arc;
use tracing::instrument;
use url::Url;

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

fn origin_key(parsed: &Url) -> Option<String> {
    let host = parsed.host_str()?;
    let port = parsed.port_or_known_default().unwrap_or(0);
    Some(format!(
        "{}://{}:{port}",
        parsed.scheme(),
        host.to_ascii_lowercase()
    ))
}

fn validate_return_url(
    raw: &str,
    allowed_origins: &[String],
    allow_localhost_http: bool,
) -> Result<String, BrokerError> {
    let parsed = Url::parse(raw).map_err(|_| BrokerError::AuthenticationFailed)?;
    if parsed.username() != "" || parsed.password().is_some() {
        return Err(BrokerError::AuthenticationFailed);
    }
    let origin = origin_key(&parsed).ok_or(BrokerError::AuthenticationFailed)?;

    if !allowed_origins.is_empty() {
        let allowed = allowed_origins.iter().any(|candidate| {
            Url::parse(candidate)
                .ok()
                .and_then(|url| origin_key(&url))
                .is_some_and(|key| key == origin)
        });
        if !allowed {
            return Err(BrokerError::AuthenticationFailed);
        }
        return Ok(raw.to_string());
    }

    if !allow_localhost_http {
        return Err(BrokerError::AuthenticationFailed);
    }

    let host = parsed.host_str().ok_or(BrokerError::AuthenticationFailed)?;
    let ok_scheme = parsed.scheme() == "http"
        && (host == "localhost" || host == "127.0.0.1" || host.ends_with(".localhost"));
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
    if !state
        .login_limiter
        .allow(&ga4gh_http::client_key(&headers))
        .await
    {
        return Err(BrokerError::TooManyRequests);
    }
    let return_url = return_url
        .as_deref()
        .map(|raw| {
            validate_return_url(
                raw,
                &state.config.server.allowed_return_url_origins,
                state.config.is_development(),
            )
        })
        .transpose()?;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allowlist_accepts_configured_origin() {
        let allowed = vec!["http://localhost:8095".to_string()];
        assert!(
            validate_return_url("http://localhost:8095/auth/callback", &allowed, false).is_ok()
        );
    }

    #[test]
    fn allowlist_rejects_other_https_origin() {
        let allowed = vec!["https://admin.example.org".to_string()];
        assert!(validate_return_url("https://evil.example/steal", &allowed, false).is_err());
    }

    #[test]
    fn empty_allowlist_rejects_https_outside_development() {
        assert!(validate_return_url("https://evil.example/steal", &[], false).is_err());
    }

    #[test]
    fn development_without_allowlist_permits_localhost_only() {
        assert!(validate_return_url("http://localhost:8095/cb", &[], true).is_ok());
        assert!(validate_return_url("https://evil.example/cb", &[], true).is_err());
    }

    #[test]
    fn rejects_embedded_userinfo() {
        let allowed = vec!["http://localhost:8095".to_string()];
        assert!(validate_return_url("http://evil@localhost:8095/cb", &allowed, true).is_err());
    }
}
