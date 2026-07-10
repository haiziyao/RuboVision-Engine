use std::time::Duration;

use async_trait::async_trait;

use crate::{Message, SourceError, SourceHandler, config::SourceConfig};

pub struct IntervalSource {
    key: String,
    interval: Duration,
}

impl IntervalSource {
    pub fn new(key: impl Into<String>, interval: Duration) -> Self {
        Self {
            key: key.into(),
            interval,
        }
    }
}

#[async_trait]
impl SourceHandler for IntervalSource {
    async fn handle(&mut self, _config: &SourceConfig) -> Result<Message, SourceError> {
        if self.interval.is_zero() {
            return Err(SourceError::Config {
                message: "interval_ms must be greater than zero".to_string(),
            });
        }
        tokio::time::sleep(self.interval).await;
        Ok(Message::new(&self.key))
    }
}
