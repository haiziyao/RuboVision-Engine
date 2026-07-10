use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};
use std::time::{SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use tokio::sync::RwLock;

use crate::{
    Output, Sink, SinkError, WebEvent, WebHistory, WebHub, WebOutputFrame, config::SinkConfig,
};

pub const WEB_SINK_ID: &str = "web";

pub struct WebSink {
    history: Arc<RwLock<WebHistory>>,
    hub: WebHub,
    next_id: AtomicU64,
}

impl WebSink {
    pub fn new(history: Arc<RwLock<WebHistory>>, hub: WebHub) -> Self {
        Self {
            history,
            hub,
            next_id: AtomicU64::new(1),
        }
    }
}

#[async_trait]
impl Sink for WebSink {
    async fn handle(&self, output: &Output, _sink_config: &SinkConfig) -> Result<(), SinkError> {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let frame = WebOutputFrame::from_output(id, now_ms(), output);
        self.history.write().await.push(frame.clone());
        self.hub.publish(WebEvent::output(frame));
        Ok(())
    }
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}
