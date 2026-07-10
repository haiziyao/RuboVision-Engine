use std::convert::Infallible;

use axum::{
    Json,
    extract::{Path, Query, State},
    response::{
        Html,
        sse::{Event, Sse},
    },
};
use futures::{Stream, stream};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};

use crate::{
    config::{
        BindingConfig, ConfigAccess, DeviceConfig, FuncConfig, RuboConfig, SinkConfig,
        SourceConfig, save_update, update_binding, update_device, update_func, update_sink,
        update_source,
    },
    web::{WebError, WebEvent, WebInterface, WebOutputFrame, WebResponse, WebSource, WebState},
};

pub async fn index() -> Html<&'static str> {
    Html(index_html())
}

pub fn index_html() -> &'static str {
    include_str!("index.html")
}

pub async fn interface(State(state): State<WebState>) -> Json<WebResponse<WebInterface>> {
    Json(WebResponse::ok(WebInterface::from_config(state.config())))
}

pub async fn health(State(state): State<WebState>) -> Json<WebResponse<WebHealth>> {
    let config = state.runtime_config();
    let config = config.read().expect("web config lock poisoned");
    Json(WebResponse::ok(WebHealth {
        running: state
            .runtime_control()
            .is_some_and(|control| control.running()),
        web_enabled: state.config().enabled(),
        config_valid: config.validate(),
    }))
}

pub async fn runtime_summary(
    State(state): State<WebState>,
) -> Json<WebResponse<WebRuntimeSummary>> {
    let config = state.runtime_config();
    let (sources, devices, functions, sinks, bindings) = {
        let config = config.read().expect("web config lock poisoned");
        (
            config.sources().len(),
            config.devices().len(),
            config.funcs().len(),
            config.sinks().len(),
            config.bindings().len(),
        )
    };
    let history = state.history();
    let history = history.read().await;
    Json(WebResponse::ok(WebRuntimeSummary {
        running: state
            .runtime_control()
            .is_some_and(|control| control.running()),
        sources,
        devices,
        functions,
        sinks,
        bindings,
        output_count: history.count(),
        error_count: history.error_count(),
        last_output_at_ms: history.last_output_at_ms(),
    }))
}

pub async fn runtime_chain(State(state): State<WebState>) -> Json<WebResponse<Value>> {
    let config = state.runtime_config();
    let config = config.read().expect("web config lock poisoned");
    Json(WebResponse::ok(chain_data(&config)))
}

pub async fn runtime_outputs_latest(
    State(state): State<WebState>,
) -> Json<WebResponse<Vec<WebOutputFrame>>> {
    let history = state.history();
    let history = history.read().await;
    Json(WebResponse::ok(history.latest(20)))
}

pub async fn runtime_events(
    State(state): State<WebState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let receiver = state.hub().subscribe();
    let stream = stream::unfold(receiver, |mut receiver| async move {
        match receiver.recv().await {
            Ok(event) => {
                let data = serde_json::to_string(&event).unwrap_or_else(|_| "{}".to_string());
                let item = Event::default().event(event.kind().event_name()).data(data);
                Some((Ok(item), receiver))
            }
            Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                let event = WebEvent::runtime_error("web event stream lagged");
                let data = serde_json::to_string(&event).unwrap_or_else(|_| "{}".to_string());
                let item = Event::default().event(event.kind().event_name()).data(data);
                Some((Ok(item), receiver))
            }
            Err(tokio::sync::broadcast::error::RecvError::Closed) => None,
        }
    });
    Sse::new(stream)
}

pub async fn runtime_start(State(state): State<WebState>) -> Json<WebResponse<String>> {
    let Some(control) = state.runtime_control() else {
        return Json(WebResponse::error(WebError::runtime(
            "runtime control is not connected",
        )));
    };
    match control.start().await {
        Ok(()) => Json(WebResponse::ok("started".to_string())),
        Err(error) => Json(WebResponse::error(WebError::runtime(error))),
    }
}

