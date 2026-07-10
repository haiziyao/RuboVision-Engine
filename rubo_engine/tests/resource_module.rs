use rubo_engine::{
    ChannelSinkFactory, ChannelSourceFactory, Message, Output, OutputRoute, OutputTiming,
    RuntimeResources, SinkRegister, SourceRegister,
    config::{SinkConfig, SourceConfig},
};
use serde_json::json;

#[test]
fn resources_provide_channel_source_receiver_once() {
    let (sender, receiver) = tokio::sync::mpsc::channel(4);
    sender.try_send(Message::new("frame")).unwrap();
    let mut resources = RuntimeResources::new();
    resources.insert_source_channel("camera", receiver);
    let mut register = SourceRegister::new();
    register.register("channel", ChannelSourceFactory);
    let config = SourceConfig::new("camera").kind("channel");

    let mut source = register
        .build_with_resources(&config, &mut resources)
        .unwrap();
    let message = block_on(source.handle(&config)).unwrap();
    let error = match register.build_with_resources(&config, &mut resources) {
        Ok(_) => panic!("unexpected source handler"),
        Err(error) => error,
    };

    assert_eq!(message.key(), "frame");
    assert!(matches!(
        error,
        rubo_engine::SourceError::ResourceMissing { .. }
    ));
}

#[test]
fn resources_register_channel_sink() {
    let (sender, mut receiver) = tokio::sync::mpsc::channel(4);
    let mut resources = RuntimeResources::new();
    resources.insert_sink_channel("web", sender);
    let mut register = SinkRegister::new();
    register
        .register_with_resources("web", ChannelSinkFactory, &mut resources)
        .unwrap();

    let sink = register.get("web").unwrap();
    block_on(sink.handle(&success_output(), &SinkConfig::new("web"))).unwrap();
    let output = receiver.try_recv().unwrap();

    assert_eq!(output.route().source_id(), "source");
}

#[test]
fn resources_register_config_sinks_from_kind() {
    let (sender, mut receiver) = tokio::sync::mpsc::channel(4);
    let mut resources = RuntimeResources::new();
    resources.insert_sink_channel("web", sender);
    let mut config = rubo_engine::config::RuboConfig::default();
    config
        .sinks_mut()
        .insert("web".to_string(), SinkConfig::new("web").kind("channel"));
    let mut register = SinkRegister::new();
    register.register_factory("channel", ChannelSinkFactory);

    register
        .register_config_sinks_with_resources(&config, &mut resources)
        .unwrap();
    let sink = register.get("web").unwrap();
    block_on(sink.handle(&success_output(), &SinkConfig::new("web"))).unwrap();
    let output = receiver.try_recv().unwrap();

    assert_eq!(output.route().source_id(), "source");
}

#[test]
fn resources_register_config_sinks_returns_error_when_kind_is_missing() {
    let mut config = rubo_engine::config::RuboConfig::default();
    config
        .sinks_mut()
        .insert("web".to_string(), SinkConfig::new("web"));
    let mut register = SinkRegister::new();

    let error = register
        .register_config_sinks_with_resources(&config, &mut RuntimeResources::new())
        .unwrap_err();

    assert!(matches!(error, rubo_engine::SinkError::KindMissing { .. }));
}

#[test]
fn register_channel_sink_returns_error_when_resource_is_missing() {
    let mut register = SinkRegister::new();
    let error = register
        .register_with_resources("web", ChannelSinkFactory, &mut RuntimeResources::new())
        .unwrap_err();

    assert!(matches!(
        error,
        rubo_engine::SinkError::ResourceMissing { .. }
    ));
}

fn success_output() -> Output {
    Output::success(
        OutputRoute::new(
            Some("binding"),
            "source",
            "frame",
            Some("func"),
            vec!["web".to_string()],
        ),
        OutputTiming::new(1, 2),
        rubo_engine::FuncResult::new(json!({ "value": 1 })),
    )
}

fn block_on<F>(mut future: F) -> F::Output
where
    F: std::future::Future,
{
    let waker = noop_waker();
    let mut context = std::task::Context::from_waker(&waker);
    let mut future = unsafe { std::pin::Pin::new_unchecked(&mut future) };
    loop {
        match future.as_mut().poll(&mut context) {
            std::task::Poll::Ready(output) => return output,
            std::task::Poll::Pending => panic!("test future unexpectedly pending"),
        }
    }
}

fn noop_waker() -> std::task::Waker {
    unsafe { std::task::Waker::from_raw(noop_raw_waker()) }
}

fn noop_raw_waker() -> std::task::RawWaker {
    std::task::RawWaker::new(std::ptr::null(), &NOOP_RAW_WAKER_VTABLE)
}

static NOOP_RAW_WAKER_VTABLE: std::task::RawWakerVTable =
    std::task::RawWakerVTable::new(noop_clone, noop_wake, noop_wake, noop_drop);

unsafe fn noop_clone(_: *const ()) -> std::task::RawWaker {
    noop_raw_waker()
}

unsafe fn noop_wake(_: *const ()) {}

unsafe fn noop_drop(_: *const ()) {}
