use async_trait::async_trait;
use rubo_engine::{
    ChannelSinkFactory, ChannelSourceFactory, Device, DevicePool, DeviceRef, DeviceRegister,
    FuncResult, Function, FunctionCall, FunctionError, FunctionRegister, Message, OutputState,
    RuntimeError, RuntimeResources, Sink, SinkError, SinkRegister, SinkRouteState, SourceError,
    SourceFactory, SourceHandler, SourceRegister,
    config::{BindingConfig, DeviceConfig, FuncConfig, RuboConfig, SinkConfig, SourceConfig},
    handle_message, run_config, run_config_sources, run_config_with_resources, run_source,
    run_source_messages,
};
use serde_json::json;
use std::{
    future::Future,
    pin::Pin,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    task::{Context, Poll, RawWaker, RawWakerVTable, Waker},
};
use tokio::sync::mpsc;

#[test]
fn handle_message_runs_dispatch_execute_and_sink_route() {
    let config = valid_config();
    let mut functions = FunctionRegister::new();
    functions.register("scale", ScaleFunction);
    let mut devices = DevicePool::new();
    devices.insert("camera", DeviceRef::shared("camera", Camera { value: 4 }));
    let sink_calls = Arc::new(AtomicUsize::new(0));
    let audit_calls = Arc::new(AtomicUsize::new(0));
    let mut sinks = SinkRegister::new();
    sinks.register("web", CountingSink::new(sink_calls.clone()));
    sinks.register("audit", CountingSink::new(audit_calls.clone()));

    let result = block_on(handle_message(
        "source",
        Message::new("frame")
            .started_at_ms(9)
            .payload(json!({ "value": 2 })),
        &config,
        &functions,
        &devices,
        &sinks,
    ));

    assert_eq!(result.output().route().binding_id(), Some("binding"));
    assert_eq!(result.output().route().sink_ids().len(), 2);
    match result.output().state() {
        OutputState::Success(func_result) => assert_eq!(func_result.value()["value"], 8),
        OutputState::Error(error) => panic!("unexpected output error: {}", error.message()),
    }
    assert_eq!(result.sink_results().len(), 2);
    assert!(
        result
            .sink_results()
            .iter()
            .all(|result| matches!(result.state(), SinkRouteState::Handled))
    );
    assert_eq!(sink_calls.load(Ordering::SeqCst), 1);
    assert_eq!(audit_calls.load(Ordering::SeqCst), 1);
}

#[test]
fn handle_message_wraps_dispatch_error_without_running_sinks() {
    let result = block_on(handle_message(
        "source",
        Message::new("missing"),
        &valid_config(),
        &FunctionRegister::new(),
        &DevicePool::new(),
        &SinkRegister::new(),
    ));

    assert_eq!(result.output().route().source_id(), "source");
    assert_eq!(result.output().route().key(), "missing");
    match result.output().state() {
        OutputState::Success(_) => panic!("unexpected success output"),
        OutputState::Error(error) => {
            assert_eq!(error.kind(), &rubo_engine::OutputErrorKind::Dispatch)
        }
    }
    assert!(result.sink_results().is_empty());
}

#[test]
fn run_source_messages_handles_messages_until_channel_closes() {
    let config = valid_config();
    let mut functions = FunctionRegister::new();
    functions.register("scale", ScaleFunction);
    let mut devices = DevicePool::new();
    devices.insert("camera", DeviceRef::shared("camera", Camera { value: 3 }));
    let sink_calls = Arc::new(AtomicUsize::new(0));
    let mut sinks = SinkRegister::new();
    sinks.register("web", CountingSink::new(sink_calls.clone()));
    sinks.register("audit", CountingSink::new(sink_calls.clone()));
    let (sender, receiver) = mpsc::channel(8);
    sender
        .try_send(Message::new("frame").payload(json!({ "value": 2 })))
        .unwrap();
    sender
        .try_send(Message::new("frame").payload(json!({ "value": 5 })))
        .unwrap();
    drop(sender);

    let results = block_on(run_source_messages(
        "source", receiver, &config, &functions, &devices, &sinks,
    ));

    assert_eq!(results.len(), 2);
    assert_eq!(sink_calls.load(Ordering::SeqCst), 4);
    match results[0].output().state() {
        OutputState::Success(func_result) => assert_eq!(func_result.value()["value"], 6),
        OutputState::Error(error) => panic!("unexpected output error: {}", error.message()),
    }
    match results[1].output().state() {
        OutputState::Success(func_result) => assert_eq!(func_result.value()["value"], 15),
        OutputState::Error(error) => panic!("unexpected output error: {}", error.message()),
    }
}

