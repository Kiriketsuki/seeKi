use serde::Deserialize;
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    #[serde(default)]
    pub tables: TablesConfig,
    #[serde(default)]
    pub display: DisplayConfig,
    #[serde(default)]
    pub branding: BrandingConfig,
}

#[derive(Debug, Deserialize, Default)]
pub struct TablesConfig {
    #[serde(default)]
    pub include: Option<Vec<String>>,
    #[serde(default)]
    pub exclude: Option<Vec<String>>,
}

impl TablesConfig {
    pub fn allows(&self, table: &str) -> bool {
        let included = match &self.include {
            Some(include) => include.iter().any(|candidate| candidate == table),
            None => true,
        };
        let excluded = match &self.exclude {
            Some(exclude) => exclude.iter().any(|candidate| candidate == table),
            None => false,
        };

        included && !excluded
    }

    pub fn warn_overlaps(&self) {
        if let (Some(include), Some(exclude)) = (&self.include, &self.exclude) {
            for table in include {
                if exclude.contains(table) {
                    tracing::warn!(
                        table = %table,
                        "table appears in both [tables] include and exclude — it will be excluded"
                    );
                }
            }
        }
    }
}

#[derive(Debug, Deserialize, Default)]
pub struct DisplayConfig {
    #[serde(default)]
    pub tables: HashMap<String, String>,
    #[serde(default)]
    pub columns: HashMap<String, HashMap<String, String>>,
}

#[derive(Debug, Deserialize, Default)]
pub struct BrandingConfig {
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub subtitle: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ServerConfig {
    #[serde(default = "default_host")]
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
}

#[derive(Debug, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
    #[serde(default)]
    pub kind: DatabaseKind,
    #[serde(default = "default_max_connections")]
    pub max_connections: u32,
}

#[derive(Debug, Deserialize, Default, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum DatabaseKind {
    #[default]
    Postgres,
    Sqlite,
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

pub fn display_name_table(table: &str, config: &DisplayConfig) -> String {
    config
        .tables
        .get(table)
        .cloned()
        .unwrap_or_else(|| casualify(table, false))
}

pub fn display_name_column(table: &str, column: &str, config: &DisplayConfig) -> String {
    config
        .columns
        .get(table)
        .and_then(|columns| columns.get(column))
        .cloned()
        .unwrap_or_else(|| casualify(column, true))
}

fn casualify(name: &str, drop_id_suffix: bool) -> String {
    let normalized = if drop_id_suffix {
        name.strip_suffix("_id").unwrap_or(name)
    } else {
        name
    };

    let result: String = normalized
        .split('_')
        .filter(|segment| !segment.is_empty())
        .map(title_case)
        .collect::<Vec<_>>()
        .join(" ");

    if result.is_empty() {
        name.to_string()
    } else {
        result
    }
}

fn title_case(segment: &str) -> String {
    if segment.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit()) && segment.len() > 1 {
        return segment.to_string();
    }
    let mut chars = segment.chars();
    match chars.next() {
        Some(first) => format!(
            "{}{}",
            first.to_uppercase(),
            chars.as_str().to_ascii_lowercase()
        ),
        None => String::new(),
    }
}

/// Error type distinguishing "no config file" from "config file exists but is invalid".
#[derive(Debug)]
pub enum ConfigLoadError {
    /// No config file found at any candidate path — safe to enter setup mode.
    NotFound,
    /// Config file exists but failed to read or parse — should NOT enter setup mode.
    Invalid { path: PathBuf, source: anyhow::Error },
}

impl std::fmt::Display for ConfigLoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound => write!(f, "No config file found"),
            Self::Invalid { path, source } => {
                write!(f, "Invalid config at {}: {source}", path.display())
            }
        }
    }
}

impl std::error::Error for ConfigLoadError {}

impl AppConfig {
    pub fn load() -> Result<Self, ConfigLoadError> {
        let config_path = Self::find_config_file()?;
        let content = std::fs::read_to_string(&config_path).map_err(|e| {
            ConfigLoadError::Invalid {
                path: config_path.clone(),
                source: e.into(),
            }
        })?;
        let config = Self::parse(&content).map_err(|e| ConfigLoadError::Invalid {
            path: config_path.clone(),
            source: e,
        })?;
        tracing::info!("Loaded config from {}", config_path.display());
        Ok(config)
    }

