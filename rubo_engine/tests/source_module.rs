use rubo_engine::{
    ChannelSource, IntervalSource, IntervalSourceFactory, ManualSource, ManualSourceFactory,
    Message, Source, SourceError, SourceHandler, SourceRegister,
    config::{ConfigAccess, SourceConfig},
};
use serde_json::json;
use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll, RawWaker, RawWakerVTable, Waker},
    time::Duration,
};

#[test]
fn message_builder_sets_metadata_and_payload() {
    let event = Message::new("tick")
        .description("timer tick")
        .started_at_ms(42)
        .payload(json!({ "value": 7 }));

    assert_eq!(event.key(), "tick");
    assert_eq!(event.description_ref(), "timer tick");
    assert_eq!(event.started_at_ms_ref(), Some(42));
    assert_eq!(event.payload_ref()["value"], 7);
}

#[test]
fn source_start_calls_handler_and_sends_event_through_mpsc() {
    let (sender, mut receiver) = tokio::sync::mpsc::channel(4);
    let mut source = Source::new("timer", sender, OneEventHandler { calls: 0 });
    let config = SourceConfig::new("timer").set("interval_ms", 1000_u64);

    let error = block_on(source.start(&config)).unwrap_err();
    let event = receiver.try_recv().unwrap();

    assert!(matches!(error, SourceError::SourceHandle { .. }));
    assert_eq!(source.id(), "timer");
    assert_eq!(event.key(), "tick");
    assert_eq!(event.payload_ref()["interval_ms"], 1000);
}

#[test]
fn manual_source_returns_pushed_messages_in_order() {
    let mut source = ManualSource::new();
    source.push(Message::new("first"));
    source.push(Message::new("second"));
    let config = SourceConfig::new("manual");

    let first = block_on(source.handle(&config)).unwrap();
    let second = block_on(source.handle(&config)).unwrap();

    assert_eq!(first.key(), "first");
    assert_eq!(second.key(), "second");
}

#[test]
fn manual_source_waits_when_empty_instead_of_spinning() {
    let mut source = ManualSource::new();
    let config = SourceConfig::new("manual");
    let mut future = Box::pin(source.handle(&config));
    let waker = noop_waker();
    let mut context = Context::from_waker(&waker);

    assert!(matches!(future.as_mut().poll(&mut context), Poll::Pending));
}

#[test]
fn source_register_rejects_zero_interval() {
    let mut register = SourceRegister::new();
    register.register("interval", IntervalSourceFactory);
    let config = SourceConfig::new("timer")
        .kind("interval")
        .set("key", "tick")
        .set("interval_ms", 0_u64);

    let error = match register.build(&config) {
        Ok(_) => panic!("unexpected zero interval source"),
        Err(error) => error,
    };

    assert!(matches!(error, SourceError::Config { .. }));
}

#[test]
fn channel_source_reads_messages_from_receiver() {
    let (sender, receiver) = tokio::sync::mpsc::channel(4);
    let mut source = ChannelSource::new(receiver);
    sender.try_send(Message::new("frame")).unwrap();
    drop(sender);
    let config = SourceConfig::new("channel");

    let message = block_on(source.handle(&config)).unwrap();
    let closed = block_on(source.handle(&config)).unwrap_err();

    assert_eq!(message.key(), "frame");
    assert!(matches!(closed, SourceError::SourceHandle { .. }));
}

#[tokio::test]
async fn interval_source_creates_message_with_configured_key() {
    let mut source = IntervalSource::new("tick", Duration::from_millis(1));
    let config = SourceConfig::new("interval");

    let message = source.handle(&config).await.unwrap();

    assert_eq!(message.key(), "tick");
}

#[test]
fn source_register_builds_manual_source_by_kind() {
    let mut register = SourceRegister::new();
    register.register("manual", ManualSourceFactory);
    let config = SourceConfig::new("manual_source").kind("manual");

    assert!(register.build(&config).is_ok());
}

#[tokio::test]
async fn source_register_builds_interval_source_from_config() {
    let mut register = SourceRegister::new();
    register.register("interval", IntervalSourceFactory);
    let config = SourceConfig::new("timer")
        .kind("interval")
        .set("key", "tick")
        .set("interval_ms", 1_u64);

    let mut source = register.build(&config).unwrap();
    let message = source.handle(&config).await.unwrap();

    assert_eq!(message.key(), "tick");
}

#[test]
fn source_register_returns_error_when_kind_is_missing() {
    let register = SourceRegister::new();
    let error = match register.build(&SourceConfig::new("missing")) {
        Ok(_) => panic!("unexpected source handler"),
        Err(error) => error,
    };

    assert!(matches!(error, SourceError::KindMissing { .. }));
}

#[test]
fn source_register_returns_error_when_kind_is_not_registered() {
    let register = SourceRegister::new();
    let error = match register.build(&SourceConfig::new("unknown").kind("unknown")) {
        Ok(_) => panic!("unexpected source handler"),
        Err(error) => error,
    };

    assert!(matches!(error, SourceError::KindNotRegistered { .. }));
}

struct OneEventHandler {
    calls: usize,
}

#[async_trait::async_trait]
impl SourceHandler for OneEventHandler {
    async fn handle(&mut self, config: &SourceConfig) -> Result<Message, SourceError> {
        self.calls += 1;
        if self.calls == 1 {
            let interval_ms = config.get::<u64>("interval_ms")?;
            Ok(Message::new("tick").payload(json!({ "interval_ms": interval_ms })))
        } else {
            Err(SourceError::SourceHandle {
                message: "stop test source".to_string(),
            })
        }
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
