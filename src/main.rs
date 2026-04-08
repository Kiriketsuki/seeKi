mod api;
mod auth;
mod config;
mod db;
#[cfg(test)]
mod testutil;

use std::sync::Arc;

use axum::Router;
use tower_http::cors::{AllowOrigin, CorsLayer};
use tracing_subscriber::EnvFilter;

use crate::config::{AppConfig, ConfigLoadError};
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

    match AppConfig::load() {
        Ok(config) => start_normal(config).await,
        Err(ConfigLoadError::NotFound) => start_setup_mode().await,
        Err(ConfigLoadError::Invalid { path, source }) => {
            tracing::error!(
                "Config file at {} is invalid: {source}",
                path.display()
            );
            tracing::error!("Fix the config file or delete it to enter setup mode");
            anyhow::bail!("Invalid config at {}: {source}", path.display())
        }
    }
}

/// Build a CORS layer that only allows localhost origins.
fn localhost_cors() -> CorsLayer {
    CorsLayer::new()
        .allow_origin(AllowOrigin::predicate(|origin, _| {
            if let Ok(s) = origin.to_str() {
                s.starts_with("http://localhost")
                    || s.starts_with("http://127.0.0.1")
                    || s.starts_with("http://[::1]")
            } else {
                false
            }
        }))
        .allow_methods(tower_http::cors::Any)
        .allow_headers(tower_http::cors::Any)
}

/// Normal mode: config exists, connect to DB and serve full API.
async fn start_normal(config: AppConfig) -> anyhow::Result<()> {
    config.tables.warn_overlaps();
    let bind_addr = format!("{}:{}", config.server.host, config.server.port);

    tracing::info!("Connecting to database...");
    let db = DatabasePool::connect(&config.database).await?;
    tracing::info!("Connected successfully");

    let state = Arc::new(AppState { db, config });

    let app = Router::new()
        .nest("/api", api::router())
        .layer(localhost_cors())
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(&bind_addr).await?;
    tracing::info!("SeeKi listening on http://{bind_addr}");
    axum::serve(listener, app).await?;

    Ok(())
}

/// Setup mode: no config found, serve only setup wizard endpoints.
async fn start_setup_mode() -> anyhow::Result<()> {
    let bind_addr = "127.0.0.1:3141";

    tracing::info!("No config file found — starting in setup mode");

    let app = Router::new()
        .nest("/api", api::setup::router())
        .layer(localhost_cors());

    let listener = tokio::net::TcpListener::bind(bind_addr).await?;
    tracing::info!("SeeKi setup wizard listening on http://{bind_addr}");
    tracing::info!("Configure your database connection, then restart the app");
    axum::serve(listener, app).await?;

    Ok(())
}
