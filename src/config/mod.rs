mod error;
mod hostname;
mod serveraddr;

pub use hostname::Hostname;
pub use serveraddr::ServerAddr;

use error::ConfigError;
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File},
    io,
};

static CONFIG_PATH: &str = "config.yml";

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct Config {
    pub forwards: Vec<Forward>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Forward {
    pub hostname: Hostname,
    pub target: ServerAddr,
}

pub fn load() -> Result<Config, ConfigError> {
    File::open(CONFIG_PATH)
        .map_err(ConfigError::from)
        .and_then(|file| serde_yaml::from_reader(file).map_err(ConfigError::from))
        .or_else(|config| match config {
            ConfigError::IO(ref e) if e.kind() == io::ErrorKind::NotFound => {
                let config = Default::default();
                save(&config)?;

                Ok(config)
            }
            err => Err(err),
        })
}

pub fn save(config: &Config) -> Result<(), ConfigError> {
    let data = serde_yaml::to_string(config)?;

    fs::write(CONFIG_PATH, data).map_err(ConfigError::from)
}