#[tokio::test]
async fn run_source_injects_sender_and_connects_handler_to_runtime() {
    let config = valid_config();
    let mut functions = FunctionRegister::new();
    functions.register("scale", ScaleFunction);
    let mut devices = DevicePool::new();
    devices.insert("camera", DeviceRef::shared("camera", Camera { value: 2 }));
    let sink_calls = Arc::new(AtomicUsize::new(0));
    let mut sinks = SinkRegister::new();
    sinks.register("web", CountingSink::new(sink_calls.clone()));
    sinks.register("audit", CountingSink::new(sink_calls.clone()));
    let source_config = SourceConfig::new("source");

    let result = run_source(
        "source",
        TwoMessageHandler { sent: 0 },
        &source_config,
        1,
        &config,
        &functions,
        &devices,
        &sinks,
    )
    .await;

    assert!(matches!(
        result.source_result(),
        Err(SourceError::SourceHandle { .. })
    ));
    assert_eq!(result.runtime_outputs().len(), 2);
    assert_eq!(sink_calls.load(Ordering::SeqCst), 4);
}

#[tokio::test]
async fn run_config_sources_builds_and_runs_registered_sources() {
    let config = config_with_two_sources();
    let mut source_register = SourceRegister::new();
    source_register.register("finite", FiniteSourceFactory);
    let mut functions = FunctionRegister::new();
    functions.register("scale", ScaleFunction);
    let mut devices = DevicePool::new();
    devices.insert("camera", DeviceRef::shared("camera", Camera { value: 2 }));
    let sink_calls = Arc::new(AtomicUsize::new(0));
    let mut sinks = SinkRegister::new();
    sinks.register("web", CountingSink::new(sink_calls.clone()));
    sinks.register("audit", CountingSink::new(sink_calls.clone()));

    let results =
        run_config_sources(&source_register, 1, &config, &functions, &devices, &sinks).await;

    assert_eq!(results.len(), 2);
    assert!(results.iter().all(|result| matches!(
        result.source_result(),
        Err(SourceError::SourceHandle { .. })
    )));
    assert_eq!(
        results
            .iter()
            .map(|result| result.runtime_outputs().len())
            .sum::<usize>(),
        2
    );
    assert_eq!(sink_calls.load(Ordering::SeqCst), 4);
}

#[tokio::test]
async fn run_config_builds_device_pool_and_runs_config_sources() {
    let config = config_with_two_sources();
    let mut source_register = SourceRegister::new();
    source_register.register("finite", FiniteSourceFactory);
    let mut device_register = DeviceRegister::new();
    device_register.register_device::<Camera>("camera");
    let mut functions = FunctionRegister::new();
    functions.register("scale", ScaleFunction);
    let sink_calls = Arc::new(AtomicUsize::new(0));
    let mut sinks = SinkRegister::new();
    sinks.register("web", CountingSink::new(sink_calls.clone()));
    sinks.register("audit", CountingSink::new(sink_calls.clone()));

    let results = run_config(
        &config,
        &source_register,
        &device_register,
        &functions,
        &sinks,
        1,
    )
    .await
    .unwrap();

    assert_eq!(results.len(), 2);
    assert_eq!(sink_calls.load(Ordering::SeqCst), 4);
}

#[tokio::test]
async fn run_config_returns_runtime_error_when_device_pool_cannot_build() {
    let config = config_with_two_sources();
    let mut source_register = SourceRegister::new();
    source_register.register("finite", FiniteSourceFactory);

    let error = match run_config(
        &config,
        &source_register,
        &DeviceRegister::new(),
        &FunctionRegister::new(),
        &SinkRegister::new(),
        1,
    )
    .await
    {
        Ok(_) => panic!("unexpected runtime result"),
        Err(error) => error,
    };

    assert!(matches!(
        error,
        RuntimeError::Device { error: rubo_engine::DeviceError::KindNotRegistered { kind } } if kind == "camera"
    ));
}

