pub mod app;
pub mod clients;
pub mod config;
pub mod csv;
pub mod datetime;
pub mod duo;
pub mod entity;
pub mod error;
pub mod events;
pub mod handlers;
pub mod health;
pub mod roles;
pub mod session;
pub mod state;

pub use app::build_router;
pub use config::AdminUiConfig;
pub use state::AppState;

/// Start the admin UI HTTP server (blocks until shutdown).
pub async fn run(config: AdminUiConfig) -> anyhow::Result<()> {
    use std::net::SocketAddr;

    let listen_addr: SocketAddr = config.listen_addr.parse()?;
    let state = AppState::new(config);
    let app = build_router(state);

    tracing::info!(%listen_addr, "admin-ui listening");
    let listener = tokio::net::TcpListener::bind(listen_addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
