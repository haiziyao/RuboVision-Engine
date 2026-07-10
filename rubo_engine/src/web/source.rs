use serde_json::Value;
use tokio::sync::mpsc;

use crate::{Message, SourceError};

pub const WEB_SOURCE_ID: &str = "web";

pub struct WebSource {
    sender: mpsc::Sender<Message>,
}

impl WebSource {
    pub fn new(sender: mpsc::Sender<Message>) -> Self {
        Self { sender }
    }

    pub async fn trigger(
        &self,
        binding_id: impl Into<String>,
        description: impl Into<String>,
        payload: Value,
    ) -> Result<(), SourceError> {
        let message = Message::new(binding_id)
            .description(description)
            .payload(payload);
        self.sender
            .send(message)
            .await
            .map_err(|error| SourceError::SourceSend {
                message: error.to_string(),
            })
    }
}
