use anyhow::anyhow;
use log::warn;
use tracing::info;
use crate::source::{Source, BaseSource, make_event_usual};
use crate::config::binding::TimerBinding;
#[derive(Default)]
pub struct TimerSource {
    pub base: BaseSource,
}

impl Source for TimerSource {
    fn base(&self) -> &BaseSource {
        &self.base
    }

    fn base_mut(&mut self) -> &mut BaseSource {
        &mut self.base
    }
}

impl TimerSource {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn start(&self,timer_binding: Vec<TimerBinding> ) -> anyhow::Result<()> {

        // to get the sender
        let Some(tx) = self.get_sender() else {
            warn!("LoopSource.listen called before sender was initialized");
            return Err(anyhow!("source sender is not initialized"));
        };

        for bind in &timer_binding {
            let event = make_event_usual(
                bind.task_id.as_str(),
                bind.function_id.as_str(),
                bind.device_id.as_str(),
            );

            info!("TimerSource sending event {:?}", bind);

            match self.send(event).await {
                Ok(()) => info!("TimerSource sent event {:?}", bind),
                Err(e) => warn!("TimerSource send event error: {:?}", e),
            }
        }

        Ok(())
    }
}