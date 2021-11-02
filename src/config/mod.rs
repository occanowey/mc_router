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
    defaulthost: Option<Hostname>,
    pub virtualhosts: Vec<VirtualHost>,
}

impl Config {
    pub fn get_default_target(&self) -> Option<&VirtualHost> {
        self.defaulthost
            .as_ref()
            .and_then(|d| self.virtualhosts.iter().find(|f| f.hostname == &d.0))
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
// TODO: allow multiple targets and try them by priority/round robin
// e.g.
// prority:
// 1. forward
// 2. status
// try connect to farward of dispaly preset status if that fails
//
// round robin:
// 1. forward
// 2. forward
// load balance between the targets
pub struct VirtualHost {
    pub hostname: Hostname,
    pub target: HostTarget,
    // pub targets: Vec<HostTarget>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "lowercase")]
pub enum HostTarget {
    Forward(ServerAddr),

    // TODO: flesh this out, there's many more fields the status can contain (or just allow a raw json object)
    Status {
        online_players: i64,
        max_players: i64,
        description: String,
    },
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