pub async fn runtime_stop(State(state): State<WebState>) -> Json<WebResponse<String>> {
    let Some(control) = state.runtime_control() else {
        return Json(WebResponse::error(WebError::runtime(
            "runtime control is not connected",
        )));
    };
    match control.stop().await {
        Ok(()) => Json(WebResponse::ok("stopped".to_string())),
        Err(error) => Json(WebResponse::error(WebError::runtime(error))),
    }
}

pub async fn runtime_restart(State(state): State<WebState>) -> Json<WebResponse<String>> {
    let Some(control) = state.runtime_control() else {
        return Json(WebResponse::error(WebError::runtime(
            "runtime control is not connected",
        )));
    };
    match control.restart().await {
        Ok(()) => Json(WebResponse::ok("restarted".to_string())),
        Err(error) => Json(WebResponse::error(WebError::runtime(error))),
    }
}

pub async fn runtime_control_status(
    State(state): State<WebState>,
) -> Json<WebResponse<WebRuntimeControlStatus>> {
    Json(WebResponse::ok(WebRuntimeControlStatus {
        running: state
            .runtime_control()
            .is_some_and(|control| control.running()),
    }))
}

pub async fn outputs(
    State(state): State<WebState>,
    Query(query): Query<OutputQuery>,
) -> Json<WebResponse<Vec<WebOutputFrame>>> {
    let history = state.history();
    let history = history.read().await;
    let limit = query.limit.unwrap_or(100);
    let frames = history
        .all_latest_first()
        .into_iter()
        .filter(|frame| output_matches(frame, &query))
        .take(limit)
        .collect();
    Json(WebResponse::ok(frames))
}

pub async fn outputs_latest(
    State(state): State<WebState>,
) -> Json<WebResponse<Vec<WebOutputFrame>>> {
    let history = state.history();
    let history = history.read().await;
    Json(WebResponse::ok(history.latest(100)))
}

pub async fn output_detail(
    State(state): State<WebState>,
    Path(id): Path<u64>,
) -> Json<WebResponse<WebOutputFrame>> {
    let history = state.history();
    let history = history.read().await;
    match history.get(id) {
        Some(frame) => Json(WebResponse::ok(frame)),
        None => Json(WebResponse::error(WebError::not_found(format!(
            "output `{id}` not found"
        )))),
    }
}

pub async fn config(State(state): State<WebState>) -> Json<WebResponse<RuboConfig>> {
    let config = state.rubo_config();
    let config = config.read().expect("web config lock poisoned");
    Json(WebResponse::ok(config.clone()))
}

pub async fn config_sources(
    State(state): State<WebState>,
) -> Json<WebResponse<std::collections::HashMap<String, SourceConfig>>> {
    let config = state.rubo_config();
    let config = config.read().expect("web config lock poisoned");
    Json(WebResponse::ok(config.sources().clone()))
}

pub async fn config_devices(
    State(state): State<WebState>,
) -> Json<WebResponse<std::collections::HashMap<String, DeviceConfig>>> {
    let config = state.rubo_config();
    let config = config.read().expect("web config lock poisoned");
    Json(WebResponse::ok(config.devices().clone()))
}

pub async fn config_functions(
    State(state): State<WebState>,
) -> Json<WebResponse<std::collections::HashMap<String, FuncConfig>>> {
    let config = state.rubo_config();
    let config = config.read().expect("web config lock poisoned");
    Json(WebResponse::ok(config.funcs().clone()))
}

pub async fn config_sinks(
    State(state): State<WebState>,
) -> Json<WebResponse<std::collections::HashMap<String, SinkConfig>>> {
    let config = state.rubo_config();
    let config = config.read().expect("web config lock poisoned");
    Json(WebResponse::ok(config.sinks().clone()))
}

pub async fn config_bindings(
    State(state): State<WebState>,
) -> Json<WebResponse<std::collections::HashMap<String, BindingConfig>>> {
    let config = state.rubo_config();
    let config = config.read().expect("web config lock poisoned");
    Json(WebResponse::ok(config.bindings().clone()))
}

