pub mod backup;
pub mod cluster;
pub mod config;
pub mod deploy;
pub mod monitor;
pub mod nodes;

use crate::config::DotLanthConfig;
use crate::database::DotLanthDatabase;
use anyhow::Result;

pub struct CommandContext {
    pub config: DotLanthConfig,
    pub database: DotLanthDatabase,
}

impl CommandContext {
    pub fn new(config: DotLanthConfig) -> Result<Self> {
        let database = DotLanthDatabase::new(&config.data_dir.join("mock_db"))?;
        if config.mock_data.generate_sample_data {
            database.generate_sample_data()?;
        }
        Ok(Self { config, database })
    }
}
