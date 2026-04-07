use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
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

impl AppConfig {
    pub fn load() -> anyhow::Result<Self> {
        let config_path = Self::find_config_file()?;
        let content = std::fs::read_to_string(&config_path)?;
        let config: Self = toml::from_str(&content)?;
        tracing::info!("Loaded config from {}", config_path.display());
        Ok(config)
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
