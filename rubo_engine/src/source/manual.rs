use async_trait::async_trait;
use tokio::sync::mpsc;

use crate::{Message, SourceError, SourceHandler, config::SourceConfig};

pub struct ManualSource {
    sender: mpsc::UnboundedSender<Message>,
    receiver: mpsc::UnboundedReceiver<Message>,
}

impl ManualSource {
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::unbounded_channel();
        Self { sender, receiver }
    }

    pub fn push(&mut self, message: Message) {
        let _ = self.sender.send(message);
    }
}

impl Default for ManualSource {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SourceHandler for ManualSource {
    async fn handle(&mut self, _config: &SourceConfig) -> Result<Message, SourceError> {
        self.receiver
            .recv()
            .await
            .ok_or_else(|| SourceError::SourceHandle {
                message: "manual source channel closed".to_string(),
            })
    }
}