pub async fn update_source_api(
    State(state): State<WebState>,
    Path(id): Path<String>,
    Json(source): Json<SourceConfig>,
) -> Json<WebResponse<String>> {
    if source.id() != id {
        return Json(WebResponse::error(WebError::invalid_request(format!(
            "source path id `{id}` does not match body id `{}`",
            source.id()
        ))));
    }
    let config = state.rubo_config();
    let mut config = config.write().expect("web config lock poisoned");
    update_source(&mut config, source);
    state
        .hub()
        .publish(WebEvent::config_updated(format!("source `{id}` updated")));
    Json(WebResponse::ok(id))
}

pub async fn remove_source_api(
    State(state): State<WebState>,
    Path(id): Path<String>,
) -> Json<WebResponse<String>> {
    let config = state.rubo_config();
    let mut config = config.write().expect("web config lock poisoned");
    match config.sources_mut().remove(&id) {
        Some(_) => {
            state
                .hub()
                .publish(WebEvent::config_updated(format!("source `{id}` removed")));
            Json(WebResponse::ok(id))
        }
        None => Json(WebResponse::error(WebError::not_found(format!(
            "source `{id}` not found"
        )))),
    }
}

pub async fn update_device_api(
    State(state): State<WebState>,
    Path(id): Path<String>,
    Json(device): Json<DeviceConfig>,
) -> Json<WebResponse<String>> {
    if device.id() != id {
        return Json(WebResponse::error(WebError::invalid_request(format!(
            "device path id `{id}` does not match body id `{}`",
            device.id()
        ))));
    }
    let config = state.rubo_config();
    let mut config = config.write().expect("web config lock poisoned");
    update_device(&mut config, device);
    state
        .hub()
        .publish(WebEvent::config_updated(format!("device `{id}` updated")));
    Json(WebResponse::ok(id))
}

pub async fn update_function_api(
    State(state): State<WebState>,
    Path(id): Path<String>,
    Json(function): Json<FuncConfig>,
) -> Json<WebResponse<String>> {
    if function.id() != id {
        return Json(WebResponse::error(WebError::invalid_request(format!(
            "function path id `{id}` does not match body id `{}`",
            function.id()
        ))));
    }
    let config = state.rubo_config();
    let mut config = config.write().expect("web config lock poisoned");
    update_func(&mut config, function);
    state
        .hub()
        .publish(WebEvent::config_updated(format!("function `{id}` updated")));
    Json(WebResponse::ok(id))
}

pub async fn update_sink_api(
    State(state): State<WebState>,
    Path(id): Path<String>,
    Json(sink): Json<SinkConfig>,
) -> Json<WebResponse<String>> {
    if sink.id() != id {
        return Json(WebResponse::error(WebError::invalid_request(format!(
            "sink path id `{id}` does not match body id `{}`",
            sink.id()
        ))));
    }
    let config = state.rubo_config();
    let mut config = config.write().expect("web config lock poisoned");
    update_sink(&mut config, sink);
    state
        .hub()
        .publish(WebEvent::config_updated(format!("sink `{id}` updated")));
    Json(WebResponse::ok(id))
}

pub async fn update_binding_api(
    State(state): State<WebState>,
    Path(id): Path<String>,
    Json(binding): Json<BindingConfig>,
) -> Json<WebResponse<String>> {
    if binding.id() != id {
        return Json(WebResponse::error(WebError::invalid_request(format!(
            "binding path id `{id}` does not match body id `{}`",
            binding.id()
        ))));
    }
    let config = state.rubo_config();
    let mut config = config.write().expect("web config lock poisoned");
    update_binding(&mut config, binding);
    state
        .hub()
        .publish(WebEvent::config_updated(format!("binding `{id}` updated")));
    Json(WebResponse::ok(id))
}

pub async fn config_validate(State(state): State<WebState>) -> Json<WebResponse<WebConfigValid>> {
    let config = state.rubo_config();
    let config = config.read().expect("web config lock poisoned");
    Json(WebResponse::ok(WebConfigValid {
        valid: config.validate(),
    }))
}

pub async fn config_save(State(state): State<WebState>) -> Json<WebResponse<String>> {
    let config = state.rubo_config();
    let config = config.read().expect("web config lock poisoned");
    match save_update(state.root(), state.app_config(), &config) {
        Ok(()) => {
            state
                .hub()
                .publish(WebEvent::config_updated("config saved"));
            Json(WebResponse::ok("saved".to_string()))
        }
        Err(error) => Json(WebResponse::error(WebError::config(error.to_string()))),
    }
}

