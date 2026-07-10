use crate::{DispatchError, Message};

#[derive(Debug, Clone, PartialEq)]
pub struct DispatchMessage {
    source_id: String,
    message: Message,
}

impl DispatchMessage {
    pub fn new(source_id: impl Into<String>, message: Message) -> Self {
        Self {
            source_id: source_id.into(),
            message,
        }
    }

    pub fn source_id(&self) -> &str {
        &self.source_id
    }

    pub fn message(&self) -> &Message {
        &self.message
    }

    pub(crate) fn into_parts(self) -> (String, Message) {
        (self.source_id, self.message)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum DispatchOutput {
    Task(TaskRequest),
    Error(DispatchError),
}

#[derive(Debug, Clone, PartialEq)]
pub struct TaskRequest {
    binding_id: String,
    source_id: String,
    key: String,
    func_id: String,
    message: Message,
    device_ids: Vec<String>,
    sink_ids: Vec<String>,
}

impl TaskRequest {
    pub(crate) fn new(
        binding_id: impl Into<String>,
        source_id: impl Into<String>,
        key: impl Into<String>,
        func_id: impl Into<String>,
        message: Message,
        device_ids: Vec<String>,
        sink_ids: Vec<String>,
    ) -> Self {
        Self {
            binding_id: binding_id.into(),
            source_id: source_id.into(),
            key: key.into(),
            func_id: func_id.into(),
            message,
            device_ids,
            sink_ids,
        }
    }

    pub fn binding_id(&self) -> &str {
        &self.binding_id
    }

    pub fn source_id(&self) -> &str {
        &self.source_id
    }

    pub fn key(&self) -> &str {
        &self.key
    }

    pub fn func_id(&self) -> &str {
        &self.func_id
    }

    pub fn message(&self) -> &Message {
        &self.message
    }

    pub fn device_ids(&self) -> &[String] {
        &self.device_ids
    }

    pub fn sink_ids(&self) -> &[String] {
        &self.sink_ids
    }
}
