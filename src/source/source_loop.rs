use std::sync::mpsc::Sender;
use std::thread::sleep;
use std::time::Duration;
use log::warn;
use anyhow::{anyhow, Result};
use axum::handler::Handler;
use tracing::info;
use crate::source::{Source, BaseSource, Event, make_event_usual};
use crate::config::binding::LoopBinding;

#[derive(Default)]
pub struct LoopSource {
    pub base: BaseSource,
}

impl Source for LoopSource {
    fn base(&self) -> &BaseSource {
        &self.base
    }

    fn base_mut(&mut self) -> &mut BaseSource {
        &mut self.base
    }

}


impl LoopSource {
    pub fn new() -> LoopSource {
        LoopSource::default()
    }

    pub async fn start(&self,loop_binding: Vec<LoopBinding>) ->Result<()> {
        let Some(_) = self.get_sender() else {
            warn!("LoopSource.listen called before sender was initialized");
            return Err(anyhow!("source sender is not initialized"));
        };

        // get Tasks

        for bind in &loop_binding {
            let event = make_event_usual(
                bind.task_id.as_str(),
                bind.function_id.as_str(),
                bind.device_id.as_str(),
            );

            info!("LoopSource sending event {:?}", bind);

            match self.send(event).await {
                Ok(()) => info!("LoopSource sent event {:?}", bind),
                Err(e) => warn!("LoopSource send event error: {:?}", e),
            }
        }

        loop {
            tokio::time::sleep(Duration::from_secs(5)).await;
            let event =Event::DebugEvent("debug".to_string());
            match self.send(event).await {
                Ok(()) => info!("LoopSource sends debug"),
                Err(e) => warn!("LoopSource sends nothing error {:?}", e)
            }
        }
        Ok(())
    }
}

