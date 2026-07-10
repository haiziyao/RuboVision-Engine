use async_trait::async_trait;

use crate::{Output, SinkError, config::SinkConfig};

#[async_trait]
pub trait Sink: Send + Sync + 'static {
    async fn handle(&self, output: &Output, sink_config: &SinkConfig) -> Result<(), SinkError>;
}
