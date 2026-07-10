use async_trait::async_trait;
use axum::{
    Json,
    extract::{Path, State},
};
use rubo_engine::{
    Device, Engine, FuncResult, Function, FunctionCall, FunctionError, Message, OutputState,
    config::{
        AppConfig, BindingConfig, ConfigAccess, ConfigStore, DeviceConfig, FuncConfig, RuboConfig,
        SinkConfig, SourceConfig,
    },
    web::api::{
        WebDebugTriggerRequest, config_save, debug_trigger, runtime_control_status, runtime_start,
        runtime_stop,
    },
    web::api::{update_binding_api, update_source_api},
};
use serde_json::json;
use tokio::{
    sync::mpsc,
    time::{Duration, sleep, timeout},
};

#[tokio::test]
async fn engine_run_once_orchestrates_channel_source_function_and_channel_sink() {
    let mut config = RuboConfig::default();
    config.sources_mut().insert(
        "input".to_string(),
        SourceConfig::new("input").kind("channel"),
    );
    config
        .devices_mut()
        .insert("camera".to_string(), DeviceConfig::new("camera", "camera"));
    config
        .funcs_mut()
        .insert("scale".to_string(), FuncConfig::new("scale"));
    config
        .sinks_mut()
        .insert("out".to_string(), SinkConfig::new("out").kind("channel"));
    config.bindings_mut().insert(
        "binding".to_string(),
        BindingConfig::new("binding")
            .source("input", "frame")
            .func("scale")
            .device("camera")
            .sink("out"),
    );
    let (source_sender, source_receiver) = mpsc::channel(4);
    source_sender
        .try_send(Message::new("frame").payload(json!({ "value": 4 })))
        .unwrap();
    drop(source_sender);
    let (sink_sender, mut sink_receiver) = mpsc::channel(4);
    let mut engine = Engine::new(".", AppConfig::default(), config);
    engine.insert_source_channel("input", source_receiver);
    engine.insert_sink_channel("out", sink_sender);
    engine.register_device::<Camera>("camera");
    engine.register_function("scale", ScaleFunction);

    let results = engine.run_once(1).await.unwrap();
    let output = sink_receiver.try_recv().unwrap();

    assert_eq!(results.len(), 1);
    match output.state() {
        OutputState::Success(result) => assert_eq!(result.value()["value"], 12),
        OutputState::Error(error) => panic!("unexpected output error: {}", error.message()),
    }
}

#[tokio::test]
async fn engine_run_is_the_main_runtime_orchestration_entry() {
    let mut config = RuboConfig::default();
    config.sources_mut().insert(
        "input".to_string(),
        SourceConfig::new("input").kind("channel"),
    );
    config
        .funcs_mut()
        .insert("scale".to_string(), FuncConfig::new("scale"));
    config
        .devices_mut()
        .insert("camera".to_string(), DeviceConfig::new("camera", "camera"));
    config
        .sinks_mut()
        .insert("out".to_string(), SinkConfig::new("out").kind("channel"));
    config.bindings_mut().insert(
        "binding".to_string(),
        BindingConfig::new("binding")
            .source("input", "frame")
            .func("scale")
            .device("camera")
            .sink("out"),
    );
    let (source_sender, source_receiver) = mpsc::channel(4);
    source_sender
        .try_send(Message::new("frame").payload(json!({ "value": 5 })))
        .unwrap();
    drop(source_sender);
    let (sink_sender, mut sink_receiver) = mpsc::channel(4);
    let mut engine = Engine::new(".", AppConfig::default(), config);
    engine.insert_source_channel("input", source_receiver);
    engine.insert_sink_channel("out", sink_sender);
    engine.register_device::<Camera>("camera");
    engine.register_function("scale", ScaleFunction);

    let results = engine.run(1).await.unwrap();
    let output = sink_receiver.try_recv().unwrap();

    assert_eq!(results.len(), 1);
    match output.state() {
        OutputState::Success(result) => assert_eq!(result.value()["value"], 15),
        OutputState::Error(error) => panic!("unexpected output error: {}", error.message()),
    }
}

#[test]
fn engine_prepare_web_keeps_generated_entries_out_of_user_config() {
    let mut config = RuboConfig::default();
    config.sources_mut().insert(
        "input".to_string(),
        SourceConfig::new("input").kind("channel"),
    );
    config
        .funcs_mut()
        .insert("inspect".to_string(), FuncConfig::new("inspect"));
    config.bindings_mut().insert(
        "inspect_input".to_string(),
        BindingConfig::new("inspect_input")
            .source("input", "frame")
            .func("inspect")
            .debug(true),
    );
    let mut engine = Engine::new(".", AppConfig::default(), config);

    engine.prepare_web();

    assert!(!engine.config().sources().contains_key("web"));
    assert!(!engine.config().sinks().contains_key("web"));
    assert!(
        !engine
            .config()
            .bindings()
            .contains_key("web.debug.inspect_input")
    );
    assert!(engine.sinks().contains("web"));
    let state = engine.web_state().unwrap();
    assert!(state.source_sender().is_some());
    let runtime_config = state.runtime_config();
    let runtime_config = runtime_config
        .read()
        .expect("test runtime config lock poisoned");
    assert_eq!(runtime_config.sources()["web"].kind_ref(), "channel");
    assert_eq!(runtime_config.sinks()["web"].kind_ref(), "web");
    assert!(
        runtime_config
            .bindings()
            .contains_key("web.debug.inspect_input")
    );
}

