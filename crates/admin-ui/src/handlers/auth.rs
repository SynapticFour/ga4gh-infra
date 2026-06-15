use askama::Template;
use askama_axum::IntoResponse;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{Redirect, Response};
use axum::Form;
use serde::Deserialize;

use crate::handlers::SharedState;
use crate::session::{clear_session_cookie, set_session_cookie, RequireAuth, UserSession};

#[derive(Template)]
#[template(path = "auth/login.html")]
struct LoginTemplate {
    broker_login_url: String,
}

#[derive(Template)]
#[template(path = "auth/callback.html")]
struct CallbackTemplate {
    public_base_url: String,
}

#[derive(Debug, Deserialize)]
pub struct ReturnUrlQuery {
    pub return_url: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SessionForm {
    pub access_token: String,
}

pub async fn login_page(State(state): State<SharedState>) -> impl IntoResponse {
    let return_url = format!(
        "{}/auth/callback",
        state.config.public_base_url.trim_end_matches('/')
    );
    let broker_login_url = format!(
        "{}/login?return_url={}",
        state.config.broker_base_url.trim_end_matches('/'),
        urlencoding(return_url)
    );
    LoginTemplate { broker_login_url }.into_response()
}

pub async fn callback_page(State(state): State<SharedState>) -> impl IntoResponse {
    CallbackTemplate {
        public_base_url: state.config.public_base_url.clone(),
    }
    .into_response()
}

pub async fn establish_session(
    State(state): State<SharedState>,
    Form(form): Form<SessionForm>,
) -> Result<Response, StatusCode> {
    let session = UserSession::from_access_token(&form.access_token, &state.config)
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    let token = session
        .encode(&state.config.session_secret)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut headers = HeaderMap::new();
    set_session_cookie(&mut headers, &token, state.config.session_ttl().as_secs());
    headers.insert("HX-Redirect", "/".parse().unwrap());
    Ok((headers, StatusCode::NO_CONTENT).into_response())
}

pub async fn logout(_auth: RequireAuth, State(_state): State<SharedState>) -> Response {
    let mut headers = HeaderMap::new();
    clear_session_cookie(&mut headers);
    (headers, Redirect::to("/login")).into_response()
}

fn urlencoding(s: String) -> String {
    s.chars()
        .map(|c| match c {
            'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' | '.' | '~' => c.to_string(),
            _ => format!("%{:02X}", c as u8),
        })
        .collect()
}
