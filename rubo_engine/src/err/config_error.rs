use std::fmt::{Display, Formatter};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigError {
    ConfigLoad { message: String },
    ConfigWrite { message: String },
    ConfigFormat { message: String },
    ConfigMismatch { message: String },
}

impl Display for ConfigError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ConfigLoad { message } => write!(formatter, "config load: {message}"),
            Self::ConfigWrite { message } => write!(formatter, "config write: {message}"),
            Self::ConfigFormat { message } => write!(formatter, "config format: {message}"),
            Self::ConfigMismatch { message } => write!(formatter, "config mismatch: {message}"),
        }
    }
}

impl std::error::Error for ConfigError {}

impl From<::config::ConfigError> for ConfigError {
    fn from(error: ::config::ConfigError) -> Self {
        Self::ConfigLoad {
            message: error.to_string(),
        }
    }
}

impl From<std::io::Error> for ConfigError {
    fn from(error: std::io::Error) -> Self {
        Self::ConfigWrite {
            message: error.to_string(),
        }
    }
}
