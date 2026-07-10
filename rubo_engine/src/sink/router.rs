use crate::{
    Output, SinkError,
    config::RuboConfig,
    log::{success_text, text, warn_text},
};
use tracing::{Instrument, info, info_span};

use super::SinkRegister;

pub async fn route_output(
    output: &Output,
    config: &RuboConfig,
    sinks: &SinkRegister,
) -> Vec<SinkRouteResult> {
    let source_id = output.route().source_id().to_string();
    let key = output.route().key().to_string();
    let span = info_span!("sink.route", source_id = %source_id, key = %key);
    async move {
        info!(
            "{}",
            text(format!(
                "sink.route.start source={} key={} sinks={}",
                source_id,
                key,
                output.route().sink_ids().len()
            ))
        );
        let mut results = Vec::new();
        for sink_id in output.route().sink_ids() {
            let Some(sink) = sinks.get(sink_id) else {
                info!(
                    "{}",
                    warn_text(format!("sink.route.missing sink={sink_id}"))
                );
                results.push(SinkRouteResult::new(sink_id, SinkRouteState::SinkNotFound));
                continue;
            };

            let Some(sink_config) = config.sinks().get(sink_id) else {
                info!(
                    "{}",
                    warn_text(format!("sink.route.config_missing sink={sink_id}"))
                );
                results.push(SinkRouteResult::new(
                    sink_id,
                    SinkRouteState::SinkConfigNotFound,
                ));
                continue;
            };

            match sink.handle(output, sink_config).await {
                Ok(()) => {
                    info!(
                        "{}",
                        success_text(format!("sink.route.handled sink={sink_id}"))
                    );
                    results.push(SinkRouteResult::new(sink_id, SinkRouteState::Handled));
                }
                Err(error) => {
                    info!(
                        "{}",
                        warn_text(format!(
                            "sink.route.handle_error sink={sink_id} error={error}"
                        ))
                    );
                    results.push(SinkRouteResult::new(
                        sink_id,
                        SinkRouteState::HandleError(error),
                    ));
                }
            }
        }
        results
    }
    .instrument(span)
    .await
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SinkRouteResult {
    sink_id: String,
    state: SinkRouteState,
}

impl SinkRouteResult {
    pub fn new(sink_id: impl Into<String>, state: SinkRouteState) -> Self {
        Self {
            sink_id: sink_id.into(),
            state,
        }
    }

    pub fn sink_id(&self) -> &str {
        &self.sink_id
    }

    pub fn state(&self) -> &SinkRouteState {
        &self.state
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SinkRouteState {
    Handled,
    SinkNotFound,
    SinkConfigNotFound,
    HandleError(SinkError),
}
