use async_trait::async_trait;
use tokio::sync::mpsc;

use crate::{Output, Sink, SinkError, config::SinkConfig};

pub struct ChannelSink {
    sender: mpsc::Sender<Output>,
}

impl ChannelSink {
    pub fn new(sender: mpsc::Sender<Output>) -> Self {
        Self { sender }
    }
}

#[async_trait]
impl Sink for ChannelSink {
    async fn handle(&self, output: &Output, _sink_config: &SinkConfig) -> Result<(), SinkError> {
        self.sender
            .send(output.clone())
            .await
            .map_err(|error| SinkError::Handle {
                message: error.to_string(),
            })
    }
}
