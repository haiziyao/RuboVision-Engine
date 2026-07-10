use async_trait::async_trait;
use rubo_engine::{
    Device, DeviceRef, FuncResult, Function, FunctionCall, FunctionDevices, FunctionError,
    FunctionRegister, Message,
    config::{ConfigAccess, DeviceConfig, FuncConfig},
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

#[test]
fn function_register_stores_and_returns_function_by_id() {
    let mut register = FunctionRegister::new();
    register.register("scale", ScaleFunction { factor: 3 });
    let config = FuncConfig::new("scale").set("factor", 99_i64);
    let message = Message::new("scale").payload(json!({ "value": 2 }));
    let camera = DeviceRef::shared("camera", Camera { value: 4 });
    let mut devices = FunctionDevices::new();
    devices.insert("camera", &camera);
    let function_call = FunctionCall::new(&config, &message, devices);

    let function = register.get("scale").unwrap();
    let result = block_on(function.call(function_call)).unwrap();

    assert_eq!(result.value()["value"], 24);
    assert_eq!(result.description_ref(), "scaled");
}

#[test]
fn function_call_reads_config_message_and_limited_devices() {
    let config = FuncConfig::new("scale").set("factor", 5_i64);
    let camera = DeviceRef::shared("camera", Camera { value: 2 });
    let mut devices = FunctionDevices::new();
    devices.insert("camera", &camera);
    let message = Message::new("scale").payload(json!({ "value": 3 }));
    let function_call = FunctionCall::new(&config, &message, devices);

    let factor: i64 = function_call.function_config().get("factor").unwrap();
    let message_value = function_call.message().payload_ref()["value"]
        .as_i64()
        .unwrap();
    let camera = function_call.devices().get::<Camera>("camera").unwrap();

    assert_eq!(factor, 5);
    assert_eq!(message_value, 3);
    assert_eq!(camera.make_img(), 2);
}

#[test]
fn function_register_holds_shared_function_for_concurrent_calls() {
    let function = FunctionRefCounter::new();
    let calls = function.calls.clone();
    let mut register = FunctionRegister::new();
    register.register("count", function);
    let config = FuncConfig::new("count");
    let message_one = Message::new("count").payload(json!({}));
    let message_two = Message::new("count").payload(json!({}));
    let function_call_one = FunctionCall::new(&config, &message_one, FunctionDevices::new());
    let function_call_two = FunctionCall::new(&config, &message_two, FunctionDevices::new());

    let function_one = register.get("count").unwrap();
    let function_two = register.get("count").unwrap();
    block_on(function_one.call(function_call_one)).unwrap();
    block_on(function_two.call(function_call_two)).unwrap();

    assert_eq!(calls.load(Ordering::SeqCst), 2);
}

#[test]
fn function_register_returns_none_for_missing_id() {
    let register = FunctionRegister::new();

    assert!(register.get("missing").is_none());
}

#[test]
fn function_devices_returns_error_when_device_is_not_allowed_for_call() {
    let devices = FunctionDevices::new();

    let error = match devices.get::<Camera>("camera") {
        Ok(_) => panic!("unexpected device returned"),
        Err(error) => error,
    };

    assert!(matches!(
        error,
        FunctionError::DeviceNotFound { id } if id == "camera"
    ));
}

struct Camera {
    value: i64,
}

impl Camera {
    fn make_img(&self) -> i64 {
        self.value
    }
}

#[async_trait]
impl Device for Camera {
    async fn create(_config: &DeviceConfig) -> Result<Self, rubo_engine::DeviceError> {
        Ok(Self { value: 0 })
    }
}

struct ScaleFunction {
    factor: i64,
}

#[async_trait]
impl Function for ScaleFunction {
    async fn call(&self, function_call: FunctionCall<'_>) -> Result<FuncResult, FunctionError> {
        let input = function_call.message().payload_ref()["value"]
            .as_i64()
            .unwrap();
        let camera = function_call.devices().get::<Camera>("camera")?;
        Ok(
            FuncResult::new(json!({ "value": input * self.factor * camera.make_img() }))
                .description("scaled"),
        )
    }
}

struct FunctionRefCounter {
    calls: Arc<AtomicUsize>,
}

impl FunctionRefCounter {
    fn new() -> Self {
        Self {
            calls: Arc::new(AtomicUsize::new(0)),
        }
    }
}

#[async_trait]
impl Function for FunctionRefCounter {
    async fn call(&self, _function_call: FunctionCall<'_>) -> Result<FuncResult, FunctionError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(FuncResult::new(json!({ "ok": true })))
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
