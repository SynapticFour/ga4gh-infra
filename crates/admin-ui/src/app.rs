use std::path::PathBuf;
use std::sync::Arc;

use axum::extract::DefaultBodyLimit;
use axum::routing::{get, post};
use axum::Router;
use tower_http::services::ServeDir;
use tower_http::trace::TraceLayer;

use crate::config::AdminUiConfig;
use crate::handlers;
use crate::state::AppState;

fn resolve_static_dir(config: &AdminUiConfig) -> PathBuf {
    if let Some(dir) = &config.static_dir {
        return dir.clone();
    }

    let manifest_static = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("static");
    if manifest_static.is_dir() {
        return manifest_static;
    }

    let docker_static = PathBuf::from("/app/static");
    if docker_static.is_dir() {
        return docker_static;
    }

    manifest_static
}

pub fn build_router(state: AppState) -> Router {
    let static_dir = resolve_static_dir(&state.config);

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
        .route(
            "/datasets",
            get(handlers::datasets::list_page).post(handlers::datasets::create),
        )
        .route("/datasets/:id", get(handlers::datasets::detail_page))
        .route("/datasets/:id/edit", post(handlers::datasets::update))
        .route(
            "/projects",
            get(handlers::projects::list_page).post(handlers::projects::create),
        )
        .route("/projects/:id", get(handlers::projects::detail_page))
        .route("/projects/:id/edit", post(handlers::projects::update))
        .route("/grants", get(handlers::grants::list_page))
        .route("/grants/export.csv", get(handlers::grants::export_csv))
        .route("/grants/:id/revoke", post(handlers::grants::revoke))
        .route(
            "/researchers",
            get(handlers::researchers::search_page).post(handlers::researchers::search),
        )
        .route("/audit", get(handlers::audit::list_page))
        .route("/audit/export.csv", get(handlers::audit::export_csv))
        .route("/services", get(handlers::services::list_page))
        .route("/services/register", post(handlers::services::register_service))
        .route(
            "/services/:id/delete",
            post(handlers::services::delete_service),
        )
        .route("/agreements", get(handlers::agreements::index_page))
        .route(
            "/agreements/compatibility-check",
            post(handlers::agreements::compatibility_check),
        )
        .route("/system", get(handlers::system::index_page))
        .route("/system/sources", post(handlers::system::create_source))
        .route("/system/mappings", post(handlers::system::create_mapping))
        .route(
            "/system/mappings/:id/delete",
            post(handlers::system::delete_mapping),
        )
        .nest_service("/static", ServeDir::new(static_dir))
        .layer(DefaultBodyLimit::max(1024 * 1024))
        .layer(TraceLayer::new_for_http())
        .with_state(Arc::new(state))
}
