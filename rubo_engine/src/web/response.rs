use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct WebResponse<T> {
    ok: bool,
    data: Option<T>,
    error: Option<WebError>,
}

impl<T> WebResponse<T> {
    pub fn ok(data: T) -> Self {
        Self {
            ok: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn error(error: WebError) -> Self {
        Self {
            ok: false,
            data: None,
            error: Some(error),
        }
    }

    pub fn is_ok(&self) -> bool {
        self.ok
    }

    pub fn data(&self) -> Option<&T> {
        self.data.as_ref()
    }

    pub fn error_ref(&self) -> Option<&WebError> {
        self.error.as_ref()
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct WebError {
    kind: WebErrorKind,
    message: String,
}

impl WebError {
    pub fn config(message: impl Into<String>) -> Self {
        Self::new(WebErrorKind::Config, message)
    }

    pub fn runtime(message: impl Into<String>) -> Self {
        Self::new(WebErrorKind::Runtime, message)
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new(WebErrorKind::NotFound, message)
    }

    pub fn invalid_request(message: impl Into<String>) -> Self {
        Self::new(WebErrorKind::InvalidRequest, message)
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self::new(WebErrorKind::Internal, message)
    }

    pub fn kind(&self) -> &WebErrorKind {
        &self.kind
    }

    pub fn message(&self) -> &str {
        &self.message
    }

    fn new(kind: WebErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WebErrorKind {
    Config,
    Runtime,
    NotFound,
    InvalidRequest,
    Internal,
}
