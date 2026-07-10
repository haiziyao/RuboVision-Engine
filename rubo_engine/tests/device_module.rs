use async_trait::async_trait;
use rubo_engine::{
    Device, DeviceError, DevicePool, DeviceRef, DeviceRegister, MutexDevice, build_device_pool,
    config::{ConfigAccess, DeviceConfig, RuboConfig},
};
use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll, RawWaker, RawWakerVTable, Waker},
};

#[test]
fn device_register_creates_shared_device_from_kind() {
    let mut register = DeviceRegister::new();
    register.register_device::<Camera>("camera");
    let config = DeviceConfig::new("front_camera", "camera").set("name", "front");

    let device = block_on(register.create(&config)).unwrap();
    let camera = device.get::<Camera>().unwrap();

    assert_eq!(camera.make_img(), "img:front");
}

#[test]
fn device_register_creates_mutex_device_from_kind() {
    let mut register = DeviceRegister::new();
    register.register_mutex_device::<Counter>("counter");
    let config = DeviceConfig::new("main_counter", "counter").set("value", 41_i64);

    let device = block_on(register.create(&config)).unwrap();
    let counter = device.get_mutex::<Counter>().unwrap();
    let mut counter = block_on(counter.lock());

    assert_eq!(counter.next(), 42);
}

#[test]
fn device_register_returns_kind_not_registered() {
    let register = DeviceRegister::new();
    let config = DeviceConfig::new("front_camera", "camera");

    let error = block_on(register.create(&config)).unwrap_err();

    assert!(matches!(
        error,
        DeviceError::KindNotRegistered { kind } if kind == "camera"
    ));
}

#[test]
fn device_ref_returns_type_mismatch_for_wrong_getter() {
    let mut register = DeviceRegister::new();
    register.register_device::<Camera>("camera");
    let config = DeviceConfig::new("front_camera", "camera").set("name", "front");

    let device = block_on(register.create(&config)).unwrap();
    let error = device.get_mutex::<Counter>().unwrap_err();

    assert!(matches!(
        error,
        DeviceError::TypeMismatch { id, expected } if id == "front_camera" && expected.contains("Counter")
    ));
}

#[test]
fn device_pool_inserts_and_gets_device_by_id() {
    let mut pool = DevicePool::new();
    let device = DeviceRef::shared(
        "front_camera",
        Camera {
            name: "front".to_string(),
        },
    );

    pool.insert("front_camera", device);

    let camera = pool.get("front_camera").unwrap().get::<Camera>().unwrap();
    assert_eq!(camera.make_img(), "img:front");
    assert!(pool.get("missing").is_none());
}

#[test]
fn build_device_pool_creates_devices_from_rubo_config() {
    let mut config = RuboConfig::default();
    config.devices_mut().insert(
        "front_camera".to_string(),
        DeviceConfig::new("front_camera", "camera").set("name", "front"),
    );
    config.devices_mut().insert(
        "main_counter".to_string(),
        DeviceConfig::new("main_counter", "counter").set("value", 1_i64),
    );
    let mut register = DeviceRegister::new();
    register.register_device::<Camera>("camera");
    register.register_mutex_device::<Counter>("counter");

    let pool = block_on(build_device_pool(&config, &register)).unwrap();

    let camera = pool.get("front_camera").unwrap().get::<Camera>().unwrap();
    let counter = pool
        .get("main_counter")
        .unwrap()
        .get_mutex::<Counter>()
        .unwrap();
    assert_eq!(camera.make_img(), "img:front");
    assert_eq!(block_on(counter.lock()).next(), 2);
}

#[test]
fn build_device_pool_returns_error_when_device_kind_is_not_registered() {
    let mut config = RuboConfig::default();
    config.devices_mut().insert(
        "front_camera".to_string(),
        DeviceConfig::new("front_camera", "missing"),
    );

    let error = block_on(build_device_pool(&config, &DeviceRegister::new())).unwrap_err();

    assert!(matches!(
        error,
        DeviceError::KindNotRegistered { kind } if kind == "missing"
    ));
}

struct Camera {
    name: String,
}

impl Camera {
    fn make_img(&self) -> String {
        format!("img:{}", self.name)
    }
}

#[async_trait]
impl Device for Camera {
    async fn create(config: &DeviceConfig) -> Result<Self, DeviceError> {
        Ok(Self {
            name: config.get("name")?,
        })
    }
}

#[derive(Debug)]
struct Counter {
    value: i64,
}

impl Counter {
    fn next(&mut self) -> i64 {
        self.value += 1;
        self.value
    }
}

#[async_trait]
impl MutexDevice for Counter {
    async fn create(config: &DeviceConfig) -> Result<Self, DeviceError> {
        Ok(Self {
            value: config.get("value")?,
        })
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
