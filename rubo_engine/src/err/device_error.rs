use std::fmt::{Display, Formatter};

use super::ConfigError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeviceError {
    Create { message: String },
    KindNotRegistered { kind: String },
    TypeMismatch { id: String, expected: String },
}

impl Display for DeviceError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Create { message } => write!(formatter, "device create error: {message}"),
            Self::KindNotRegistered { kind } => {
                write!(formatter, "device kind not registered: {kind}")
            }
            Self::TypeMismatch { id, expected } => {
                write!(
                    formatter,
                    "device type mismatch: id={id} expected={expected}"
                )
            }
        }
    }
}

impl std::error::Error for DeviceError {}

impl From<ConfigError> for DeviceError {
    fn from(error: ConfigError) -> Self {
        Self::Create {
            message: error.to_string(),
        }
    }
}
