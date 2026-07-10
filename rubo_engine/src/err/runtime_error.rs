use std::fmt::{Display, Formatter};

use super::DeviceError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeError {
    ConfigInvalid { message: String },
    Device { error: DeviceError },
    TaskJoin { message: String },
}

impl Display for RuntimeError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ConfigInvalid { message } => {
                write!(formatter, "runtime config invalid: {message}")
            }
            Self::Device { error } => write!(formatter, "runtime device error: {error}"),
            Self::TaskJoin { message } => write!(formatter, "runtime task join error: {message}"),
        }
    }
}

impl std::error::Error for RuntimeError {}

impl From<DeviceError> for RuntimeError {
    fn from(error: DeviceError) -> Self {
        Self::Device { error }
    }
}
