use std::sync::Arc;

use async_trait::async_trait;
use axum::extract::{FromRef, FromRequestParts};
use axum::http::request::Parts;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Redirect, Response};
use chrono::{Duration, Utc};
use jsonwebtoken::{
    dangerous::insecure_decode, decode, encode, DecodingKey, EncodingKey, Header, Validation,
};
use serde::{Deserialize, Serialize};

use crate::config::AdminUiConfig;
use crate::roles::is_admin;
use crate::state::AppState;

pub const SESSION_COOKIE: &str = "ga4gh_admin_session";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserSession {
    pub sub: String,
    pub display_name: Option<String>,
    pub email: Option<String>,
    pub groups: Vec<String>,
    pub is_admin: bool,
    pub exp: i64,
}

impl UserSession {
    pub fn from_access_token(
        token: &str,
        config: &AdminUiConfig,
    ) -> Result<Self, jsonwebtoken::errors::Error> {
        // Broker access token: read claims only (signature verified upstream by broker OIDC).
        let claims = insecure_decode::<serde_json::Value>(token)?.claims;

        let sub = claims
            .get("sub")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();
        let display_name = claims
            .get("name")
            .and_then(|v| v.as_str())
            .map(str::to_string);
        let email = claims
            .get("email")
            .and_then(|v| v.as_str())
            .map(str::to_string);

        let groups = extract_groups(&claims, &config.admin_claim);
        let is_admin = is_admin(&groups, &config.admin_claim_value);
        let exp = Utc::now()
            .checked_add_signed(Duration::hours(config.session_ttl_hours as i64))
            .unwrap_or_else(Utc::now)
            .timestamp();

        Ok(Self {
            sub,
            display_name,
            email,
            groups,
            is_admin,
            exp,
        })
    }

    pub fn encode(&self, secret: &str) -> Result<String, jsonwebtoken::errors::Error> {
        encode(
            &Header::default(),
            self,
            &EncodingKey::from_secret(secret.as_bytes()),
        )
    }
}

fn extract_groups(claims: &serde_json::Value, claim_name: &str) -> Vec<String> {
    claims
        .get(claim_name)
        .and_then(|v| {
            if let Some(arr) = v.as_array() {
                Some(
                    arr.iter()
                        .filter_map(|x| x.as_str().map(str::to_string))
                        .collect(),
                )
            } else {
                v.as_str().map(|s| vec![s.to_string()])
            }
        })
        .unwrap_or_default()
}

pub fn decode_session(cookie_value: &str, secret: &str) -> Option<UserSession> {
    let mut validation = Validation::default();
    validation.validate_exp = true;
    decode::<UserSession>(
        cookie_value,
        &DecodingKey::from_secret(secret.as_bytes()),
        &validation,
    )
    .ok()
    .map(|data| data.claims)
}

pub fn set_session_cookie(headers: &mut HeaderMap, token: &str, max_age_secs: u64) {
    let cookie =
        format!("{SESSION_COOKIE}={token}; HttpOnly; Path=/; SameSite=Lax; Max-Age={max_age_secs}");
    headers.insert(
        axum::http::header::SET_COOKIE,
        cookie.parse().expect("valid cookie"),
    );
}

pub fn clear_session_cookie(headers: &mut HeaderMap) {
    let cookie = format!("{SESSION_COOKIE}=; HttpOnly; Path=/; SameSite=Lax; Max-Age=0");
    headers.insert(
        axum::http::header::SET_COOKIE,
        cookie.parse().expect("valid cookie"),
    );
}

pub struct RequireAuth(pub UserSession);

impl RequireAuth {
    #[allow(clippy::result_large_err)]
    pub fn require_admin(&self) -> Result<(), Response> {
        if self.0.is_admin {
            Ok(())
        } else {
            Err((StatusCode::FORBIDDEN, "admin role required").into_response())
        }
    }
}

#[async_trait]
impl<S> FromRequestParts<S> for RequireAuth
where
    S: Send + Sync,
    Arc<AppState>: FromRef<S>,
{
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let app = Arc::<AppState>::from_ref(state);
        let cookie = parts
            .headers
            .get(axum::http::header::COOKIE)
            .and_then(|v| v.to_str().ok())
            .and_then(|cookies| {
                cookies.split(';').find_map(|pair| {
                    let (name, value) = pair.trim().split_once('=')?;
                    if name == SESSION_COOKIE {
                        Some(value.to_string())
                    } else {
                        None
                    }
                })
            });

        match cookie.and_then(|c| decode_session(&c, &app.config.session_secret)) {
            Some(session) => Ok(RequireAuth(session)),
            None => {
                if parts
                    .headers
                    .get("HX-Request")
                    .and_then(|v| v.to_str().ok())
                    == Some("true")
                {
                    Err(
                        (StatusCode::UNAUTHORIZED, "session expired — sign in again")
                            .into_response(),
                    )
                } else {
                    Err(Redirect::to("/login").into_response())
                }
            }
        }
    }
}
