use async_trait::async_trait;
use rubo_engine::{
    Device, DevicePool, DeviceRef, DispatchMessage, FuncResult, Function, FunctionCall,
    FunctionError, FunctionRegister, Message, OutputErrorKind, OutputState,
    config::{BindingConfig, ConfigAccess, FuncConfig, RuboConfig},
    dispatch, execute,
};
use serde_json::json;
use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll, RawWaker, RawWakerVTable, Waker},
};

#[test]
fn execute_runs_task_request_and_returns_success_output() {
    let mut config = RuboConfig::default();
    config.bindings_mut().insert(
        "binding".to_string(),
        BindingConfig::new("binding")
            .source("source", "frame")
            .func("scale")
            .device("camera")
            .sink("web"),
    );
    config.funcs_mut().insert(
        "scale".to_string(),
        FuncConfig::new("scale").set("factor", 3_i64),
    );
    config.devices_mut().insert(
        "camera".to_string(),
        rubo_engine::config::DeviceConfig::new("camera", "camera"),
    );
    config.sinks_mut().insert(
        "web".to_string(),
        rubo_engine::config::SinkConfig::new("web"),
    );
    let mut functions = FunctionRegister::new();
    functions.register("scale", ScaleFunction);
    let mut devices = DevicePool::new();
    devices.insert("camera", DeviceRef::shared("camera", Camera { value: 4 }));
    let dispatch_output = dispatch(
        &config,
        DispatchMessage::new(
            "source",
            Message::new("frame")
                .started_at_ms(7)
                .payload(json!({ "value": 2 })),
        ),
    );

    let output = block_on(execute(dispatch_output, &config, &functions, &devices));

    assert_eq!(output.route().binding_id(), Some("binding"));
    assert_eq!(output.route().func_id(), Some("scale"));
    assert_eq!(output.route().sink_ids(), &["web".to_string()]);
    assert_eq!(output.timing().started_at_ms(), 7);
    assert!(output.timing().finished_at_ms() >= output.timing().started_at_ms());
    match output.state() {
        OutputState::Success(result) => assert_eq!(result.value()["value"], 24),
        OutputState::Error(error) => panic!("unexpected output error: {}", error.message()),
    }
}

#[test]
fn execute_wraps_dispatch_error_as_output_error() {
    let output = block_on(execute(
        dispatch(
            &RuboConfig::default(),
            DispatchMessage::new("source", Message::new("frame")),
        ),
        &RuboConfig::default(),
        &FunctionRegister::new(),
        &DevicePool::new(),
    ));

    assert_eq!(output.route().binding_id(), None);
    assert_eq!(output.route().source_id(), "source");
    assert_eq!(output.route().key(), "frame");
    assert_eq!(output.route().func_id(), None);
    match output.state() {
        OutputState::Success(_) => panic!("unexpected success output"),
        OutputState::Error(error) => assert_eq!(error.kind(), &OutputErrorKind::Dispatch),
    }
}

#[test]
fn execute_returns_runtime_error_when_function_is_missing() {
    let mut config = RuboConfig::default();
    config.bindings_mut().insert(
        "binding".to_string(),
        BindingConfig::new("binding")
            .source("source", "frame")
            .func("missing"),
    );
    config
        .funcs_mut()
        .insert("missing".to_string(), FuncConfig::new("missing"));
    let dispatch_output = dispatch(
        &config,
        DispatchMessage::new("source", Message::new("frame")),
    );

    let output = block_on(execute(
        dispatch_output,
        &config,
        &FunctionRegister::new(),
        &DevicePool::new(),
    ));

    match output.state() {
        OutputState::Success(_) => panic!("unexpected success output"),
        OutputState::Error(error) => assert_eq!(error.kind(), &OutputErrorKind::Runtime),
    }
}

struct Camera {
    value: i64,
}

#[async_trait]
impl Device for Camera {
    async fn create(
        _config: &rubo_engine::config::DeviceConfig,
    ) -> Result<Self, rubo_engine::DeviceError> {
        Ok(Self { value: 0 })
    }
}

struct ScaleFunction;

#[async_trait]
impl Function for ScaleFunction {
    async fn call(&self, function_call: FunctionCall<'_>) -> Result<FuncResult, FunctionError> {
        let factor: i64 = function_call.function_config().get("factor")?;
        let value = function_call.message().payload_ref()["value"]
            .as_i64()
            .unwrap();
        let camera = function_call.devices().get::<Camera>("camera")?;
        Ok(FuncResult::new(
            json!({ "value": value * factor * camera.value }),
        ))
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
