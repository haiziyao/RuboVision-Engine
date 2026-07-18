use crate::{
    DevicePool, DispatchMessage, FunctionRegister, Message, Output, SinkRegister, SinkRouteResult,
    config::RuboConfig, dispatch, execute, log::text, route_output,
};
use tracing::{Instrument, info, info_span};

pub async fn handle_message(
    source_id: impl Into<String>,
    message: Message,
    config: &RuboConfig,
    functions: &FunctionRegister,
    devices: &DevicePool,
    sinks: &SinkRegister,
    image_enabled: bool,
) -> RuntimeOutput {
    let source_id = source_id.into();
    let key = message.key().to_string();
    let span_source_id = source_id.clone();
    let span_key = key.clone();
    async move {
        info!(
            "{}",
            text(format!(
                "runtime.message.handle source={} key={}",
                source_id, key
            ))
        );
        let dispatch_output = dispatch(config, DispatchMessage::new(source_id, message));
        let output = execute(dispatch_output, config, functions, devices, image_enabled).await;
        let sink_results = route_output(&output, config, sinks).await;
        RuntimeOutput::new(output, sink_results)
    }
    .instrument(info_span!(
        "runtime.handle_message",
        source_id = %span_source_id,
        key = %span_key
    ))
    .await
}

pub struct RuntimeOutput {
    output: Output,
    sink_results: Vec<SinkRouteResult>,
}

impl RuntimeOutput {
    pub fn new(output: Output, sink_results: Vec<SinkRouteResult>) -> Self {
        Self {
            output,
            sink_results,
        }
    }

    pub fn output(&self) -> &Output {
        &self.output
    }

    pub fn sink_results(&self) -> &[SinkRouteResult] {
        &self.sink_results
    }
}
