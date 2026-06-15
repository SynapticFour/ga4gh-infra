use std::sync::Arc;

use axum::extract::DefaultBodyLimit;
use axum::routing::{get, post};
use axum::Router;
use tower_http::services::ServeDir;
use tower_http::trace::TraceLayer;

use crate::handlers;
use crate::state::AppState;

pub fn build_router(state: AppState) -> Router {
    let static_dir = state
        .config
        .static_dir
        .clone()
        .unwrap_or_else(|| std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("static"));

    Router::new()
        .route("/", get(handlers::dashboard::dashboard))
        .route("/login", get(handlers::auth::login_page))
        .route("/auth/callback", get(handlers::auth::callback_page))
        .route("/auth/session", post(handlers::auth::establish_session))
        .route("/logout", post(handlers::auth::logout))
        .route("/dac", get(handlers::dac::queue_page))
        .route("/dac/queue", get(handlers::dac::queue_partial))
        .route("/dac/requests/:id/approve", post(handlers::dac::approve))
        .route("/dac/requests/:id/reject", post(handlers::dac::reject))
        .route("/dac/requests/:id/escalate", post(handlers::dac::escalate))
        .route("/datasets", get(handlers::datasets::list_page).post(handlers::datasets::create))
        .route("/datasets/:id", get(handlers::datasets::detail_page))
        .nest_service("/static", ServeDir::new(static_dir))
        .layer(DefaultBodyLimit::max(1024 * 1024))
        .layer(TraceLayer::new_for_http())
        .with_state(Arc::new(state))
}