#[tokio::test]
async fn run_config_returns_runtime_error_when_config_reference_is_invalid() {
    let mut config = config_with_two_sources();
    config.sources_mut().remove("first");
    let mut source_register = SourceRegister::new();
    source_register.register("finite", FiniteSourceFactory);

    let error = match run_config(
        &config,
        &source_register,
        &DeviceRegister::new(),
        &FunctionRegister::new(),
        &SinkRegister::new(),
        1,
    )
    .await
    {
        Ok(_) => panic!("unexpected runtime result"),
        Err(error) => error,
    };

    assert!(matches!(
        error,
        RuntimeError::ConfigInvalid { message } if message.contains("first_binding") && message.contains("source")
    ));
}

#[tokio::test]
async fn run_config_returns_runtime_error_when_source_kind_is_missing() {
    let mut config = config_with_two_sources();
    config
        .sources_mut()
        .insert("first".to_string(), SourceConfig::new("first"));
    let mut source_register = SourceRegister::new();
    source_register.register("finite", FiniteSourceFactory);

    let error = match run_config(
        &config,
        &source_register,
        &DeviceRegister::new(),
        &FunctionRegister::new(),
        &SinkRegister::new(),
        1,
    )
    .await
    {
        Ok(_) => panic!("unexpected runtime result"),
        Err(error) => error,
    };

    assert!(matches!(
        error,
        RuntimeError::ConfigInvalid { message } if message.contains("first") && message.contains("kind")
    ));
}

#[tokio::test]
async fn run_config_returns_runtime_error_when_source_kind_is_not_registered() {
    let config = config_with_two_sources();

    let error = match run_config(
        &config,
        &SourceRegister::new(),
        &DeviceRegister::new(),
        &FunctionRegister::new(),
        &SinkRegister::new(),
        1,
    )
    .await
    {
        Ok(_) => panic!("unexpected runtime result"),
        Err(error) => error,
    };

    assert!(matches!(
        error,
        RuntimeError::ConfigInvalid { message } if message.contains("finite") && message.contains("source kind")
    ));
}

#[tokio::test]
async fn run_config_routes_function_registration_error_to_binding_sinks() {
    let config = config_with_two_sources();
    let mut source_register = SourceRegister::new();
    source_register.register("finite", FiniteSourceFactory);
    let mut device_register = DeviceRegister::new();
    device_register.register_device::<Camera>("camera");
    let sink_calls = Arc::new(AtomicUsize::new(0));
    let mut sinks = SinkRegister::new();
    sinks.register("web", CountingSink::new(sink_calls.clone()));
    sinks.register("audit", CountingSink::new(sink_calls.clone()));

    let results = run_config(
        &config,
        &source_register,
        &device_register,
        &FunctionRegister::new(),
        &sinks,
        1,
    )
    .await
    .unwrap();

    assert_eq!(results.len(), 2);
    assert_eq!(sink_calls.load(Ordering::SeqCst), 4);
    assert!(results.iter().all(|source| {
        source.runtime_outputs().iter().all(|runtime_output| {
            runtime_output.output().route().func_id() == Some("scale")
                && matches!(
                    runtime_output.output().state(),
                    OutputState::Error(error)
                        if error.kind() == &rubo_engine::OutputErrorKind::Runtime
                            && error.message().contains("function `scale` missing")
                )
                && runtime_output
                    .sink_results()
                    .iter()
                    .all(|result| matches!(result.state(), SinkRouteState::Handled))
        })
    }));
}

#[tokio::test]
async fn run_config_returns_runtime_error_when_sink_kind_is_missing() {
    let mut config = config_with_two_sources();
    config
        .sinks_mut()
        .insert("web".to_string(), SinkConfig::new("web"));
    let mut source_register = SourceRegister::new();
    source_register.register("finite", FiniteSourceFactory);
    let mut device_register = DeviceRegister::new();
    device_register.register_device::<Camera>("camera");
    let mut functions = FunctionRegister::new();
    functions.register("scale", ScaleFunction);
    let mut sinks = SinkRegister::new();
    sinks.register("web", CountingSink::new(Arc::new(AtomicUsize::new(0))));
    sinks.register("audit", CountingSink::new(Arc::new(AtomicUsize::new(0))));

    let error = match run_config(
        &config,
        &source_register,
        &device_register,
        &functions,
        &sinks,
        1,
    )
    .await
    {
        Ok(_) => panic!("unexpected runtime result"),
        Err(error) => error,
    };

    assert!(matches!(
        error,
        RuntimeError::ConfigInvalid { message } if message.contains("web") && message.contains("kind")
    ));
}

