use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DotLanthConfig {
    pub data_dir: PathBuf,
    pub ui: UiConfig,
    pub mock_data: MockDataConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiConfig {
    pub theme: String,
    pub refresh_rate_ms: u64,
    pub show_debug_info: bool,
    pub max_log_lines: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MockDataConfig {
    pub generate_sample_data: bool,
    pub node_count: usize,
    pub deployment_count: usize,
    pub simulate_failures: bool,
}

impl Default for DotLanthConfig {
    fn default() -> Self {
        Self {
            data_dir: dirs::data_local_dir().unwrap_or_else(|| PathBuf::from(".")).join("dotlanth"),
            ui: UiConfig {
                theme: "default".to_string(),
                refresh_rate_ms: 1000,
                show_debug_info: false,
                max_log_lines: 1000,
            },
            mock_data: MockDataConfig {
                generate_sample_data: true,
                node_count: 3,
                deployment_count: 5,
                simulate_failures: true,
            },
        }
    }
}

impl DotLanthConfig {
    pub fn load_from_file(path: impl AsRef<Path>) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Self = toml::from_str(&content)?;
        Ok(config)
    }

    pub fn save_to_file(&self, path: impl AsRef<Path>) -> Result<()> {
        let content = toml::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    pub fn resolve_config(cli_config: Option<PathBuf>, cli_data_dir: Option<PathBuf>) -> Result<Self> {
        let mut config = if let Some(config_path) = cli_config {
            Self::load_from_file(config_path)?
        } else if let Ok(env_config) = std::env::var("DOTLANTH_CONFIG") {
            Self::load_from_file(env_config)?
        } else {
            Self::default()
        };

        // CLI data_dir overrides environment settings
        if let Some(data_dir) = cli_data_dir {
            config.data_dir = data_dir;
        } else if let Ok(env_data_dir) = std::env::var("DOTLANTH_DATA_DIR") {
            config.data_dir = PathBuf::from(env_data_dir);
        }

        std::fs::create_dir_all(&config.data_dir)?;
        Ok(config)
    }
}