    pub fn parse(content: &str) -> anyhow::Result<Self> {
        Ok(toml::from_str(content)?)
    }

    fn find_config_file() -> Result<PathBuf, ConfigLoadError> {
        let candidates = [
            PathBuf::from("seeki.toml"),
            dirs_next::config_dir()
                .map(|d| d.join("seeki").join("config.toml"))
                .unwrap_or_default(),
        ];

        for path in &candidates {
            if path.exists() {
                return Ok(path.clone());
            }
        }

        Err(ConfigLoadError::NotFound)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    const MINIMAL_CONFIG: &str = r#"
[server]
host = "127.0.0.1"
port = 3141

[database]
kind = "postgres"
url = "postgres://user:pass@localhost:5432/mydb"
"#;

    const FULL_CONFIG: &str = r#"
[server]
host = "0.0.0.0"
port = 4000

[database]
kind = "sqlite"
url = "sqlite:seeki.db"
max_connections = 10

[tables]
include = ["vehicles_log", "drivers", "audit_log"]
exclude = ["audit_log"]

[display.tables]
vehicles_log = "Fleet Log"

[display.columns.vehicles_log]
posn_lat = "Latitude"
supervisor_id = "Supervisor"

[branding]
title = "AutoConnect"
subtitle = "Fleet Telemetry"
"#;

    #[test]
    fn minimal_config_loads_with_defaults() {
        let config = load_config(MINIMAL_CONFIG);

        assert!(config.tables.include.is_none());
        assert!(config.tables.exclude.is_none());
        assert!(config.display.tables.is_empty());
        assert!(config.display.columns.is_empty());
        assert!(config.branding.title.is_none());
        assert!(config.branding.subtitle.is_none());
    }

    #[test]
    fn full_config_loads_all_extensions() {
        let config = load_config(FULL_CONFIG);

        assert_eq!(
            config
                .tables
                .include
                .as_ref()
                .expect("include should be set"),
            &vec![
                "vehicles_log".to_string(),
                "drivers".to_string(),
                "audit_log".to_string(),
            ]
        );
        assert_eq!(
            config
                .tables
                .exclude
                .as_ref()
                .expect("exclude should be set"),
            &vec!["audit_log".to_string()]
        );
        assert_eq!(
            config
                .display
                .tables
                .get("vehicles_log")
                .map(String::as_str),
            Some("Fleet Log")
        );
        assert_eq!(
            config
                .display
                .columns
                .get("vehicles_log")
                .and_then(|columns| columns.get("posn_lat"))
                .map(String::as_str),
            Some("Latitude")
        );
        assert_eq!(config.branding.title.as_deref(), Some("AutoConnect"));
        assert_eq!(config.branding.subtitle.as_deref(), Some("Fleet Telemetry"));
    }

    #[test]
    fn tables_config_applies_include_then_exclude() {
        let config = TablesConfig {
            include: Some(vec!["a".into(), "b".into(), "c".into()]),
            exclude: Some(vec!["c".into()]),
        };

        assert_eq!(
            effective_tables(&config, &["a", "b", "c", "d"]),
            vec!["a", "b"]
        );
    }

    #[test]
    fn tables_config_applies_include_only() {
        let config = TablesConfig {
            include: Some(vec!["a".into(), "b".into()]),
            exclude: None,
        };

        assert_eq!(
            effective_tables(&config, &["a", "b", "c", "d"]),
            vec!["a", "b"]
        );
    }

    #[test]
    fn tables_config_applies_exclude_only() {
        let config = TablesConfig {
            include: None,
            exclude: Some(vec!["c".into()]),
        };

        assert_eq!(
            effective_tables(&config, &["a", "b", "c", "d"]),
            vec!["a", "b", "d"]
        );
    }

    #[test]
    fn tables_config_allows_all_tables_when_unset() {
        let config = TablesConfig::default();

        assert_eq!(
            effective_tables(&config, &["a", "b", "c"]),
            vec!["a", "b", "c"]
        );
    }

    #[test]
    fn column_display_name_uses_title_case_heuristic() {
        assert_eq!(
            display_name_column("my_table", "some_column", &DisplayConfig::default()),
            "Some Column"
        );
    }

    #[test]
    fn column_display_name_drops_id_suffix() {
        assert_eq!(
            display_name_column("vehicles_log", "supervisor_id", &DisplayConfig::default()),
            "Supervisor"
        );
    }

    #[test]
    fn column_display_name_prefers_override() {
        let config = AppConfig::parse(FULL_CONFIG).expect("full config should parse");

        assert_eq!(
            display_name_column("vehicles_log", "posn_lat", &config.display),
            "Latitude"
        );
    }

    #[test]
    fn table_display_name_uses_title_case_heuristic() {
        assert_eq!(
            display_name_table("vehicles_log", &DisplayConfig::default()),
            "Vehicles Log"
        );
    }

    #[test]
    fn table_display_name_prefers_override() {
        let config = AppConfig::parse(FULL_CONFIG).expect("full config should parse");

        assert_eq!(
            display_name_table("vehicles_log", &config.display),
            "Fleet Log"
        );
    }

    #[test]
    fn example_config_parses() {
        let config =
            AppConfig::parse(include_str!("../seeki.toml.example")).expect("example config parses");

        assert!(config.tables.include.is_some());
        assert!(!config.display.tables.is_empty());
        assert!(!config.display.columns.is_empty());
        assert_eq!(config.branding.title.as_deref(), Some("AutoConnect"));
        assert_eq!(config.branding.subtitle.as_deref(), Some("Fleet Telemetry"));
        assert_eq!(
            config
                .display
                .columns
                .get("vehicles_log")
                .and_then(|c| c.get("posn_lat"))
                .map(String::as_str),
            Some("Latitude")
        );
    }

    #[test]
    fn casualify_preserves_all_caps_segments() {
        assert_eq!(
            display_name_column("t", "GPS_LATITUDE", &DisplayConfig::default()),
            "GPS LATITUDE"
        );
    }

    #[test]
    fn casualify_preserves_mixed_caps_segments() {
        assert_eq!(
            display_name_table("HTTP_STATUS", &DisplayConfig::default()),
            "HTTP STATUS"
        );
    }

    #[test]
    fn casualify_handles_id_only_column() {
        // "_id" with drop_id_suffix strips to "" — fallback returns raw name
        assert_eq!(
            display_name_column("t", "_id", &DisplayConfig::default()),
            "_id"
        );
    }

    #[test]
    fn casualify_handles_bare_id_column() {
        assert_eq!(
            display_name_column("t", "id", &DisplayConfig::default()),
            "Id"
        );
    }

    #[test]
    fn casualify_handles_empty_string() {
        assert_eq!(
            display_name_column("t", "", &DisplayConfig::default()),
            ""
        );
    }

    #[test]
    fn casualify_handles_numbers_in_name() {
        assert_eq!(
            display_name_table("vehicle_v2_data", &DisplayConfig::default()),
            "Vehicle V2 Data"
        );
    }

    fn effective_tables<'a>(config: &TablesConfig, tables: &'a [&'a str]) -> Vec<&'a str> {
        tables
            .iter()
            .copied()
            .filter(|table| config.allows(table))
            .collect()
    }

    fn load_config(content: &str) -> AppConfig {
        let _guard = crate::testutil::cwd_lock()
            .lock()
            .expect("cwd lock should not be poisoned");
        let _temp_dir = TempConfigDir::new(content);

        match AppConfig::load() {
            Ok(config) => config,
            Err(e) => panic!("config should load: {e}"),
        }
    }

    struct TempConfigDir {
        original_dir: PathBuf,
        temp_dir: PathBuf,
    }

    impl TempConfigDir {
        fn new(content: &str) -> Self {
            let original_dir = std::env::current_dir().expect("current dir should exist");
            let temp_dir = std::env::temp_dir().join(format!(
                "seeki-config-test-{}-{}",
                std::process::id(),
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .expect("system time should be after epoch")
                    .as_nanos()
            ));

            fs::create_dir_all(&temp_dir).expect("temp dir should be created");
            fs::write(temp_dir.join("seeki.toml"), content).expect("temp config should be written");
            std::env::set_current_dir(&temp_dir).expect("cwd should switch to temp dir");

            Self {
                original_dir,
                temp_dir,
            }
        }
    }

    impl Drop for TempConfigDir {
        fn drop(&mut self) {
            std::env::set_current_dir(&self.original_dir).expect("cwd should be restored");
            fs::remove_dir_all(&self.temp_dir).expect("temp dir should be removed");
        }
    }
}
