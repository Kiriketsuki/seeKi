mod api;
mod auth;
mod config;
mod db;

use std::sync::Arc;

use axum::Router;
use tower_http::cors::CorsLayer;
use tracing_subscriber::EnvFilter;

use crate::config::AppConfig;
use crate::db::DatabasePool;

pub struct AppState {
    pub db: DatabasePool,
    pub config: AppConfig,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("seeki=info".parse()?))
        .init();

    let config = AppConfig::load()?;
    let bind_addr = format!("{}:{}", config.server.host, config.server.port);

    tracing::info!("Connecting to database...");
    let db = DatabasePool::connect(&config.database).await?;
    tracing::info!("Connected successfully");

    let state = Arc::new(AppState { db, config });

    let app = Router::new()
        .nest("/api", api::router())
        .layer(CorsLayer::permissive())
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(&bind_addr).await?;
    tracing::info!("SeeKi listening on http://{bind_addr}");
    axum::serve(listener, app).await?;

    Ok(())
}
