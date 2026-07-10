use async_trait::async_trait;
use rubo_engine::{
    ChannelSink, FuncResult, Output, OutputRoute, OutputState, OutputTiming, Sink, SinkError,
    SinkRegister, SinkRouteState,
    config::{ConfigAccess, RuboConfig, SinkConfig},
    route_output,
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
fn sink_register_stores_and_returns_sink_by_id() {
    let sink = CountingSink::new();
    let calls = sink.calls.clone();
    let mut register = SinkRegister::new();
    register.register("stdout", sink);
    let output = success_output();
    let config = SinkConfig::new("stdout").set("prefix", "ok");

    let sink = register.get("stdout").unwrap();
    block_on(sink.handle(&output, &config)).unwrap();

    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

#[test]
fn sink_register_returns_none_for_missing_id() {
    let register = SinkRegister::new();

    assert!(register.get("missing").is_none());
}

#[test]
fn sink_can_read_output_and_sink_config() {
    let sink = InspectSink;
    let output = success_output();
    let config = SinkConfig::new("inspect").set("prefix", "result");

    block_on(sink.handle(&output, &config)).unwrap();
}

#[test]
fn sink_config_stores_kind() {
    let config = SinkConfig::new("web").kind("channel");

    assert_eq!(config.id(), "web");
    assert_eq!(config.kind_ref(), "channel");
}

#[test]
fn route_output_handles_each_sink_and_keeps_failures_isolated() {
    let mut config = RuboConfig::default();
    config
        .sinks_mut()
        .insert("ok".to_string(), SinkConfig::new("ok").set("prefix", "ok"));
    config
        .sinks_mut()
        .insert("fail".to_string(), SinkConfig::new("fail"));
    let mut register = SinkRegister::new();
    register.register("ok", CountingSink::new());
    register.register("fail", FailingSink);
    let output = Output::success(
        OutputRoute::new(
            Some("binding"),
            "source",
            "frame",
            Some("func"),
            vec![
                "ok".to_string(),
                "fail".to_string(),
                "missing".to_string(),
                "no_config".to_string(),
            ],
        ),
        OutputTiming::new(1, 2),
        FuncResult::new(json!({ "value": 1 })),
    );
    register.register("no_config", CountingSink::new());

    let results = block_on(route_output(&output, &config, &register));

    assert_eq!(results.len(), 4);
    assert!(matches!(results[0].state(), SinkRouteState::Handled));
    assert!(matches!(results[1].state(), SinkRouteState::HandleError(_)));
    assert!(matches!(results[2].state(), SinkRouteState::SinkNotFound));
    assert!(matches!(
        results[3].state(),
        SinkRouteState::SinkConfigNotFound
    ));
}

#[test]
fn channel_sink_sends_output_to_receiver() {
    let (sender, mut receiver) = tokio::sync::mpsc::channel(4);
    let sink = ChannelSink::new(sender);
    let output = success_output();
    let config = SinkConfig::new("channel");

    block_on(sink.handle(&output, &config)).unwrap();
    let received = receiver.try_recv().unwrap();

    assert_eq!(received.route().binding_id(), Some("binding"));
    assert_eq!(received.route().key(), "frame");
}

#[test]
fn channel_sink_returns_error_when_receiver_is_closed() {
    let (sender, receiver) = tokio::sync::mpsc::channel(4);
    drop(receiver);
    let sink = ChannelSink::new(sender);

    let error = block_on(sink.handle(&success_output(), &SinkConfig::new("channel"))).unwrap_err();

    assert!(matches!(error, SinkError::Handle { .. }));
}

struct CountingSink {
    calls: Arc<AtomicUsize>,
}

impl CountingSink {
    fn new() -> Self {
        Self {
            calls: Arc::new(AtomicUsize::new(0)),
        }
    }
}

#[async_trait]
impl Sink for CountingSink {
    async fn handle(&self, output: &Output, sink_config: &SinkConfig) -> Result<(), SinkError> {
        assert_eq!(sink_config.get::<String>("prefix")?, "ok");
        assert_eq!(output.route().binding_id(), Some("binding"));
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}

struct InspectSink;

#[async_trait]
impl Sink for InspectSink {
    async fn handle(&self, output: &Output, sink_config: &SinkConfig) -> Result<(), SinkError> {
        assert_eq!(sink_config.get::<String>("prefix")?, "result");
        match output.state() {
            OutputState::Success(result) => assert_eq!(result.value()["value"], 1),
            OutputState::Error(error) => {
                panic!("unexpected sink output error: {}", error.message())
            }
        }
        Ok(())
    }
}

struct FailingSink;

#[async_trait]
impl Sink for FailingSink {
    async fn handle(&self, _output: &Output, _sink_config: &SinkConfig) -> Result<(), SinkError> {
        Err(SinkError::Handle {
            message: "fail sink".to_string(),
        })
    }
}

fn success_output() -> Output {
    Output::success(
        OutputRoute::new(
            Some("binding"),
            "source",
            "frame",
            Some("func"),
            vec!["stdout".to_string()],
        ),
        OutputTiming::new(1, 2),
        FuncResult::new(json!({ "value": 1 })),
    )
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
