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
            Err(e) => Json(TestConnectionResponse {
                success: false,
                tables: None,
                error: Some(e.to_string()),
            }),
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
    if let Err(e) = parse_db_kind(&req.database.kind) {
        return Json(SaveConfigResponse {
            success: false,
            error: Some(e),
        });
    }

    let toml_content = format!(
        r#"[server]
host = "{host}"
port = {port}

[database]
kind = "{kind}"
url = "{url}"
max_connections = {max_connections}
"#,
        host = req.server.host,
        port = req.server.port,
        kind = req.database.kind,
        url = req.database.url,
        max_connections = req.database.max_connections,
    );

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

    /// Global lock to prevent CWD races between tests that change the working directory.
    fn cwd_lock() -> &'static std::sync::Mutex<()> {
        static LOCK: std::sync::OnceLock<std::sync::Mutex<()>> = std::sync::OnceLock::new();
        LOCK.get_or_init(|| std::sync::Mutex::new(()))
    }

    #[tokio::test]
    async fn save_config_writes_valid_toml() {
        let _guard = cwd_lock().lock().expect("cwd lock should not be poisoned");

        let temp_dir = std::env::temp_dir().join(format!(
            "seeki-setup-test-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        let app = setup_router();
        let body = serde_json::json!({
            "server": { "host": "0.0.0.0", "port": 8080 },
            "database": { "kind": "postgres", "url": "postgres://u:p@localhost/db", "max_connections": 3 }
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
        assert_eq!(json["success"], true);

        // Verify the written file parses as valid config
        let content = std::fs::read_to_string(temp_dir.join("seeki.toml")).unwrap();
        let config = AppConfig::parse(&content).unwrap();
        assert_eq!(config.server.host, "0.0.0.0");
        assert_eq!(config.server.port, 8080);
        assert_eq!(config.database.url, "postgres://u:p@localhost/db");
        assert_eq!(config.database.max_connections, 3);

        // Cleanup
        std::env::set_current_dir(&original_dir).unwrap();
        std::fs::remove_dir_all(&temp_dir).unwrap();
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
