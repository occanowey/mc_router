mod hostname;
mod serveraddr;

pub use hostname::Hostname;
pub use serveraddr::ServerAddr;

use anyhow::Result;
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
    pub fn get_default_host(&self) -> Option<&VirtualHost> {
        self.defaulthost.as_ref().and_then(|d| {
            self.virtualhosts
                .iter()
                .find(|f| f.hostname == d.0.as_str())
        })
    }
}

#[derive(Serialize, Deserialize, Debug)]
// TODO: allow multiple targets and try them by priority/round robin
// e.g.
// prority:
// 1. forward
// 2. status
// try connect to farward of display preset status if that fails
//
// round robin:
// 1. forward
// 2. forward
// load balance between the targets
pub struct VirtualHost {
    pub hostname: Hostname,
    pub action: Action,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum Action {
    Conditional {
        status: StatusAction,
        login: LoginAction,
    },

    Static {
        r#static: StaticAction,
    },
    Forward {
        forward: ForwardAction,
    },
}

impl Action {
    pub fn get_status_action(&self) -> StatusAction {
        match self {
            Action::Conditional { status, .. } => status.clone(),

            Action::Static { r#static } => StatusAction::Static {
                r#static: r#static.clone(),
            },
            Action::Forward { forward } => StatusAction::Forward {
                forward: forward.clone(),
            },
        }
    }

    pub fn get_login_action(&self) -> LoginAction {
        match self {
            Action::Conditional { login, .. } => login.clone(),

            Action::Static { r#static } => LoginAction::Static {
                r#static: r#static.clone(),
            },
            Action::Forward { forward } => LoginAction::Forward {
                forward: forward.clone(),
            },
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum StatusAction {
    Static { r#static: StaticAction },
    Forward { forward: ForwardAction },
    Modify { modify: ModifyAction },
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum LoginAction {
    Static { r#static: StaticAction },
    Forward { forward: ForwardAction },
}

// TODO: flesh this out, there's many more fields the status can contain (or just allow a raw json object?)
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct StaticAction {
    pub version_name: Option<String>,
    pub protocol_version: Option<i32>,
    pub cur_players: Option<i64>,
    pub max_players: Option<i64>,
    pub description: Option<String>,

    pub kick_message: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ForwardAction(pub ServerAddr);

// todo big work
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ModifyAction {}

pub fn load() -> Result<Config> {
    let file = File::open(CONFIG_PATH);

    if let Ok(file) = file {
        Ok(serde_yaml::from_reader(file)?)
    } else {
        Ok(match file.unwrap_err() {
            err if err.kind() == io::ErrorKind::NotFound => {
                let config = Default::default();

                save(&config)?;
                Ok(config)
            }
            other => Err(other),
        }?)
    }
}

pub fn save(config: &Config) -> Result<()> {
    fs::write(CONFIG_PATH, serde_yaml::to_string(config)?)?;
    Ok(())
}
