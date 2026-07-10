use std::fmt::{Display, Formatter};

use super::ConfigError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SinkError {
    Handle { message: String },
    KindMissing { id: String },
    KindNotRegistered { kind: String },
    ResourceMissing { id: String },
    Config { message: String },
}

impl Display for SinkError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Handle { message } => write!(formatter, "sink handle error: {message}"),
            Self::KindMissing { id } => write!(formatter, "sink kind missing: {id}"),
            Self::KindNotRegistered { kind } => {
                write!(formatter, "sink kind not registered: {kind}")
            }
            Self::ResourceMissing { id } => write!(formatter, "sink resource missing: {id}"),
            Self::Config { message } => write!(formatter, "sink config error: {message}"),
        }
    }
}

impl std::error::Error for SinkError {}

impl From<ConfigError> for SinkError {
    fn from(error: ConfigError) -> Self {
        Self::Config {
            message: error.to_string(),
        }
    }
}
