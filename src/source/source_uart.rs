use anyhow::anyhow;
use log::warn;
use tracing::info;
use crate::source::{Source, BaseSource, make_event_usual};
use crate::source::source_timer::TimerSource;
use crate::config::binding::UartBinding;
#[derive(Default)]
pub struct UartSource {
    pub base: BaseSource,
    pub port: String,
}

impl Source for UartSource {
    fn base(&self) -> &BaseSource {
        &self.base
    }

    fn base_mut(&mut self) -> &mut BaseSource {
        &mut self.base
    }
}

impl UartSource {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn start(&self,uart_binding: Vec<UartBinding>) -> anyhow::Result<()> {

        // to get the sender
        let Some(tx) = self.get_sender() else {
            warn!("LoopSource.listen called before sender was initialized");
            return Err(anyhow!("source sender is not initialized"));
        };

        for bind in &uart_binding {
            let event = make_event_usual(
                bind.task_id.as_str(),
                bind.function_id.as_str(),
                bind.device_id.as_str(),
            );

            info!("UartSource sending event {:?}", bind);

            match self.send(event).await {
                Ok(()) => info!("UartSource sent event {:?}", bind),
                Err(e) => warn!("UartSource send event error: {:?}", e),
            }
        }
        
        Ok(())
    }
}