#[tokio::test]
async fn run_config_records_sink_registration_error_in_route_results() {
    let config = config_with_two_sources();
    let mut source_register = SourceRegister::new();
    source_register.register("finite", FiniteSourceFactory);
    let mut device_register = DeviceRegister::new();
    device_register.register_device::<Camera>("camera");
    let mut functions = FunctionRegister::new();
    functions.register("scale", ScaleFunction);

    let results = run_config(
        &config,
        &source_register,
        &device_register,
        &functions,
        &SinkRegister::new(),
        1,
    )
    .await
    .unwrap();

    assert_eq!(results.len(), 2);
    assert_eq!(
        results
            .iter()
            .flat_map(|source| source.runtime_outputs())
            .flat_map(|runtime_output| runtime_output.sink_results())
            .filter(|result| matches!(result.state(), SinkRouteState::SinkNotFound))
            .count(),
        4
    );
}

#[tokio::test]
async fn run_config_with_resources_runs_channel_source_and_channel_sink() {
    let mut config = valid_config();
    config.sources_mut().clear();
    config.sinks_mut().clear();
    config.bindings_mut().clear();
    config.sources_mut().insert(
        "input".to_string(),
        SourceConfig::new("input").kind("channel"),
    );
    config
        .sinks_mut()
        .insert("web".to_string(), SinkConfig::new("web").kind("channel"));
    config.bindings_mut().insert(
        "binding".to_string(),
        BindingConfig::new("binding")
            .source("input", "frame")
            .func("scale")
            .device("camera")
            .sink("web"),
    );
    let (source_sender, source_receiver) = mpsc::channel(4);
    source_sender
        .try_send(Message::new("frame").payload(json!({ "value": 5 })))
        .unwrap();
    drop(source_sender);
    let (sink_sender, mut sink_receiver) = mpsc::channel(4);
    let mut resources = RuntimeResources::new();
    resources.insert_source_channel("input", source_receiver);
    resources.insert_sink_channel("web", sink_sender);
    let mut source_register = SourceRegister::new();
    source_register.register("channel", ChannelSourceFactory);
    let mut sink_register = SinkRegister::new();
    sink_register
        .register_with_resources("web", ChannelSinkFactory, &mut resources)
        .unwrap();
    let mut device_register = DeviceRegister::new();
    device_register.register_device::<Camera>("camera");
    let mut functions = FunctionRegister::new();
    functions.register("scale", ScaleFunction);

    let results = run_config_with_resources(
        &config,
        &source_register,
        &mut resources,
        &device_register,
        &functions,
        &sink_register,
        1,
    )
    .await
    .unwrap();
    let output = sink_receiver.try_recv().unwrap();

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].runtime_outputs().len(), 1);
    assert_eq!(output.route().source_id(), "input");
    assert_eq!(output.route().key(), "frame");
    match output.state() {
        OutputState::Success(_) => {}
        OutputState::Error(error) => panic!("unexpected output error: {}", error.message()),
    }
}

fn valid_config() -> RuboConfig {
    let mut config = RuboConfig::default();
    config.sources_mut().insert(
        "source".to_string(),
        rubo_engine::config::SourceConfig::new("source"),
    );
    config
        .devices_mut()
        .insert("camera".to_string(), DeviceConfig::new("camera", "camera"));
    config
        .funcs_mut()
        .insert("scale".to_string(), FuncConfig::new("scale"));
    config
        .sinks_mut()
        .insert("web".to_string(), SinkConfig::new("web").kind("counting"));
    config.sinks_mut().insert(
        "audit".to_string(),
        SinkConfig::new("audit").kind("counting"),
    );
    config.bindings_mut().insert(
        "binding".to_string(),
        BindingConfig::new("binding")
            .source("source", "frame")
            .func("scale")
            .device("camera")
            .sink("web")
            .sink("audit"),
    );
    config
}

