use crate::Message;

#[derive(Debug, Clone, PartialEq)]
pub struct DispatchError {
    source_id: String,
    key: String,
    message: Message,
    kind: DispatchErrorKind,
}

impl DispatchError {
    pub(crate) fn new(
        source_id: impl Into<String>,
        key: impl Into<String>,
        message: Message,
        kind: DispatchErrorKind,
    ) -> Self {
        Self {
            source_id: source_id.into(),
            key: key.into(),
            message,
            kind,
        }
    }

    pub fn source_id(&self) -> &str {
        &self.source_id
    }

    pub fn key(&self) -> &str {
        &self.key
    }

    pub fn message(&self) -> &Message {
        &self.message
    }

    pub fn kind(&self) -> &DispatchErrorKind {
        &self.kind
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DispatchErrorKind {
    BindingNotFound,
    BindingConflict,
    ConfigInvalid,
    FuncNotFound,
    DeviceNotFound,
    SinkNotFound,
}
