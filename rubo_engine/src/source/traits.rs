use async_trait::async_trait;
use tokio::sync::mpsc;

use crate::{SourceError, config::SourceConfig};

use super::Message;

pub struct Source {
    id: String,
    sender: mpsc::Sender<Message>,
    handler: Box<dyn SourceHandler>,
}

impl Source {
    pub fn new(
        id: impl Into<String>,
        sender: mpsc::Sender<Message>,
        handler: impl SourceHandler,
    ) -> Self {
        Self {
            id: id.into(),
            sender,
            handler: Box::new(handler),
        }
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub async fn start(&mut self, config: &SourceConfig) -> Result<(), SourceError> {
        loop {
            let message = self.handler.handle(config).await?;
            self.sender
                .send(message)
                .await
                .map_err(|error| SourceError::SourceSend {
                    message: error.to_string(),
                })?;
        }
    }
}

#[async_trait]
pub trait SourceHandler: Send + 'static {
    async fn handle(&mut self, config: &SourceConfig) -> Result<Message, SourceError>;
}

#[async_trait]
impl<T> SourceHandler for Box<T>
where
    T: SourceHandler + ?Sized,
{
    async fn handle(&mut self, config: &SourceConfig) -> Result<Message, SourceError> {
        self.as_mut().handle(config).await
    }
}
