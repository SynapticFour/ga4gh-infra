use askama::Template;
use askama_axum::IntoResponse;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::Html;

use crate::handlers::{render_layout, SharedState};
use crate::session::RequireAuth;

#[derive(Template)]
#[template(path = "agreements/index.html")]
struct AgreementsInner;

pub async fn index_page(auth: RequireAuth, State(_state): State<SharedState>) -> impl IntoResponse {
    if auth.require_admin().is_err() {
        return auth.require_admin().unwrap_err().into_response();
    }
    match render_layout("Agreements", "agreements", &auth.0, AgreementsInner) {
        Ok(html) => Html(html).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}
