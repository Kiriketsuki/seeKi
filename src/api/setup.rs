use axum::{Json, Router, routing::post};
use serde::{Deserialize, Serialize};

use crate::config::{AppConfig, DatabaseKind};
use crate::db::postgres;

pub fn router() -> Router {
    Router::new()
        .route("/setup/test-connection", post(test_connection))
        .route("/setup/save", post(save_config))
}

#[derive(Deserialize)]
struct TestConnectionRequest {
    kind: String,
    url: String,
}

#[derive(Serialize)]
struct TestConnectionResponse {
    success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    tables: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

async fn test_connection(
    Json(req): Json<TestConnectionRequest>,
) -> Json<TestConnectionResponse> {
    let kind = match parse_db_kind(&req.kind) {
        Ok(k) => k,
        Err(e) => {
            return Json(TestConnectionResponse {
                success: false,
                tables: None,
                error: Some(e),
            });
        }
    };

    match kind {
        DatabaseKind::Postgres => match postgres::test_connection(&req.url).await {
            Ok(tables) => Json(TestConnectionResponse {
                success: true,
                tables: Some(tables),
                error: None,
            }),
            Err(e) => {
                tracing::error!(error = %e, "test_connection failed");
                Json(TestConnectionResponse {
                    success: false,
                    tables: None,
                    error: Some("Failed to connect to database. Check your connection URL and ensure the database is running.".to_string()),
                })
            }
        },
        DatabaseKind::Sqlite => Json(TestConnectionResponse {
            success: false,
            tables: None,
            error: Some("SQLite support coming in v0.2".to_string()),
        }),
    }
}

#[derive(Deserialize)]
struct SaveConfigRequest {
    server: SaveServerConfig,
    database: SaveDatabaseConfig,
}

#[derive(Deserialize)]
struct SaveServerConfig {
    #[serde(default = "default_host")]
    host: String,
    #[serde(default = "default_port")]
    port: u16,
}

#[derive(Deserialize)]
struct SaveDatabaseConfig {
    kind: String,
    url: String,
    #[serde(default = "default_max_connections")]
    max_connections: u32,
}

fn default_host() -> String {
    "127.0.0.1".to_string()
}

fn default_port() -> u16 {
    3141
}

fn default_max_connections() -> u32 {
    5
}

#[derive(Serialize)]
struct SaveConfigResponse {
    success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

async fn save_config(Json(req): Json<SaveConfigRequest>) -> Json<SaveConfigResponse> {
    // Validate the database kind
    let kind = match parse_db_kind(&req.database.kind) {
        Ok(k) => k,
        Err(e) => {
            return Json(SaveConfigResponse {
                success: false,
                error: Some(e),
            });
        }
    };

    // Verify the connection is reachable before writing config
    match kind {
        DatabaseKind::Postgres => {
            if let Err(e) = postgres::test_connection(&req.database.url).await {
                tracing::error!(error = %e, "save_config: connection test failed");
                return Json(SaveConfigResponse {
                    success: false,
                    error: Some(
                        "Cannot connect to the database. Check your connection URL and ensure the database is running.".to_string(),
                    ),
                });
            }
        }
        DatabaseKind::Sqlite => {
            return Json(SaveConfigResponse {
                success: false,
                error: Some("SQLite support coming in v0.2".to_string()),
            });
        }
    }

    // Build a typed struct and serialize via toml to prevent injection
    let config_to_write = toml::value::Table::from_iter([
        (
            "server".to_string(),
            toml::Value::Table(toml::value::Table::from_iter([
                ("host".to_string(), toml::Value::String(req.server.host)),
                (
                    "port".to_string(),
                    toml::Value::Integer(req.server.port as i64),
                ),
            ])),
        ),
        (
            "database".to_string(),
            toml::Value::Table(toml::value::Table::from_iter([
                (
                    "kind".to_string(),
                    toml::Value::String(req.database.kind),
                ),
                ("url".to_string(), toml::Value::String(req.database.url)),
                (
                    "max_connections".to_string(),
                    toml::Value::Integer(req.database.max_connections as i64),
                ),
            ])),
        ),
    ]);

    let toml_content = match toml::to_string_pretty(&config_to_write) {
        Ok(s) => s,
        Err(e) => {
            return Json(SaveConfigResponse {
                success: false,
                error: Some(format!("Failed to serialize config: {e}")),
            });
        }
    };

    // Validate the generated TOML parses as a valid AppConfig
    if let Err(e) = AppConfig::parse(&toml_content) {
        return Json(SaveConfigResponse {
            success: false,
            error: Some(format!("Generated config is invalid: {e}")),
        });
    }

    match std::fs::write("seeki.toml", &toml_content) {
        Ok(()) => Json(SaveConfigResponse {
            success: true,
            error: None,
        }),
        Err(e) => Json(SaveConfigResponse {
            success: false,
            error: Some(format!("Failed to write seeki.toml: {e}")),
        }),
    }
}

fn parse_db_kind(kind: &str) -> Result<DatabaseKind, String> {
    match kind {
        "postgres" => Ok(DatabaseKind::Postgres),
        "sqlite" => Ok(DatabaseKind::Sqlite),
        other => Err(format!("Unsupported database kind: {other}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::Body, http::Request};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    fn setup_router() -> Router {
        router()
    }

    #[tokio::test]
    async fn test_connection_invalid_kind() {
        let app = setup_router();
        let body = serde_json::json!({
            "kind": "mysql",
            "url": "mysql://localhost/db"
        });

        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/setup/test-connection")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_string(&body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), 200);
        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(json["success"], false);
        assert!(json["error"].as_str().unwrap().contains("Unsupported"));
    }

    #[tokio::test]
    async fn test_connection_sqlite_not_supported() {
        let app = setup_router();
        let body = serde_json::json!({
            "kind": "sqlite",
            "url": "sqlite:test.db"
        });

        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/setup/test-connection")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_string(&body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(json["success"], false);
        assert!(json["error"].as_str().unwrap().contains("v0.2"));
    }

    #[tokio::test]
    async fn test_connection_bad_postgres_url() {
        let app = setup_router();
        let body = serde_json::json!({
            "kind": "postgres",
            "url": "postgres://invalid:invalid@localhost:59999/nonexistent"
        });

        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/setup/test-connection")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_string(&body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(json["success"], false);
        assert!(json["error"].is_string());
    }

    #[tokio::test]
    async fn save_config_rejects_unreachable_db() {
        let app = setup_router();
        let body = serde_json::json!({
            "server": { "host": "0.0.0.0", "port": 8080 },
            "database": { "kind": "postgres", "url": "postgres://u:p@localhost:59999/db", "max_connections": 3 }
        });

        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/setup/save")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_string(&body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(json["success"], false);
        assert!(json["error"].as_str().unwrap().contains("Cannot connect"));
    }

    #[tokio::test]
    async fn save_config_invalid_kind() {
        let app = setup_router();
        let body = serde_json::json!({
            "server": { "host": "127.0.0.1", "port": 3141 },
            "database": { "kind": "mysql", "url": "mysql://localhost/db" }
        });

        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/setup/save")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_string(&body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(json["success"], false);
        assert!(json["error"].as_str().unwrap().contains("Unsupported"));
    }
}
