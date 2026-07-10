use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

use crate::WebOutputFrame;

#[derive(Debug, Clone)]
pub struct WebHub {
    sender: broadcast::Sender<WebEvent>,
}

impl WebHub {
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self { sender }
    }

    pub fn publish(&self, event: WebEvent) {
        let _ = self.sender.send(event);
    }

    pub fn subscribe(&self) -> broadcast::Receiver<WebEvent> {
        self.sender.subscribe()
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct WebEvent {
    kind: WebEventKind,
    frame: Option<WebOutputFrame>,
    message: Option<String>,
}

impl WebEvent {
    pub fn output(frame: WebOutputFrame) -> Self {
        Self {
            kind: WebEventKind::Output,
            frame: Some(frame),
            message: None,
        }
    }

    pub fn runtime_error(message: impl Into<String>) -> Self {
        Self {
            kind: WebEventKind::RuntimeError,
            frame: None,
            message: Some(message.into()),
        }
    }

    pub fn config_updated(message: impl Into<String>) -> Self {
        Self {
            kind: WebEventKind::ConfigUpdated,
            frame: None,
            message: Some(message.into()),
        }
    }

    pub fn heartbeat() -> Self {
        Self {
            kind: WebEventKind::Heartbeat,
            frame: None,
            message: None,
        }
    }

    pub fn kind(&self) -> &WebEventKind {
        &self.kind
    }

    pub fn frame(&self) -> Option<&WebOutputFrame> {
        self.frame.as_ref()
    }

    pub fn message(&self) -> Option<&str> {
        self.message.as_deref()
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WebEventKind {
    Output,
    RuntimeError,
    ConfigUpdated,
    Heartbeat,
}

impl WebEventKind {
    pub fn event_name(&self) -> &'static str {
        match self {
            Self::Output => "output",
            Self::RuntimeError => "runtime_error",
            Self::ConfigUpdated => "config_updated",
            Self::Heartbeat => "heartbeat",
        }
    }
}
