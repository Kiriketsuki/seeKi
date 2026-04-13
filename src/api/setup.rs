use std::sync::Arc;

use axum::{Extension, Json, http::StatusCode, response::IntoResponse};
use serde::{Deserialize, Serialize};

use crate::AppState;
use crate::app_mode::{AppMode, SharedAppMode};
use crate::config::{AppConfig, DatabaseKind, SecretsConfig, SshAuthMethod, SshConfig};
use crate::db::{DatabasePool, postgres};

#[cfg(test)]
use crate::app_mode::initial_mode;

// ── Request / response types ─────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct TestConnectionRequest {
    kind: String,
    url: String,
    ssh: Option<SshWizardConfig>,
}

#[derive(Deserialize, Clone)]
pub struct SshWizardConfig {
    host: String,
    #[serde(default = "default_ssh_port")]
    port: u16,
    username: String,
    auth_method: String,
    key_path: Option<String>,
    key_passphrase: Option<String>,
    password: Option<String>,
}

fn default_ssh_port() -> u16 {
    22
}

#[derive(Serialize)]
struct TablePreviewDto {
    schema: String,
    name: String, // always the bare table name (never "schema.table")
    estimated_rows: i64,
    is_system: bool,
}

#[derive(Serialize)]
struct SchemaPreviewDto {
    name: String,
    table_count: i64,
}

#[derive(Serialize)]
pub struct TestConnectionResponse {
    success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    tables: Option<Vec<TablePreviewDto>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    schemas: Option<Vec<SchemaPreviewDto>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error_source: Option<String>,
}

#[derive(Deserialize)]
pub struct SaveConfigRequest {
    #[serde(default)]
    server: SaveServerConfig,
    database: SaveDatabaseConfig,
    ssh: Option<SshWizardConfig>,
    tables: Option<SaveTablesConfig>,
    branding: Option<SaveBrandingConfig>,
}

#[derive(Deserialize, Default)]
pub struct SaveServerConfig {
    #[serde(default = "default_host")]
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
}

#[derive(Deserialize)]
pub struct SaveDatabaseConfig {
    pub kind: String,
    pub url: String,
    #[serde(default = "default_max_connections")]
    pub max_connections: u32,
    #[serde(default)]
    pub schemas: Option<Vec<String>>,
}

#[derive(Deserialize)]
struct SaveTablesConfig {
    include: Option<Vec<String>>,
}

