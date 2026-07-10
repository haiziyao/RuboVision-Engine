use std::sync::Arc;

use axum::{Json, extract::State};
use rubo_engine::{
    FuncResult, Output, OutputError, OutputErrorKind, OutputRoute, OutputTiming, Sink, WebConfig,
    WebError, WebErrorKind, WebEvent, WebEventKind, WebHistory, WebHub, WebInterface,
    WebOutputFrame, WebOutputState, WebResponse, WebSink, WebState, build_router,
    config::{AppConfig, RuboConfig, SinkConfig, SourceConfig},
    web::api::{
        WebDebugTriggerRequest, debug_trigger, health, index_html, remove_source_api,
        runtime_control_status, runtime_restart, runtime_start, runtime_stop, runtime_summary,
    },
};
use serde_json::json;
use tokio::sync::{RwLock, mpsc};

#[test]
fn web_index_html_contains_new_three_page_workstation() {
    let html = index_html();
    assert!(html.contains("RuboEngine 运行中心"));
    assert!(html.contains("执行总览"));
    assert!(html.contains("历史信息"));
    assert!(html.contains("配置管理"));
    assert!(html.contains(r#"data-page="overview""#));
    assert!(html.contains(r#"data-page="history""#));
    assert!(html.contains(r#"data-page="config""#));
    assert!(!html.contains(r#"data-page="debug""#));
    assert!(html.contains("最新执行结果"));
    assert!(html.contains("调试触发"));
    assert!(html.contains("本次输出不包含图像"));
    assert!(html.contains(r#"id="clear-current""#));
    assert!(html.contains("clearCurrentDisplay"));
    assert!(html.contains(r#"id="current-result-value""#));
    assert!(html.contains("resultValue"));
    assert!(html.contains("<th>Value</th>"));
    assert!(!html.contains("瑙嗚"));
    assert!(html.contains("findOutputImage"));
}

#[test]
fn web_index_html_discovers_routes_and_updates_one_config_instance() {
    let html = index_html();
    assert!(html.contains("loadInterface"));
    assert!(html.contains("EventSource"));
    assert!(html.contains("应用到内存"));
    assert!(html.contains("method: \"PUT\""));
    assert!(html.contains("applySelectedConfig"));
    assert!(html.contains(r#"id="config-field-list""#));
    assert!(html.contains(r#"id="config-json-toggle""#));
    assert!(!html.contains(r#"id="config-edit-fields""#));
    assert!(html.contains("renderConfigFields"));
    assert!(html.contains("renderConfigTreeNode"));
    assert!(html.contains("updateConfigDraftValue"));
    assert!(html.contains("addConfigArrayItem"));
}

#[test]
fn web_config_defaults_to_confirmed_port_and_routes() {
    let config = WebConfig::default();
    assert!(config.enabled());
    assert_eq!(config.host(), "127.0.0.1");
    assert_eq!(config.port(), 3888);
    assert_eq!(config.history_limit(), 500);
    assert_eq!(config.routes().runtime_events(), "/api/runtime/events");
    assert_eq!(config.routes().runtime_start(), "/api/runtime/start");
    assert_eq!(config.routes().runtime_stop(), "/api/runtime/stop");
    assert_eq!(config.routes().runtime_restart(), "/api/runtime/restart");
    assert_eq!(
        config.routes().runtime_control_status(),
        "/api/runtime/control/status"
    );
    assert_eq!(config.routes().outputs_latest(), "/api/outputs/latest");
    assert_eq!(config.routes().config_save(), "/api/config/save");
}

#[test]
fn web_response_wraps_success_and_error() {
    let ok = WebResponse::ok("ready".to_string());
    let error: WebResponse<String> = WebResponse::error(WebError::config("bad config"));
    assert!(ok.is_ok());
    assert_eq!(ok.data(), Some(&"ready".to_string()));
    assert!(!error.is_ok());
    assert_eq!(error.error_ref().unwrap().kind(), &WebErrorKind::Config);
    assert_eq!(error.error_ref().unwrap().message(), "bad config");
}

#[test]
fn web_output_frame_reads_output_fields() {
    let frame = WebOutputFrame::from_output(7, 20, &success_output());
    assert_eq!(frame.id(), 7);
    assert_eq!(frame.created_at_ms(), 20);
    assert_eq!(frame.route().binding_id(), Some("binding"));
    assert_eq!(frame.route().source_id(), "source");
    assert_eq!(frame.route().key(), "frame");
    assert_eq!(frame.route().function_id(), Some("func"));
    assert_eq!(frame.route().sink_ids(), &["web".to_string()]);
    assert_eq!(frame.timing().started_at_ms(), 1);
    assert_eq!(frame.timing().finished_at_ms(), 3);
    assert_eq!(frame.timing().duration_ms(), 2);
    assert!(matches!(frame.state(), WebOutputState::Success { .. }));
    assert_eq!(frame.state().state_name(), "success");
    assert!(frame.sink_results().is_empty());
}

#[test]
fn web_history_limits_latest_and_counts_errors() {
    let mut history = WebHistory::new(2);
    history.push(WebOutputFrame::from_output(1, 10, &success_output()));
    history.push(WebOutputFrame::from_output(2, 20, &error_output()));
    history.push(WebOutputFrame::from_output(3, 30, &success_output()));
    let latest = history.latest(10);
    assert_eq!(history.count(), 2);
    assert_eq!(history.error_count(), 1);
    assert_eq!(history.last_output_at_ms(), Some(30));
    assert_eq!(
        latest.iter().map(WebOutputFrame::id).collect::<Vec<_>>(),
        vec![3, 2]
    );
    assert!(history.get(1).is_none());
    assert_eq!(history.get(2).unwrap().id(), 2);
}

#[tokio::test]
async fn web_hub_broadcasts_events_to_subscribers() {
    let hub = WebHub::new(8);
    let mut receiver = hub.subscribe();
    hub.publish(WebEvent::config_updated("saved"));
    let event = receiver.recv().await.unwrap();
    assert_eq!(event.kind(), &WebEventKind::ConfigUpdated);
    assert_eq!(event.kind().event_name(), "config_updated");
    assert_eq!(event.message(), Some("saved"));
}

#[test]
fn web_interface_lists_confirmed_pages_and_routes() {
    let interface = WebInterface::from_config(&WebConfig::default());
    assert_eq!(interface.name(), "RuboEngine");
    assert_eq!(interface.version(), 1);
    assert!(interface.pages().contains(&"runtime".to_string()));
    assert!(interface.pages().contains(&"outputs".to_string()));
    assert!(interface.pages().contains(&"config".to_string()));
    assert!(
        interface
            .routes()
            .iter()
            .any(|route| route.name() == "debug_trigger")
    );
    assert!(
        interface
            .routes()
            .iter()
            .any(|route| route.name() == "runtime_restart")
    );
}

#[test]
fn build_router_accepts_web_state() {
    let state = WebState::new(".", AppConfig::default(), RuboConfig::default());
    let _router = build_router(state);
}

#[tokio::test]
async fn web_remove_source_api_deletes_source_from_memory() {
    let mut config = RuboConfig::default();
    config.sources_mut().insert(
        "input".to_string(),
        SourceConfig::new("input").kind("manual"),
    );
    let state = WebState::new(".", AppConfig::default(), config);
    let Json(response) = remove_source_api(
        State(state.clone()),
        axum::extract::Path("input".to_string()),
    )
    .await;
    assert!(response.is_ok());
    assert!(
        !state
            .rubo_config()
            .read()
            .expect("test config lock poisoned")
            .sources()
            .contains_key("input")
    );
}

#[tokio::test]
async fn web_runtime_control_api_returns_error_without_runtime_control() {
    let state = WebState::new(".", AppConfig::default(), RuboConfig::default());
    let Json(health) = health(State(state.clone())).await;
    let Json(summary) = runtime_summary(State(state.clone())).await;
    let Json(start) = runtime_start(State(state.clone())).await;
    let Json(stop) = runtime_stop(State(state.clone())).await;
    let Json(restart) = runtime_restart(State(state.clone())).await;
    let Json(status) = runtime_control_status(State(state)).await;
    assert!(!start.is_ok());
    assert!(!stop.is_ok());
    assert!(!restart.is_ok());
    assert!(
        !serde_json::to_value(health.data().unwrap()).unwrap()["running"]
            .as_bool()
            .unwrap()
    );
    assert!(
        !serde_json::to_value(summary.data().unwrap()).unwrap()["running"]
            .as_bool()
            .unwrap()
    );
    assert_eq!(start.error_ref().unwrap().kind(), &WebErrorKind::Runtime);
    assert!(!status.data().unwrap().running());
}

#[tokio::test]
async fn web_debug_trigger_returns_error_without_source_sender() {
    let state = WebState::new(".", AppConfig::default(), RuboConfig::default());
    let Json(response) = debug_trigger(State(state), Json(debug_trigger_request())).await;

    assert!(!response.is_ok());
    assert_eq!(response.error_ref().unwrap().kind(), &WebErrorKind::Runtime);
    assert_eq!(
        response.error_ref().unwrap().message(),
        "web source sender is not connected"
    );
}

#[tokio::test]
async fn web_debug_trigger_sends_message_when_source_sender_is_connected() {
    let mut state = WebState::new(".", AppConfig::default(), RuboConfig::default());
    let (sender, mut receiver) = mpsc::channel(1);
    state.set_source_sender(sender);

    let Json(response) = debug_trigger(State(state), Json(debug_trigger_request())).await;

    assert!(response.is_ok());
    assert_eq!(response.data(), Some(&"debug_web".to_string()));
    let message = receiver.recv().await.expect("debug message");
    assert_eq!(message.key(), "debug_web");
    assert_eq!(message.description_ref(), "manual trigger");
    assert_eq!(message.payload_ref()["value"], 1);
}

#[tokio::test]
async fn web_sink_writes_history_and_publishes_output_event() {
    let history = Arc::new(RwLock::new(WebHistory::new(10)));
    let hub = WebHub::new(10);
    let mut receiver = hub.subscribe();
    let sink = WebSink::new(history.clone(), hub);
    sink.handle(&success_output(), &SinkConfig::new("web"))
        .await
        .unwrap();
    let event = receiver.recv().await.unwrap();
    assert_eq!(event.kind(), &WebEventKind::Output);
    assert_eq!(event.frame().unwrap().id(), 1);
    let history = history.read().await;
    assert_eq!(history.count(), 1);
    assert_eq!(history.latest(1)[0].route().source_id(), "source");
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
        OutputTiming::new(1, 3),
        FuncResult::new(json!({ "value": 1 })),
    )
}

fn error_output() -> Output {
    Output::error(
        OutputRoute::new(
            Some("binding"),
            "source",
            "frame",
            Some("func"),
            vec!["web".to_string()],
        ),
        OutputTiming::new(1, 3),
        OutputError::new(OutputErrorKind::Runtime, "failed"),
    )
}

fn debug_trigger_request() -> WebDebugTriggerRequest {
    serde_json::from_value(json!({
        "binding_id": "debug_web",
        "description": "manual trigger",
        "payload": { "value": 1 }
    }))
    .expect("debug request")
}
