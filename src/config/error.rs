use std::{error::Error, fmt, io};

#[derive(Debug)]
pub enum ConfigError {
    IO(io::Error),
    Yaml(serde_yaml::Error),
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ConfigError - ")?;

        match self {
            ConfigError::IO(err) => write!(f, "IO: {:?}", err),
            ConfigError::Yaml(err) => write!(f, "Yaml: {:?}", err),
        }
    }
}

impl Error for ConfigError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            ConfigError::IO(err) => Some(err),
            ConfigError::Yaml(err) => Some(err),
        }
    }
}

impl From<io::Error> for ConfigError {
    fn from(error: io::Error) -> Self {
        ConfigError::IO(error)
    }
}

impl From<serde_yaml::Error> for ConfigError {
    fn from(error: serde_yaml::Error) -> Self {
        ConfigError::Yaml(error)
    }
}