#[derive(Deserialize)]
struct SaveBrandingConfig {
    title: Option<String>,
    subtitle: Option<String>,
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
pub struct SaveConfigResponse {
    success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

// ── Handlers ─────────────────────────────────────────────────────────────────

pub async fn test_connection(
    Extension(mode): Extension<SharedAppMode>,
    Json(req): Json<TestConnectionRequest>,
) -> impl IntoResponse {
    // Guard: reject if setup has already been completed.
    if !matches!(*mode.read().await, AppMode::Setup) {
        return (
            StatusCode::CONFLICT,
            Json(TestConnectionResponse {
                success: false,
                tables: None,
                schemas: None,
                error: Some("Setup is already complete".to_string()),
                error_source: None,
            }),
        )
            .into_response();
    }
    let kind = match parse_db_kind(&req.kind) {
        Ok(k) => k,
        Err(e) => {
            return Json(TestConnectionResponse {
                success: false,
                tables: None,
                schemas: None,
                error: Some(e),
                error_source: None,
            })
            .into_response();
        }
    };

    match kind {
        DatabaseKind::Postgres => {
            let ssh_pair = req.ssh.as_ref().map(wizard_to_ssh).transpose();

            let (ssh_config, secrets) = match ssh_pair {
                Ok(Some(pair)) => (Some(pair.0), Some(pair.1)),
                Ok(None) => (None, None),
                Err(e) => {
                    return Json(TestConnectionResponse {
                        success: false,
                        tables: None,
                        schemas: None,
                        error: Some(e),
                        error_source: Some("ssh_config".to_string()),
                    })
                    .into_response();
                }
            };

            let ssh_ref = ssh_config.as_ref().zip(secrets.as_ref());

            match postgres::test_connection(&req.url, ssh_ref).await {
                Ok((tables, schemas)) => {
                    let dtos: Vec<TablePreviewDto> = tables
                        .into_iter()
                        .map(|t| TablePreviewDto {
                            schema: t.schema,
                            name: t.name,
                            estimated_rows: t.estimated_rows,
                            is_system: t.is_system,
                        })
                        .collect();
                    let schema_dtos: Vec<SchemaPreviewDto> = schemas
                        .into_iter()
                        .map(|s| SchemaPreviewDto {
                            name: s.name,
                            table_count: s.table_count,
                        })
                        .collect();
                    Json(TestConnectionResponse {
                        success: true,
                        tables: Some(dtos),
                        schemas: Some(schema_dtos),
                        error: None,
                        error_source: None,
                    })
                    .into_response()
                }
                Err(e) => {
                    tracing::error!(error = %e, "test_connection failed");
                    let error_source = if ssh_config.is_some() {
                        Some("ssh".to_string())
                    } else {
                        None
                    };
                    Json(TestConnectionResponse {
                        success: false,
                        tables: None,
                        schemas: None,
                        error: Some(
                            "Failed to connect to database. Check your connection URL and ensure the database is running.".to_string(),
                        ),
                        error_source,
                    })
                    .into_response()
                }
            }
        }
        DatabaseKind::Sqlite => Json(TestConnectionResponse {
            success: false,
            tables: None,
            schemas: None,
            error: Some("SQLite support coming in v0.2".to_string()),
            error_source: None,
        })
        .into_response(),
    }
}

pub async fn save_config(
    Extension(mode): Extension<SharedAppMode>,
    Json(req): Json<SaveConfigRequest>,
) -> impl IntoResponse {
    // Guard: reject if setup has already been completed.
    if !matches!(*mode.read().await, AppMode::Setup) {
        return (
            StatusCode::CONFLICT,
            Json(SaveConfigResponse {
                success: false,
                error: Some("Setup is already complete".to_string()),
            }),
        )
            .into_response();
    }
    // 1. Validate DB kind
    let kind = match parse_db_kind(&req.database.kind) {
        Ok(k) => k,
        Err(e) => {
            return Json(SaveConfigResponse {
                success: false,
                error: Some(e),
            })
            .into_response();
        }
    };

    // 2. Convert SSH wizard config
    let (ssh_config, secrets) = match req.ssh.as_ref().map(wizard_to_ssh).transpose() {
        Ok(Some(pair)) => (Some(pair.0), pair.1),
        Ok(None) => (None, SecretsConfig::default()),
        Err(e) => {
            return Json(SaveConfigResponse {
                success: false,
                error: Some(e),
            })
            .into_response();
        }
    };

    // 3. Test connection before writing anything
    match kind {
        DatabaseKind::Postgres => {
            let ssh_ref = ssh_config.as_ref().map(|s| (s, &secrets));
            if let Err(e) = postgres::test_connection(&req.database.url, ssh_ref).await {
                tracing::error!(error = %e, "save_config: connection test failed");
                return Json(SaveConfigResponse {
                    success: false,
                    error: Some(
                        "Cannot connect to the database. Check your connection URL and ensure the database is running.".to_string(),
                    ),
                })
                .into_response();
            }
        }
        DatabaseKind::Sqlite => {
            return Json(SaveConfigResponse {
                success: false,
                error: Some("SQLite support coming in v0.2".to_string()),
            })
            .into_response();
        }
    }

    // 4. Build TOML content
    let toml_content = match build_config_toml(&req, &ssh_config) {
        Ok(s) => s,
        Err(e) => {
            return Json(SaveConfigResponse {
                success: false,
                error: Some(e),
            })
            .into_response();
        }
    };

    // 5. Write .seeki.secrets if any secrets present
    let secrets_content = build_secrets_toml(&secrets);

    // 6. Write seeki.toml atomically (tmp → rename)
    let tmp_path = "seeki.toml.tmp";
    if let Err(e) = std::fs::write(tmp_path, &toml_content) {
        return Json(SaveConfigResponse {
            success: false,
            error: Some(format!("Failed to write seeki.toml.tmp: {e}")),
        })
        .into_response();
    }
    if let Err(e) = std::fs::rename(tmp_path, "seeki.toml") {
        let _ = std::fs::remove_file(tmp_path);
        return Json(SaveConfigResponse {
            success: false,
            error: Some(format!("Failed to rename seeki.toml.tmp → seeki.toml: {e}")),
        })
        .into_response();
    }

    // 7. Write .seeki.secrets if needed
    if let Some(secrets_str) = secrets_content
        && let Err(e) = write_secrets_file(&secrets_str)
    {
        let _ = std::fs::remove_file(".seeki.secrets");
        let _ = std::fs::remove_file("seeki.toml");
        return Json(SaveConfigResponse {
            success: false,
            error: Some(format!("Failed to write .seeki.secrets: {e}")),
        })
        .into_response();
    }

    // 8. Connect DatabasePool and build AppState
    let app_config = match AppConfig::parse(&toml_content) {
        Ok(c) => c,
        Err(e) => {
            let _ = std::fs::remove_file("seeki.toml");
            let _ = std::fs::remove_file(".seeki.secrets");
            return Json(SaveConfigResponse {
                success: false,
                error: Some(format!("Generated config is invalid: {e}")),
            })
            .into_response();
        }
    };

    let ssh_ref = app_config.ssh.as_ref().map(|s| (s, &secrets));
    let db = match DatabasePool::connect(&app_config.database, ssh_ref).await {
        Ok(d) => d,
        Err(e) => {
            let _ = std::fs::remove_file("seeki.toml");
            let _ = std::fs::remove_file(".seeki.secrets");
            return Json(SaveConfigResponse {
                success: false,
                error: Some(format!("Failed to connect after saving config: {e}")),
            })
            .into_response();
        }
    };

    // 9. Swap mode to Normal
    let new_state = Arc::new(AppState {
        db,
        config: app_config,
    });
    *mode.write().await = AppMode::Normal(new_state);

    Json(SaveConfigResponse {
        success: true,
        error: None,
    })
    .into_response()
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn parse_db_kind(kind: &str) -> Result<DatabaseKind, String> {
    match kind {
        "postgres" => Ok(DatabaseKind::Postgres),
        "sqlite" => Ok(DatabaseKind::Sqlite),
        other => Err(format!("Unsupported database kind: {other}")),
    }
}

fn wizard_to_ssh(w: &SshWizardConfig) -> Result<(SshConfig, SecretsConfig), String> {
    let auth_method = match w.auth_method.as_str() {
        "key" => SshAuthMethod::Key,
        "agent" => SshAuthMethod::Agent,
        "password" => SshAuthMethod::Password,
        other => return Err(format!("Unknown SSH auth method: {other}")),
    };
    let ssh_config = SshConfig {
        host: w.host.clone(),
        port: w.port,
        username: w.username.clone(),
        auth_method,
        key_path: w.key_path.clone(),
    };
    let secrets = SecretsConfig {
        ssh_key_passphrase: w.key_passphrase.clone(),
        ssh_password: w.password.clone(),
    };
    Ok((ssh_config, secrets))
}

/// Build a TOML config string from the save request. Validates it round-trips as AppConfig.
pub fn build_config_toml(
    req: &SaveConfigRequest,
    ssh_config: &Option<SshConfig>,
) -> Result<String, String> {
    let mut root = toml::value::Table::new();

    root.insert(
        "server".to_string(),
        toml::Value::Table(toml::value::Table::from_iter([
            (
                "host".to_string(),
                toml::Value::String(req.server.host.clone()),
            ),
            (
                "port".to_string(),
                toml::Value::Integer(req.server.port as i64),
            ),
        ])),
    );

    let mut db_table = toml::value::Table::from_iter([
        (
            "kind".to_string(),
            toml::Value::String(req.database.kind.clone()),
        ),
        (
            "url".to_string(),
            toml::Value::String(req.database.url.clone()),
        ),
        (
            "max_connections".to_string(),
            toml::Value::Integer(req.database.max_connections as i64),
        ),
    ]);
    if let Some(schemas) = &req.database.schemas {
        if schemas.is_empty() {
            return Err(
                "database.schemas must not be empty — omit the field to default to [\"public\"]"
                    .to_string(),
            );
        }
        for s in schemas {
            if !postgres::is_valid_identifier(s) {
                return Err(format!("Invalid schema name: {s}"));
            }
        }
        db_table.insert(
            "schemas".to_string(),
            toml::Value::Array(
                schemas
                    .iter()
                    .map(|s| toml::Value::String(s.clone()))
                    .collect(),
            ),
        );
    }
    root.insert("database".to_string(), toml::Value::Table(db_table));

    if let Some(tables_cfg) = &req.tables
        && let Some(include) = &tables_cfg.include
    {
        let arr = toml::Value::Array(
            include
                .iter()
                .map(|s| toml::Value::String(s.clone()))
                .collect(),
        );
        root.insert(
            "tables".to_string(),
            toml::Value::Table(toml::value::Table::from_iter([(
                "include".to_string(),
                arr,
            )])),
        );
    }

    if let Some(branding) = &req.branding {
        let mut b = toml::value::Table::new();
        if let Some(title) = &branding.title {
            b.insert("title".to_string(), toml::Value::String(title.clone()));
        }
        if let Some(subtitle) = &branding.subtitle {
            b.insert(
                "subtitle".to_string(),
                toml::Value::String(subtitle.clone()),
            );
        }
        if !b.is_empty() {
            root.insert("branding".to_string(), toml::Value::Table(b));
        }
    }

    if let Some(ssh) = ssh_config {
        let mut s = toml::value::Table::new();
        s.insert("host".to_string(), toml::Value::String(ssh.host.clone()));
        s.insert("port".to_string(), toml::Value::Integer(ssh.port as i64));
        s.insert(
            "username".to_string(),
            toml::Value::String(ssh.username.clone()),
        );
        s.insert(
            "auth_method".to_string(),
            toml::Value::String(
                match ssh.auth_method {
                    SshAuthMethod::Key => "key",
                    SshAuthMethod::Password => "password",
                    SshAuthMethod::Agent => "agent",
                }
                .to_string(),
            ),
        );
        if let Some(key_path) = &ssh.key_path {
            s.insert(
                "key_path".to_string(),
                toml::Value::String(key_path.clone()),
            );
        }
        root.insert("ssh".to_string(), toml::Value::Table(s));
    }

    let toml_content =
        toml::to_string_pretty(&root).map_err(|e| format!("Failed to serialize config: {e}"))?;

    AppConfig::parse(&toml_content).map_err(|e| format!("Generated config is invalid: {e}"))?;

    Ok(toml_content)
}

fn build_secrets_toml(secrets: &SecretsConfig) -> Option<String> {
    let has_passphrase = secrets.ssh_key_passphrase.is_some();
    let has_password = secrets.ssh_password.is_some();
    if !has_passphrase && !has_password {
        return None;
    }
    let mut ssh_section = toml::value::Table::new();
    if let Some(p) = &secrets.ssh_key_passphrase {
        ssh_section.insert("key_passphrase".to_string(), toml::Value::String(p.clone()));
    }
    if let Some(p) = &secrets.ssh_password {
        ssh_section.insert("password".to_string(), toml::Value::String(p.clone()));
    }
    let mut root = toml::value::Table::new();
    root.insert("ssh".to_string(), toml::Value::Table(ssh_section));
    toml::to_string_pretty(&root).ok()
}

fn write_secrets_file(content: &str) -> std::io::Result<()> {
    std::fs::write(".seeki.secrets", content)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(".seeki.secrets", std::fs::Permissions::from_mode(0o600))?;
    }
    Ok(())
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{Router, body::Body, http::Request, routing::post};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    fn test_router() -> Router {
        let mode: SharedAppMode = initial_mode(None);
        Router::new()
            .route("/setup/test-connection", post(test_connection))
            .route("/setup/save", post(save_config))
            .layer(Extension(mode))
    }

    fn test_router_with_mode(mode: SharedAppMode) -> Router {
        Router::new()
            .route("/setup/test-connection", post(test_connection))
            .route("/setup/save", post(save_config))
            .layer(Extension(mode))
    }

    #[tokio::test]
    async fn test_connection_invalid_kind() {
        let app = test_router();
        let body = serde_json::json!({ "kind": "mysql", "url": "mysql://localhost/db" });

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
        let app = test_router();
        let body = serde_json::json!({ "kind": "sqlite", "url": "sqlite:test.db" });

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
        let app = test_router();
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
    async fn test_connection_with_ssh_config_returns_error_source() {
        let app = test_router();
        let body = serde_json::json!({
            "kind": "postgres",
            "url": "postgres://user:pass@localhost:5432/db",
            "ssh": {
                "host": "bastion.example.com",
                "username": "admin",
                "auth_method": "key",
                "key_path": "/nonexistent/key"
            }
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
        // error_source should indicate ssh
        assert_eq!(json["error_source"].as_str(), Some("ssh"));
    }

    #[tokio::test]
    async fn save_config_rejects_unreachable_db() {
        let app = test_router();
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
                schemas: None,
            },
            ssh: None,
            tables: None,
            branding: None,
        };

        let toml_content = build_config_toml(&req, &None).expect("should produce valid TOML");

        let parsed: toml::Value = toml::from_str(&toml_content).unwrap();
        assert_eq!(parsed["server"]["host"].as_str().unwrap(), "0.0.0.0");
        assert_eq!(parsed["server"]["port"].as_integer().unwrap(), 8080);
        assert_eq!(parsed["database"]["kind"].as_str().unwrap(), "postgres");
        assert_eq!(
            parsed["database"]["url"].as_str().unwrap(),
            "postgres://user:pass@localhost:5432/mydb"
        );
        assert_eq!(
            parsed["database"]["max_connections"].as_integer().unwrap(),
            10
        );
        AppConfig::parse(&toml_content).expect("generated TOML should parse as AppConfig");
    }

    #[test]
    fn build_config_toml_persists_schemas_field() {
        let req = SaveConfigRequest {
            server: SaveServerConfig {
                host: "127.0.0.1".to_string(),
                port: 3141,
            },
            database: SaveDatabaseConfig {
                kind: "postgres".to_string(),
                url: "postgres://u:p@localhost/db".to_string(),
                max_connections: 5,
                schemas: Some(vec!["public".to_string(), "reporting".to_string()]),
            },
            ssh: None,
            tables: None,
            branding: None,
        };

        let toml_content = build_config_toml(&req, &None).expect("should produce TOML");

        // Raw TOML contains the schemas array in [database]
        let parsed: toml::Value = toml::from_str(&toml_content).unwrap();
        let schemas = parsed["database"]["schemas"].as_array().unwrap();
        assert_eq!(schemas.len(), 2);
        assert_eq!(schemas[0].as_str().unwrap(), "public");
        assert_eq!(schemas[1].as_str().unwrap(), "reporting");

        // Round-trip: AppConfig::parse accepts it and effective_schemas reflects the selection
        let app_config = AppConfig::parse(&toml_content).expect("parsed AppConfig");
        assert_eq!(
            app_config.database.effective_schemas(),
            vec!["public".to_string(), "reporting".to_string()]
        );
    }

    #[test]
    fn build_config_toml_rejects_empty_schemas_list() {
        let req = SaveConfigRequest {
            server: SaveServerConfig {
                host: "127.0.0.1".to_string(),
                port: 3141,
            },
            database: SaveDatabaseConfig {
                kind: "postgres".to_string(),
                url: "postgres://u:p@localhost/db".to_string(),
                max_connections: 5,
                schemas: Some(vec![]),
            },
            ssh: None,
            tables: None,
            branding: None,
        };

        let err = build_config_toml(&req, &None).expect_err("empty schemas rejected");
        assert!(err.contains("must not be empty"), "unexpected: {err}");
    }

    #[test]
    fn build_config_toml_rejects_invalid_schema_name() {
        let req = SaveConfigRequest {
            server: SaveServerConfig {
                host: "127.0.0.1".to_string(),
                port: 3141,
            },
            database: SaveDatabaseConfig {
                kind: "postgres".to_string(),
                url: "postgres://u:p@localhost/db".to_string(),
                max_connections: 5,
                schemas: Some(vec!["bad;name".to_string()]),
            },
            ssh: None,
            tables: None,
            branding: None,
        };

        let err = build_config_toml(&req, &None).expect_err("invalid schema name rejected");
        assert!(err.contains("Invalid schema name"), "unexpected: {err}");
    }

    #[test]
    fn build_config_toml_without_schemas_defaults_to_public_via_effective() {
        let req = SaveConfigRequest {
            server: SaveServerConfig {
                host: "127.0.0.1".to_string(),
                port: 3141,
            },
            database: SaveDatabaseConfig {
                kind: "postgres".to_string(),
                url: "postgres://u:p@localhost/db".to_string(),
                max_connections: 5,
                schemas: None,
            },
            ssh: None,
            tables: None,
            branding: None,
        };

        let toml_content = build_config_toml(&req, &None).expect("should produce TOML");
        let app_config = AppConfig::parse(&toml_content).expect("parsed AppConfig");
        assert!(app_config.database.schemas.is_none());
        assert_eq!(
            app_config.database.effective_schemas(),
            vec!["public".to_string()]
        );
    }

    #[test]
    fn build_config_toml_with_tables_include() {
        let req = SaveConfigRequest {
            server: SaveServerConfig {
                host: "127.0.0.1".to_string(),
                port: 3141,
            },
            database: SaveDatabaseConfig {
                kind: "postgres".to_string(),
                url: "postgres://u:p@localhost/db".to_string(),
                max_connections: 5,
                schemas: None,
            },
            ssh: None,
            tables: Some(SaveTablesConfig {
                include: Some(vec!["users".to_string(), "orders".to_string()]),
            }),
            branding: None,
        };

        let toml_content = build_config_toml(&req, &None).expect("should produce TOML");
        let parsed: toml::Value = toml::from_str(&toml_content).unwrap();
        let include = parsed["tables"]["include"].as_array().unwrap();
        assert_eq!(include.len(), 2);
        assert_eq!(include[0].as_str().unwrap(), "users");
        assert_eq!(include[1].as_str().unwrap(), "orders");
    }

    #[test]
    fn build_config_toml_with_branding() {
        let req = SaveConfigRequest {
            server: SaveServerConfig {
                host: "127.0.0.1".to_string(),
                port: 3141,
            },
            database: SaveDatabaseConfig {
                kind: "postgres".to_string(),
                url: "postgres://u:p@localhost/db".to_string(),
                max_connections: 5,
                schemas: None,
            },
            ssh: None,
            tables: None,
            branding: Some(SaveBrandingConfig {
                title: Some("MyApp".to_string()),
                subtitle: Some("Dashboard".to_string()),
            }),
        };

        let toml_content = build_config_toml(&req, &None).expect("should produce TOML");
        let parsed: toml::Value = toml::from_str(&toml_content).unwrap();
        assert_eq!(parsed["branding"]["title"].as_str().unwrap(), "MyApp");
        assert_eq!(
            parsed["branding"]["subtitle"].as_str().unwrap(),
            "Dashboard"
        );
    }

    #[test]
    fn build_config_toml_with_ssh_no_secrets() {
        let ssh_config = SshConfig {
            host: "bastion.example.com".to_string(),
            port: 22,
            username: "admin".to_string(),
            auth_method: SshAuthMethod::Key,
            key_path: Some("/home/user/.ssh/id_rsa".to_string()),
        };

        let req = SaveConfigRequest {
            server: SaveServerConfig {
                host: "127.0.0.1".to_string(),
                port: 3141,
            },
            database: SaveDatabaseConfig {
                kind: "postgres".to_string(),
                url: "postgres://u:p@10.0.0.1/db".to_string(),
                max_connections: 5,
                schemas: None,
            },
            ssh: None,
            tables: None,
            branding: None,
        };

        let toml_content = build_config_toml(&req, &Some(ssh_config)).expect("should produce TOML");
        let parsed: toml::Value = toml::from_str(&toml_content).unwrap();
        assert_eq!(
            parsed["ssh"]["host"].as_str().unwrap(),
            "bastion.example.com"
        );
        assert_eq!(parsed["ssh"]["auth_method"].as_str().unwrap(), "key");
        // Secrets must NOT appear in seeki.toml
        assert!(parsed["ssh"].get("key_passphrase").is_none());
    }

    #[tokio::test]
    async fn save_config_invalid_kind() {
        let app = test_router();
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

    #[tokio::test]
    async fn save_config_accepts_request_without_server_field() {
        // Regression test: SaveConfigRequest.server has #[serde(default)] so omitting the
        // `server` key entirely must NOT cause a 422 deserialization failure.
        // The handler will still error (unsupported DB kind), but that proves serde succeeded.
        let app = test_router();
        let body = serde_json::json!({
            "database": { "kind": "mysql", "url": "mysql://localhost/db" }
            // `server` key intentionally absent
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

        // 200 with {success:false} means deserialization succeeded and the handler ran.
        // A 422 would mean serde rejected the payload due to the missing `server` field.
        assert_eq!(
            resp.status(),
            200,
            "should not be a 422 deserialization error"
        );
        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(json["success"], false);
        assert!(
            json["error"].as_str().unwrap().contains("Unsupported"),
            "expected Unsupported kind error, got: {:?}",
            json["error"]
        );
    }

    fn normal_mode() -> SharedAppMode {
        let pool = sqlx::PgPool::connect_lazy("postgres://test:test@localhost/test").unwrap();
        let db = crate::db::DatabasePool::Postgres(pool, None);
        let config = AppConfig::parse(
            "[server]\nhost = \"127.0.0.1\"\nport = 3141\n\
             [database]\nkind = \"postgres\"\nurl = \"postgres://u:p@localhost/db\"\n",
        )
        .expect("minimal config should parse");
        let state = Arc::new(crate::AppState { db, config });
        Arc::new(tokio::sync::RwLock::new(AppMode::Normal(state)))
    }

    #[tokio::test]
    async fn test_connection_rejects_when_not_in_setup_mode() {
        let app = test_router_with_mode(normal_mode());
        let body = serde_json::json!({ "kind": "postgres", "url": "postgres://u:p@localhost/db" });

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

        assert_eq!(resp.status(), 409);
        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(json["success"], false);
        assert!(json["error"].as_str().unwrap().contains("already complete"));
    }

    #[tokio::test]
    async fn save_config_rejects_when_not_in_setup_mode() {
        let app = test_router_with_mode(normal_mode());
        let body = serde_json::json!({
            "server": { "host": "127.0.0.1", "port": 3141 },
            "database": { "kind": "postgres", "url": "postgres://u:p@localhost/db", "max_connections": 5 }
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

        assert_eq!(resp.status(), 409);
        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(json["success"], false);
        assert!(json["error"].as_str().unwrap().contains("already complete"));
    }
}