pub async fn debug_bindings(
    State(state): State<WebState>,
) -> Json<WebResponse<Vec<BindingConfig>>> {
    let config = state.rubo_config();
    let config = config.read().expect("web config lock poisoned");
    let bindings = config
        .bindings()
        .values()
        .filter(|binding| binding.debug_enabled())
        .cloned()
        .collect();
    Json(WebResponse::ok(bindings))
}

pub async fn debug_trigger(
    State(state): State<WebState>,
    Json(request): Json<WebDebugTriggerRequest>,
) -> Json<WebResponse<String>> {
    let Some(sender) = state.source_sender() else {
        return Json(WebResponse::error(WebError::runtime(
            "web source sender is not connected",
        )));
    };
    let source = WebSource::new(sender);
    match source
        .trigger(
            request.binding_id.clone(),
            request.description,
            request.payload,
        )
        .await
    {
        Ok(()) => Json(WebResponse::ok(request.binding_id)),
        Err(error) => Json(WebResponse::error(WebError::runtime(error.to_string()))),
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct WebHealth {
    running: bool,
    web_enabled: bool,
    config_valid: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct WebRuntimeSummary {
    running: bool,
    sources: usize,
    devices: usize,
    functions: usize,
    sinks: usize,
    bindings: usize,
    output_count: usize,
    error_count: usize,
    last_output_at_ms: Option<u64>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct WebRuntimeControlStatus {
    running: bool,
}

impl WebRuntimeControlStatus {
    pub fn running(&self) -> bool {
        self.running
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct WebConfigValid {
    valid: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WebDebugTriggerRequest {
    binding_id: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    payload: Value,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct OutputQuery {
    source: Option<String>,
    binding: Option<String>,
    function: Option<String>,
    sink: Option<String>,
    state: Option<String>,
    limit: Option<usize>,
}

fn output_matches(frame: &WebOutputFrame, query: &OutputQuery) -> bool {
    if let Some(source) = &query.source {
        if frame.route().source_id() != source {
            return false;
        }
    }
    if let Some(binding) = &query.binding {
        if frame.route().binding_id() != Some(binding.as_str()) {
            return false;
        }
    }
    if let Some(function) = &query.function {
        if frame.route().function_id() != Some(function.as_str()) {
            return false;
        }
    }
    if let Some(sink) = &query.sink {
        if !frame.route().sink_ids().iter().any(|id| id == sink) {
            return false;
        }
    }
    if let Some(state) = &query.state {
        if frame.state().state_name() != state {
            return false;
        }
    }
    true
}

fn chain_data(config: &RuboConfig) -> Value {
    let mut bindings: Vec<_> = config.bindings().values().collect();
    bindings.sort_by(|left, right| left.id().cmp(right.id()));
    Value::Array(
        bindings
            .into_iter()
            .map(|binding| {
                json!({
                    "binding": binding.id(),
                    "source": {
                        "id": binding.source_ref().id(),
                        "event": binding.source_ref().event(),
                        "config": config.sources().get(binding.source_ref().id()).map(values).unwrap_or_default()
                    },
                    "function": {
                        "id": binding.func_ref(),
                        "config": config.funcs().get(binding.func_ref()).map(values).unwrap_or_default()
                    },
                    "devices": binding.devices().iter().map(|id| {
                        let device = config.devices().get(id);
                        json!({
                            "id": id,
                            "kind": device.map(|device| device.kind()),
                            "config": device.map(values).unwrap_or_default()
                        })
                    }).collect::<Vec<_>>(),
                    "sinks": binding.sinks().iter().map(|id| {
                        json!({
                            "id": id,
                            "config": config.sinks().get(id).map(values).unwrap_or_default()
                        })
                    }).collect::<Vec<_>>(),
                    "debug": binding.debug_enabled()
                })
            })
            .collect(),
    )
}

fn values(config: &impl ConfigAccess) -> Map<String, Value> {
    config.values().clone()
}
