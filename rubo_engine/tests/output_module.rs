use rubo_engine::{
    FuncResult, Output, OutputError, OutputErrorKind, OutputRoute, OutputState, OutputTiming,
};
use serde_json::json;

#[test]
fn output_success_stores_route_timing_and_func_result() {
    let route = OutputRoute::new(
        Some("binding"),
        "camera",
        "frame",
        Some("detect"),
        vec!["web".to_string()],
    );
    let timing = OutputTiming::new(10, 25);
    let result = FuncResult::new(json!({ "ok": true })).description("done");

    let output = Output::success(route, timing, result);

    assert_eq!(output.route().binding_id(), Some("binding"));
    assert_eq!(output.route().source_id(), "camera");
    assert_eq!(output.route().key(), "frame");
    assert_eq!(output.route().func_id(), Some("detect"));
    assert_eq!(output.route().sink_ids(), &["web".to_string()]);
    assert_eq!(output.timing().started_at_ms(), 10);
    assert_eq!(output.timing().finished_at_ms(), 25);
    assert_eq!(output.timing().duration_ms(), 15);
    match output.state() {
        OutputState::Success(result) => assert_eq!(result.description_ref(), "done"),
        OutputState::Error(_) => panic!("unexpected error output"),
    }
}

#[test]
fn output_error_stores_error_state() {
    let route = OutputRoute::new(
        None::<String>,
        "camera",
        "frame",
        None::<String>,
        Vec::new(),
    );
    let timing = OutputTiming::new(30, 20);
    let error = OutputError::new(OutputErrorKind::Dispatch, "binding missing");

    let output = Output::error(route, timing, error);

    assert_eq!(output.timing().duration_ms(), 0);
    match output.state() {
        OutputState::Success(_) => panic!("unexpected success output"),
        OutputState::Error(error) => {
            assert_eq!(error.kind(), &OutputErrorKind::Dispatch);
            assert_eq!(error.message(), "binding missing");
        }
    }
}
