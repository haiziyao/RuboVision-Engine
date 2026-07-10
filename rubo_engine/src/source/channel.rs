use async_trait::async_trait;
use tokio::sync::mpsc;

use crate::{Message, SourceError, SourceHandler, config::SourceConfig};

pub struct ChannelSource {
    receiver: mpsc::Receiver<Message>,
}

impl ChannelSource {
    pub fn new(receiver: mpsc::Receiver<Message>) -> Self {
        Self { receiver }
    }
}

#[async_trait]
impl SourceHandler for ChannelSource {
    async fn handle(&mut self, _config: &SourceConfig) -> Result<Message, SourceError> {
        match self.receiver.recv().await {
            Some(message) => Ok(message),
            None => Err(SourceError::SourceHandle {
                message: "source channel closed".to_string(),
            }),
        }
    }
}
