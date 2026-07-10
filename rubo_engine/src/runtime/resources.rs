use std::collections::HashMap;

use tokio::sync::mpsc;

use crate::{Message, Output};

#[derive(Default)]
pub struct RuntimeResources {
    source_channels: HashMap<String, mpsc::Receiver<Message>>,
    sink_channels: HashMap<String, mpsc::Sender<Output>>,
}

impl RuntimeResources {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert_source_channel(
        &mut self,
        id: impl Into<String>,
        receiver: mpsc::Receiver<Message>,
    ) {
        self.source_channels.insert(id.into(), receiver);
    }

    pub fn take_source_channel(&mut self, id: &str) -> Option<mpsc::Receiver<Message>> {
        self.source_channels.remove(id)
    }

    pub fn insert_sink_channel(&mut self, id: impl Into<String>, sender: mpsc::Sender<Output>) {
        self.sink_channels.insert(id.into(), sender);
    }

    pub fn get_sink_channel(&self, id: &str) -> Option<mpsc::Sender<Output>> {
        self.sink_channels.get(id).cloned()
    }
}
