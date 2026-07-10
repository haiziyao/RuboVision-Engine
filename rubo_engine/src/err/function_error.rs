use std::fmt::{Display, Formatter};

use super::{ConfigError, DeviceError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FunctionError {
    Create { message: String },
    Call { message: String },
    IdNotRegistered { id: String },
    DeviceNotFound { id: String },
    Config { message: String },
    Device { message: String },
}

impl Display for FunctionError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Create { message } => write!(formatter, "function create error: {message}"),
            Self::Call { message } => write!(formatter, "function call error: {message}"),
            Self::IdNotRegistered { id } => write!(formatter, "function id not registered: {id}"),
            Self::DeviceNotFound { id } => write!(formatter, "function device not found: {id}"),
            Self::Config { message } => write!(formatter, "function config error: {message}"),
            Self::Device { message } => write!(formatter, "function device error: {message}"),
        }
    }
}

impl std::error::Error for FunctionError {}

impl From<ConfigError> for FunctionError {
    fn from(error: ConfigError) -> Self {
        Self::Config {
            message: error.to_string(),
        }
    }
}

impl From<DeviceError> for FunctionError {
    fn from(error: DeviceError) -> Self {
        Self::Device {
            message: error.to_string(),
        }
    }
}
