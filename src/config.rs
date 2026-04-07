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

    normalized
        .split('_')
        .filter(|segment| !segment.is_empty())
        .map(title_case)
        .collect::<Vec<_>>()
        .join(" ")
}

fn title_case(segment: &str) -> String {
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

impl AppConfig {
    pub fn load() -> anyhow::Result<Self> {
        let config_path = Self::find_config_file()?;
        let content = std::fs::read_to_string(&config_path)?;
        let config = Self::parse(&content)?;
        tracing::info!("Loaded config from {}", config_path.display());
        Ok(config)
    }

    fn parse(content: &str) -> anyhow::Result<Self> {
        Ok(toml::from_str(content)?)
    }

    fn find_config_file() -> anyhow::Result<PathBuf> {
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

        anyhow::bail!(
            "No config file found. Create seeki.toml in the current directory.\n\
             Example:\n\n\
             [server]\n\
             host = \"127.0.0.1\"\n\
             port = 3141\n\n\
             [database]\n\
             kind = \"postgres\"\n\
             url = \"postgres://user:pass@localhost:5432/mydb\"\n"
        )
    }
}
