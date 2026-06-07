#![warn(dead_code)]

use crate::config::binding::DebugBinding;
use crate::source::{BaseSource, Source};
use anyhow::anyhow;
use log::warn;

#[derive(Default)]
pub struct WebSource {
    pub base: BaseSource,
}

impl Source for WebSource {
    fn base(&self) -> &BaseSource {
        &self.base
    }

    fn base_mut(&mut self) -> &mut BaseSource {
        &mut self.base
    }
}

impl WebSource {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn start(&self, _web_binding: Vec<DebugBinding>) -> anyhow::Result<()> {
        // to get the sender
        let Some(_tx) = self.get_sender() else {
            warn!("LoopSource.listen called before sender was initialized");
            return Err(anyhow!("source sender is not initialized"));
        };

        // TODO

        Ok(())
    }
}