#[test]
fn engine_prepare_web_keeps_direct_web_debug_binding_without_deriving_a_duplicate() {
    let mut config = RuboConfig::default();
    config
        .funcs_mut()
        .insert("debug_fun".to_string(), FuncConfig::new("debug_fun"));
    config.bindings_mut().insert(
        "debug".to_string(),
        BindingConfig::new("debug")
            .source("web", "debug")
            .func("debug_fun")
            .sink("web")
            .debug(true),
    );
    let mut engine = Engine::new(".", AppConfig::default(), config);

    engine.prepare_web();

    let config = engine.config();
    assert!(config.bindings().contains_key("debug"));
    assert!(!config.bindings().contains_key("web.debug.debug"));
    drop(config);
    let runtime_config = engine.web_state().unwrap().runtime_config();
    let runtime_config = runtime_config
        .read()
        .expect("test runtime config lock poisoned");
    assert!(runtime_config.validate());
    assert!(runtime_config.bindings().contains_key("debug"));
    assert!(!runtime_config.bindings().contains_key("web.debug.debug"));
}

#[tokio::test]
async fn web_config_save_persists_only_user_config() {
    let temp = tempfile::tempdir().unwrap();
    let mut config = RuboConfig::default();
    config.sources_mut().insert(
        "input".to_string(),
        SourceConfig::new("input").kind("channel"),
    );
    config
        .funcs_mut()
        .insert("inspect".to_string(), FuncConfig::new("inspect"));
    config.bindings_mut().insert(
        "inspect_input".to_string(),
        BindingConfig::new("inspect_input")
            .source("input", "frame")
            .func("inspect")
            .debug(true),
    );
    let expected = config.clone();
    let mut engine = Engine::new(temp.path(), AppConfig::default(), config);
    engine.prepare_web();

    let Json(response) = config_save(State(engine.web_state().unwrap().clone())).await;
    let saved = ConfigStore::load_active_config(temp.path().join("config")).unwrap();

    assert!(response.is_ok());
    assert_eq!(saved, expected);
    assert!(!saved.sources().contains_key("web"));
    assert!(!saved.sinks().contains_key("web"));
    assert!(!saved.bindings().contains_key("web.debug.inspect_input"));
}

#[tokio::test]
async fn cloned_web_state_uses_runtime_sender_and_returns_debug_output_to_web_sink() {
    let mut config = RuboConfig::default();
    config
        .funcs_mut()
        .insert("debug_fun".to_string(), FuncConfig::new("debug_fun"));
    config.bindings_mut().insert(
        "debug".to_string(),
        BindingConfig::new("debug")
            .source("web", "debug")
            .func("debug_fun")
            .sink("web")
            .debug(true),
    );
    let mut engine = Engine::new(".", AppConfig::default(), config);
    engine.register_function("debug_fun", DebugFunction);
    engine.prepare_web();
    let state = engine.web_state().unwrap().clone();
    let mut runtime = engine.runtime(8);

    runtime.start();
    sleep(Duration::from_millis(20)).await;
    let request: WebDebugTriggerRequest = serde_json::from_value(json!({
        "binding_id": "debug",
        "description": "debug",
        "payload": {}
    }))
    .unwrap();
    let Json(response) = debug_trigger(State(state.clone()), Json(request)).await;
    sleep(Duration::from_millis(20)).await;

    assert!(response.is_ok());
    assert!(state.runtime_control().unwrap().running());
    let history = state.history();
    let history = history.read().await;
    let frame = history.latest(1).into_iter().next().unwrap();
    assert!(matches!(
        frame.state(),
        rubo_engine::WebOutputState::Success { value }
            if value["text"] == "debug success"
    ));
    drop(history);
    runtime.stop().await;
}

#[tokio::test]
async fn engine_start_returns_handle_that_can_stop_running_runtime() {
    let mut source = SourceConfig::new("ticker").kind("interval");
    source.values_mut().insert("key".to_string(), json!("tick"));
    source
        .values_mut()
        .insert("interval_ms".to_string(), json!(1));
    let mut config = RuboConfig::default();
    config.sources_mut().insert("ticker".to_string(), source);
    let engine = Engine::new(".", AppConfig::default(), config);

    let handle = engine.start(1);
    sleep(Duration::from_millis(20)).await;

    assert!(handle.is_running());
    handle.stop().await;
    assert!(handle.is_stopped());
}

