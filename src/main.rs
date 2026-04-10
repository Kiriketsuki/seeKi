mod api;
mod app_mode;
mod auth;
mod config;
mod db;
mod embed;
mod ssh;
#[cfg(test)]
mod testutil;

use axum::Router;
use axum::http::Method;
use tower_http::cors::{AllowOrigin, CorsLayer};
use tracing_subscriber::EnvFilter;

use crate::app_mode::{AppMode, initial_mode};
use crate::config::{AppConfig, ConfigLoadError, SecretsConfig};
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

    let mode = match AppConfig::load() {
        Ok(config) => {
            config.tables.warn_overlaps();
            let secrets = SecretsConfig::load_from_cwd();
            let ssh_pair = config.ssh.as_ref().map(|s| (s, &secrets));
            tracing::info!("Connecting to database...");
            let db = DatabasePool::connect(&config.database, ssh_pair).await?;
            tracing::info!("Connected to database");
            initial_mode(Some(AppState { db, config }))
        }
        Err(ConfigLoadError::NotFound) => {
            tracing::info!("No config file found — starting in setup mode");
            initial_mode(None)
        }
        Err(ConfigLoadError::Invalid { path, source }) => {
            tracing::error!("Config file at {} is invalid: {source}", path.display());
            tracing::error!("Fix the config file or delete it to enter setup mode");
            anyhow::bail!("Invalid config at {}: {source}", path.display())
        }
    };

    let bind_addr = {
        let guard = mode.read().await;
        match &*guard {
            AppMode::Normal(state) => {
                format!("{}:{}", state.config.server.host, state.config.server.port)
            }
            AppMode::Setup => {
                std::env::var("SEEKI_BIND").unwrap_or_else(|_| "127.0.0.1:3141".to_string())
            }
        }
    };

    let app = Router::new()
        .nest("/api", api::router(mode.clone()))
        .layer(localhost_cors())
        .fallback(embed::handler);

    let listener = tokio::net::TcpListener::bind(&bind_addr).await?;
    tracing::info!("SeeKi listening on http://{bind_addr}");
    axum::serve(listener, app).await?;
    Ok(())
}

fn localhost_cors() -> CorsLayer {
    CorsLayer::new()
        .allow_origin(AllowOrigin::predicate(|origin, _| {
            if let Ok(s) = origin.to_str() {
                s == "http://localhost"
                    || s.starts_with("http://localhost:")
                    || s == "http://127.0.0.1"
                    || s.starts_with("http://127.0.0.1:")
                    || s == "http://[::1]"
                    || s.starts_with("http://[::1]:")
            } else {
                false
            }
        }))
        .allow_methods([Method::GET, Method::POST])
        .allow_headers([axum::http::header::CONTENT_TYPE])
}
