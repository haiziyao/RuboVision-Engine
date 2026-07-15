use async_trait::async_trait;
use rubo_engine::{Output, Sink, SinkError, config::SinkConfig};

#[derive(Default)]
pub(crate) struct HeadlessWebSink;

#[async_trait]
impl Sink for HeadlessWebSink {
    async fn handle(&self, _output: &Output, _sink_config: &SinkConfig) -> Result<(), SinkError> {
        Ok(())
    }
}
