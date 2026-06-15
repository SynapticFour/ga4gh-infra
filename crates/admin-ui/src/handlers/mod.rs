pub mod agreements;
pub mod audit;
pub mod auth;
pub mod dac;
pub mod dashboard;
pub mod datasets;
pub mod grants;
pub mod projects;
pub mod researchers;
pub mod services;
pub mod system;

use std::sync::Arc;

use askama::Template;
use axum::http::HeaderMap;

use crate::session::UserSession;
use crate::state::AppState;

pub fn display_name(session: &UserSession) -> String {
    session
        .display_name
        .clone()
        .or_else(|| session.email.clone())
        .unwrap_or_else(|| session.sub.clone())
}

pub fn role_label(session: &UserSession) -> &'static str {
    if session.is_admin {
        "Admin"
    } else {
        "Operator"
    }
}

#[derive(Template)]
#[template(path = "layout/base.html")]
pub struct BaseLayout<'a> {
    pub title: &'a str,
    pub user_name: String,
    pub role: &'a str,
    pub is_admin: bool,
    pub active: &'a str,
    pub active_admin: &'a str,
    pub content: String,
}

pub fn render_layout(
    title: &str,
    active: &str,
    session: &UserSession,
    inner: impl Template,
) -> Result<String, askama::Error> {
    let content = inner.render()?;
    BaseLayout {
        title,
        user_name: display_name(session),
        role: role_label(session),
        is_admin: session.is_admin,
        active,
        active_admin: active,
        content,
    }
    .render()
}

pub type SharedState = Arc<AppState>;

pub fn is_htmx(headers: &HeaderMap) -> bool {
    headers
        .get("HX-Request")
        .and_then(|v| v.to_str().ok())
        .map(|v| v == "true")
        .unwrap_or(false)
}

pub fn htmx_redirect(headers: &mut HeaderMap, location: &str) {
    headers.insert("HX-Redirect", location.parse().expect("valid header"));
}