#[tokio::test]
async fn web_update_source_api_updates_engine_config() {
    let mut config = RuboConfig::default();
    config.sources_mut().insert(
        "input".to_string(),
        SourceConfig::new("input").kind("channel"),
    );
    let mut engine = Engine::new(".", AppConfig::default(), config);
    engine.prepare_web();
    let state = engine.web_state().unwrap().clone();

    let Json(response) = update_source_api(
        State(state),
        Path("web_added".to_string()),
        Json(SourceConfig::new("web_added").kind("manual")),
    )
    .await;

    assert!(response.is_ok());
    assert!(engine.config().sources().contains_key("web_added"));
}

#[tokio::test]
async fn engine_runtime_restart_uses_web_updated_config() {
    let mut source = SourceConfig::new("ticker").kind("interval");
    source
        .values_mut()
        .insert("key".to_string(), json!("first"));
    source
        .values_mut()
        .insert("interval_ms".to_string(), json!(1));
    let mut config = RuboConfig::default();
    config.sources_mut().insert("ticker".to_string(), source);
    config
        .funcs_mut()
        .insert("echo".to_string(), FuncConfig::new("echo"));
    config
        .sinks_mut()
        .insert("out".to_string(), SinkConfig::new("out").kind("channel"));
    config.bindings_mut().insert(
        "binding".to_string(),
        BindingConfig::new("binding")
            .source("ticker", "first")
            .func("echo")
            .sink("out"),
    );
    let (sink_sender, mut sink_receiver) = mpsc::channel(16);
    let mut engine = Engine::new(".", AppConfig::default(), config);
    engine.insert_sink_channel("out", sink_sender);
    engine.register_function("echo", EchoFunction);
    engine.prepare_web();
    let state = engine.web_state().unwrap().clone();
    let mut runtime = engine.runtime(8);

    runtime.start();
    wait_for_output_key(&mut sink_receiver, "first").await;
    runtime.stop().await;

    let mut updated_source = SourceConfig::new("ticker").kind("interval");
    updated_source
        .values_mut()
        .insert("key".to_string(), json!("second"));
    updated_source
        .values_mut()
        .insert("interval_ms".to_string(), json!(1));
    let Json(response) = update_source_api(
        State(state),
        Path("ticker".to_string()),
        Json(updated_source),
    )
    .await;
    assert!(response.is_ok());
    let state = runtime.engine().lock().await.web_state().unwrap().clone();
    let Json(response) = update_binding_api(
        State(state),
        Path("binding".to_string()),
        Json(
            BindingConfig::new("binding")
                .source("ticker", "second")
                .func("echo")
                .sink("out"),
        ),
    )
    .await;
    assert!(response.is_ok());

    runtime.restart().await;
    wait_for_output_key(&mut sink_receiver, "second").await;
    runtime.stop().await;
}

#[tokio::test]
async fn engine_runtime_installs_web_runtime_control_api() {
    let mut source = SourceConfig::new("ticker").kind("interval");
    source.values_mut().insert("key".to_string(), json!("tick"));
    source
        .values_mut()
        .insert("interval_ms".to_string(), json!(1));
    let mut config = RuboConfig::default();
    config.sources_mut().insert("ticker".to_string(), source);
    let mut engine = Engine::new(".", AppConfig::default(), config);
    engine.prepare_web();
    let mut runtime = engine.runtime(8);
    let state = runtime.engine().lock().await.web_state().unwrap().clone();

    let Json(start) = runtime_start(State(state.clone())).await;
    sleep(Duration::from_millis(20)).await;
    let Json(status) = runtime_control_status(State(state.clone())).await;
    let Json(stop) = runtime_stop(State(state)).await;
    runtime.stop().await;

    assert!(start.is_ok());
    assert_eq!(status.data().unwrap().running(), true);
    assert!(stop.is_ok());
}

struct Camera;

#[async_trait]
impl Device for Camera {
    async fn create(_config: &DeviceConfig) -> Result<Self, rubo_engine::DeviceError> {
        Ok(Self)
    }
}

struct ScaleFunction;

#[async_trait]
impl Function for ScaleFunction {
    async fn call(&self, function_call: FunctionCall<'_>) -> Result<FuncResult, FunctionError> {
        let _camera = function_call.devices().get::<Camera>("camera")?;
        Ok(FuncResult::new(json!({
            "value": function_call.message().payload_ref()["value"].as_i64().unwrap() * 3
        })))
    }
}

struct DebugFunction;

#[async_trait]
impl Function for DebugFunction {
    async fn call(&self, _function_call: FunctionCall<'_>) -> Result<FuncResult, FunctionError> {
        Ok(FuncResult::new(json!({ "text": "debug success" })))
    }
}

struct EchoFunction;

#[async_trait]
impl Function for EchoFunction {
    async fn call(&self, function_call: FunctionCall<'_>) -> Result<FuncResult, FunctionError> {
        Ok(FuncResult::new(json!({
            "key": function_call.message().key()
        })))
    }
}

async fn wait_for_output_key(receiver: &mut mpsc::Receiver<rubo_engine::Output>, key: &str) {
    timeout(Duration::from_secs(1), async {
        loop {
            let output = receiver.recv().await.expect("output channel closed");
            if output.route().key() == key {
                break;
            }
        }
    })
    .await
    .expect("timed out waiting for output key");
}