fn config_with_two_sources() -> RuboConfig {
    let mut config = valid_config();
    config.sources_mut().clear();
    config.bindings_mut().clear();
    config.sources_mut().insert(
        "first".to_string(),
        SourceConfig::new("first").kind("finite"),
    );
    config.sources_mut().insert(
        "second".to_string(),
        SourceConfig::new("second").kind("finite"),
    );
    config.bindings_mut().insert(
        "first_binding".to_string(),
        BindingConfig::new("first_binding")
            .source("first", "frame")
            .func("scale")
            .device("camera")
            .sink("web")
            .sink("audit"),
    );
    config.bindings_mut().insert(
        "second_binding".to_string(),
        BindingConfig::new("second_binding")
            .source("second", "frame")
            .func("scale")
            .device("camera")
            .sink("web")
            .sink("audit"),
    );
    config
}

struct Camera {
    value: i64,
}

#[async_trait]
impl Device for Camera {
    async fn create(_config: &DeviceConfig) -> Result<Self, rubo_engine::DeviceError> {
        Ok(Self { value: 0 })
    }
}

struct ScaleFunction;

#[async_trait]
impl Function for ScaleFunction {
    async fn call(&self, function_call: FunctionCall<'_>) -> Result<FuncResult, FunctionError> {
        let camera = function_call.devices().get::<Camera>("camera")?;
        Ok(FuncResult::new(json!({
            "value": function_call.message().payload_ref()["value"].as_i64().unwrap() * camera.value
        })))
    }
}

struct CountingSink {
    calls: Arc<AtomicUsize>,
}

struct TwoMessageHandler {
    sent: usize,
}

struct FiniteSourceFactory;

impl SourceFactory for FiniteSourceFactory {
    fn build(&self, _config: &SourceConfig) -> Result<Box<dyn SourceHandler>, SourceError> {
        Ok(Box::new(FiniteSourceHandler { sent: false }))
    }
}

struct FiniteSourceHandler {
    sent: bool,
}

#[async_trait]
impl SourceHandler for FiniteSourceHandler {
    async fn handle(&mut self, _config: &SourceConfig) -> Result<Message, SourceError> {
        if self.sent {
            return Err(SourceError::SourceHandle {
                message: "stop source".to_string(),
            });
        }
        self.sent = true;
        Ok(Message::new("frame").payload(json!({ "value": 3 })))
    }
}

#[async_trait]
impl SourceHandler for TwoMessageHandler {
    async fn handle(&mut self, _config: &SourceConfig) -> Result<Message, SourceError> {
        self.sent += 1;
        match self.sent {
            1 => Ok(Message::new("frame").payload(json!({ "value": 3 }))),
            2 => Ok(Message::new("frame").payload(json!({ "value": 4 }))),
            _ => Err(SourceError::SourceHandle {
                message: "stop source".to_string(),
            }),
        }
    }
}

impl CountingSink {
    fn new(calls: Arc<AtomicUsize>) -> Self {
        Self { calls }
    }
}

#[async_trait]
impl Sink for CountingSink {
    async fn handle(
        &self,
        _output: &rubo_engine::Output,
        _sink_config: &SinkConfig,
    ) -> Result<(), SinkError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}

fn block_on<F>(mut future: F) -> F::Output
where
    F: Future,
{
    let waker = noop_waker();
    let mut context = Context::from_waker(&waker);
    let mut future = unsafe { Pin::new_unchecked(&mut future) };
    loop {
        match future.as_mut().poll(&mut context) {
            Poll::Ready(output) => return output,
            Poll::Pending => panic!("test future unexpectedly pending"),
        }
    }
}

fn noop_waker() -> Waker {
    unsafe { Waker::from_raw(noop_raw_waker()) }
}

fn noop_raw_waker() -> RawWaker {
    RawWaker::new(std::ptr::null(), &NOOP_RAW_WAKER_VTABLE)
}

static NOOP_RAW_WAKER_VTABLE: RawWakerVTable =
    RawWakerVTable::new(noop_clone, noop_wake, noop_wake, noop_drop);

unsafe fn noop_clone(_: *const ()) -> RawWaker {
    noop_raw_waker()
}

unsafe fn noop_wake(_: *const ()) {}

unsafe fn noop_drop(_: *const ()) {}
