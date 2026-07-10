use rubo_engine::{
    DispatchErrorKind, DispatchMessage, DispatchOutput, Message,
    config::{BindingConfig, DeviceConfig, FuncConfig, RuboConfig, SinkConfig},
    dispatch,
};
use serde_json::json;

#[test]
fn dispatch_builds_task_request_when_one_binding_matches() {
    let config = config_with_valid_refs(
        BindingConfig::new("detect_frame")
            .source("camera", "frame")
            .func("detect")
            .device("camera_device")
            .sink("web"),
    );
    let message = Message::new("frame").payload(json!({ "path": "frame.jpg" }));

    let output = dispatch(&config, DispatchMessage::new("camera", message));

    let task = match output {
        DispatchOutput::Task(task) => task,
        DispatchOutput::Error(error) => panic!("unexpected dispatch error: {:?}", error.kind()),
    };

    assert_eq!(task.binding_id(), "detect_frame");
    assert_eq!(task.source_id(), "camera");
    assert_eq!(task.key(), "frame");
    assert_eq!(task.func_id(), "detect");
    assert_eq!(task.message().payload_ref()["path"], "frame.jpg");
    assert_eq!(task.device_ids(), &["camera_device".to_string()]);
    assert_eq!(task.sink_ids(), &["web".to_string()]);
}

#[test]
fn dispatch_keeps_binding_task_when_binding_func_is_missing() {
    let config = config_with_binding(
        BindingConfig::new("detect_frame")
            .source("camera", "frame")
            .func("missing_func"),
    );

    let output = dispatch(
        &config,
        DispatchMessage::new("camera", Message::new("frame")),
    );

    let task = match output {
        DispatchOutput::Task(task) => task,
        DispatchOutput::Error(error) => panic!("unexpected dispatch error: {:?}", error.kind()),
    };

    assert_eq!(task.binding_id(), "detect_frame");
    assert_eq!(task.func_id(), "missing_func");
}

#[test]
fn dispatch_keeps_binding_task_when_binding_device_is_missing() {
    let mut config = RuboConfig::default();
    config
        .funcs_mut()
        .insert("detect".to_string(), FuncConfig::new("detect"));
    config.bindings_mut().insert(
        "detect_frame".to_string(),
        BindingConfig::new("detect_frame")
            .source("camera", "frame")
            .func("detect")
            .device("missing_camera"),
    );

    let output = dispatch(
        &config,
        DispatchMessage::new("camera", Message::new("frame")),
    );

    let task = match output {
        DispatchOutput::Task(task) => task,
        DispatchOutput::Error(error) => panic!("unexpected dispatch error: {:?}", error.kind()),
    };

    assert_eq!(task.binding_id(), "detect_frame");
    assert_eq!(task.device_ids(), &["missing_camera".to_string()]);
}

#[test]
fn dispatch_keeps_binding_task_when_binding_sink_is_missing() {
    let mut config = RuboConfig::default();
    config
        .funcs_mut()
        .insert("detect".to_string(), FuncConfig::new("detect"));
    config.devices_mut().insert(
        "camera_device".to_string(),
        DeviceConfig::new("camera_device", "camera"),
    );
    config.bindings_mut().insert(
        "detect_frame".to_string(),
        BindingConfig::new("detect_frame")
            .source("camera", "frame")
            .func("detect")
            .device("camera_device")
            .sink("missing_sink"),
    );

    let output = dispatch(
        &config,
        DispatchMessage::new("camera", Message::new("frame")),
    );

    let task = match output {
        DispatchOutput::Task(task) => task,
        DispatchOutput::Error(error) => panic!("unexpected dispatch error: {:?}", error.kind()),
    };

    assert_eq!(task.binding_id(), "detect_frame");
    assert_eq!(task.sink_ids(), &["missing_sink".to_string()]);
}

#[test]
fn dispatch_returns_binding_not_found_error_when_no_binding_matches() {
    let config = config_with_binding(
        BindingConfig::new("other")
            .source("camera", "other")
            .func("detect"),
    );
    let message = Message::new("frame").payload(json!({ "value": 1 }));

    let output = dispatch(&config, DispatchMessage::new("camera", message));

    let error = match output {
        DispatchOutput::Task(task) => panic!("unexpected task request: {}", task.binding_id()),
        DispatchOutput::Error(error) => error,
    };

    assert_eq!(error.kind(), &DispatchErrorKind::BindingNotFound);
    assert_eq!(error.source_id(), "camera");
    assert_eq!(error.key(), "frame");
    assert_eq!(error.message().payload_ref()["value"], 1);
}

#[test]
fn dispatch_returns_binding_conflict_error_when_multiple_bindings_match() {
    let mut config = RuboConfig::default();
    config.bindings_mut().insert(
        "one".to_string(),
        BindingConfig::new("one")
            .source("camera", "frame")
            .func("detect"),
    );
    config.bindings_mut().insert(
        "two".to_string(),
        BindingConfig::new("two")
            .source("camera", "frame")
            .func("save"),
    );

    let output = dispatch(
        &config,
        DispatchMessage::new("camera", Message::new("frame")),
    );

    let error = match output {
        DispatchOutput::Task(task) => panic!("unexpected task request: {}", task.binding_id()),
        DispatchOutput::Error(error) => error,
    };

    assert_eq!(error.kind(), &DispatchErrorKind::BindingConflict);
    assert_eq!(error.source_id(), "camera");
    assert_eq!(error.key(), "frame");
}

#[test]
fn dispatch_returns_config_invalid_error_when_matched_binding_has_empty_function() {
    let config = config_with_binding(BindingConfig::new("broken").source("camera", "frame"));

    let output = dispatch(
        &config,
        DispatchMessage::new("camera", Message::new("frame")),
    );

    let error = match output {
        DispatchOutput::Task(task) => panic!("unexpected task request: {}", task.binding_id()),
        DispatchOutput::Error(error) => error,
    };

    assert_eq!(error.kind(), &DispatchErrorKind::ConfigInvalid);
    assert_eq!(error.source_id(), "camera");
    assert_eq!(error.key(), "frame");
}

fn config_with_binding(binding: BindingConfig) -> RuboConfig {
    let mut config = RuboConfig::default();
    config
        .bindings_mut()
        .insert(binding.id().to_string(), binding);
    config
}

fn config_with_valid_refs(binding: BindingConfig) -> RuboConfig {
    let mut config = config_with_binding(binding);
    config
        .funcs_mut()
        .insert("detect".to_string(), FuncConfig::new("detect"));
    config.devices_mut().insert(
        "camera_device".to_string(),
        DeviceConfig::new("camera_device", "camera"),
    );
    config
        .sinks_mut()
        .insert("web".to_string(), SinkConfig::new("web"));
    config
}
