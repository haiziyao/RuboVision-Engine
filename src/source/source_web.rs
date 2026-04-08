use anyhow::anyhow;
use log::warn;
use crate::source::{Source, BaseSource};
use crate::source::source_timer::TimerSource;
use crate::config::binding::WebBinding;
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

    pub async fn start(&self,web_binding: Vec<WebBinding>) -> anyhow::Result<()> {
        
        // to get the sender
        let Some(tx) = self.get_sender() else {
            warn!("LoopSource.listen called before sender was initialized");
            return Err(anyhow!("source sender is not initialized"));
        };
        
        // TODO
        
        Ok(())
    }
}