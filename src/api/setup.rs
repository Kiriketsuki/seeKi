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

    let toml_content = match build_config_toml(&req) {
        Ok(s) => s,
        Err(e) => {
            return Json(SaveConfigResponse {
                success: false,
                error: Some(e),
            });
        }
    };

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

/// Build a typed TOML config from the save request, serialize it, and validate
/// it round-trips through `AppConfig::parse`. Returns the TOML string on success.
fn build_config_toml(req: &SaveConfigRequest) -> Result<String, String> {
    let config_to_write = toml::value::Table::from_iter([
        (
            "server".to_string(),
            toml::Value::Table(toml::value::Table::from_iter([
                ("host".to_string(), toml::Value::String(req.server.host.clone())),
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
                    toml::Value::String(req.database.kind.clone()),
                ),
                ("url".to_string(), toml::Value::String(req.database.url.clone())),
                (
                    "max_connections".to_string(),
                    toml::Value::Integer(req.database.max_connections as i64),
                ),
            ])),
        ),
    ]);

    let toml_content = toml::to_string_pretty(&config_to_write)
        .map_err(|e| format!("Failed to serialize config: {e}"))?;

    // Validate the generated TOML parses as a valid AppConfig
    AppConfig::parse(&toml_content)
        .map_err(|e| format!("Generated config is invalid: {e}"))?;

    Ok(toml_content)
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

    #[test]
    fn build_config_toml_success_roundtrips() {
        let req = SaveConfigRequest {
            server: SaveServerConfig {
                host: "0.0.0.0".to_string(),
                port: 8080,
            },
            database: SaveDatabaseConfig {
                kind: "postgres".to_string(),
                url: "postgres://user:pass@localhost:5432/mydb".to_string(),
                max_connections: 10,
            },
        };

        let toml_content = build_config_toml(&req).expect("should produce valid TOML");

        // Verify the TOML contains the expected values
        let parsed: toml::Value = toml::from_str(&toml_content).unwrap();
        assert_eq!(parsed["server"]["host"].as_str().unwrap(), "0.0.0.0");
        assert_eq!(parsed["server"]["port"].as_integer().unwrap(), 8080);
        assert_eq!(parsed["database"]["kind"].as_str().unwrap(), "postgres");
        assert_eq!(
            parsed["database"]["url"].as_str().unwrap(),
            "postgres://user:pass@localhost:5432/mydb"
        );
        assert_eq!(parsed["database"]["max_connections"].as_integer().unwrap(), 10);

        // Verify it round-trips through AppConfig::parse (already done inside
        // build_config_toml, but confirm the output is also re-parseable)
        AppConfig::parse(&toml_content).expect("generated TOML should parse as AppConfig");
    }

    #[test]
    fn build_config_toml_writes_to_disk() {
        let req = SaveConfigRequest {
            server: SaveServerConfig {
                host: "127.0.0.1".to_string(),
                port: 3141,
            },
            database: SaveDatabaseConfig {
                kind: "postgres".to_string(),
                url: "postgres://u:p@localhost/db".to_string(),
                max_connections: 5,
            },
        };

        let toml_content = build_config_toml(&req).expect("should produce valid TOML");

        let dir = std::env::temp_dir().join("seeki-test-save-config");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("seeki.toml");
        std::fs::write(&path, &toml_content).expect("should write to temp file");

        let read_back = std::fs::read_to_string(&path).unwrap();
        assert_eq!(read_back, toml_content);
        AppConfig::parse(&read_back).expect("written file should parse as AppConfig");

        // Cleanup
        let _ = std::fs::remove_dir_all(&dir);
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
