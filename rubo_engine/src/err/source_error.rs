use std::fmt::{Display, Formatter};

use super::ConfigError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceError {
    SourceHandle { message: String },
    SourceSend { message: String },
    KindMissing { id: String },
    KindNotRegistered { kind: String },
    ResourceMissing { id: String },
    Config { message: String },
}

impl Display for SourceError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SourceHandle { message } => write!(formatter, "source handle: {message}"),
            Self::SourceSend { message } => write!(formatter, "source send: {message}"),
            Self::KindMissing { id } => write!(formatter, "source kind missing: {id}"),
            Self::KindNotRegistered { kind } => {
                write!(formatter, "source kind not registered: {kind}")
            }
            Self::ResourceMissing { id } => write!(formatter, "source resource missing: {id}"),
            Self::Config { message } => write!(formatter, "source config: {message}"),
        }
    }
}

impl std::error::Error for SourceError {}

impl From<ConfigError> for SourceError {
    fn from(error: ConfigError) -> Self {
        Self::Config {
            message: error.to_string(),
        }
    }
}
