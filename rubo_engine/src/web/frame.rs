use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    Output, OutputState, RuntimeOutput, SinkRouteResult, SinkRouteState, output::OutputErrorKind,
};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct WebOutputFrame {
    id: u64,
    created_at_ms: u64,
    route: WebOutputRoute,
    timing: WebOutputTiming,
    state: WebOutputState,
    sink_results: Vec<WebSinkRouteResult>,
}

impl WebOutputFrame {
    pub fn from_output(id: u64, created_at_ms: u64, output: &Output) -> Self {
        Self {
            id,
            created_at_ms,
            route: WebOutputRoute::from_output(output),
            timing: WebOutputTiming::from_output(output),
            state: WebOutputState::from_output(output),
            sink_results: Vec::new(),
        }
    }

    pub fn from_runtime_output(id: u64, created_at_ms: u64, output: &RuntimeOutput) -> Self {
        Self {
            id,
            created_at_ms,
            route: WebOutputRoute::from_output(output.output()),
            timing: WebOutputTiming::from_output(output.output()),
            state: WebOutputState::from_output(output.output()),
            sink_results: output
                .sink_results()
                .iter()
                .map(WebSinkRouteResult::from_sink_route_result)
                .collect(),
        }
    }

    pub fn id(&self) -> u64 {
        self.id
    }

    pub fn created_at_ms(&self) -> u64 {
        self.created_at_ms
    }

    pub fn route(&self) -> &WebOutputRoute {
        &self.route
    }

    pub fn timing(&self) -> &WebOutputTiming {
        &self.timing
    }

    pub fn state(&self) -> &WebOutputState {
        &self.state
    }

    pub fn sink_results(&self) -> &[WebSinkRouteResult] {
        &self.sink_results
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct WebOutputRoute {
    binding_id: Option<String>,
    source_id: String,
    key: String,
    function_id: Option<String>,
    sink_ids: Vec<String>,
}

impl WebOutputRoute {
    fn from_output(output: &Output) -> Self {
        Self {
            binding_id: output.route().binding_id().map(ToString::to_string),
            source_id: output.route().source_id().to_string(),
            key: output.route().key().to_string(),
            function_id: output.route().func_id().map(ToString::to_string),
            sink_ids: output.route().sink_ids().to_vec(),
        }
    }

    pub fn binding_id(&self) -> Option<&str> {
        self.binding_id.as_deref()
    }

    pub fn source_id(&self) -> &str {
        &self.source_id
    }

    pub fn key(&self) -> &str {
        &self.key
    }

    pub fn function_id(&self) -> Option<&str> {
        self.function_id.as_deref()
    }

    pub fn sink_ids(&self) -> &[String] {
        &self.sink_ids
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct WebOutputTiming {
    started_at_ms: u64,
    finished_at_ms: u64,
    duration_ms: u64,
}

impl WebOutputTiming {
    fn from_output(output: &Output) -> Self {
        Self {
            started_at_ms: output.timing().started_at_ms(),
            finished_at_ms: output.timing().finished_at_ms(),
            duration_ms: output.timing().duration_ms(),
        }
    }

    pub fn started_at_ms(&self) -> u64 {
        self.started_at_ms
    }

    pub fn finished_at_ms(&self) -> u64 {
        self.finished_at_ms
    }

    pub fn duration_ms(&self) -> u64 {
        self.duration_ms
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum WebOutputState {
    Success { value: Value },
    Error { kind: String, message: String },
}

impl WebOutputState {
    fn from_output(output: &Output) -> Self {
        match output.state() {
            OutputState::Success(result) => Self::Success {
                value: result.value().clone(),
            },
            OutputState::Error(error) => Self::Error {
                kind: output_error_kind(error.kind()).to_string(),
                message: error.message().to_string(),
            },
        }
    }

    pub fn is_error(&self) -> bool {
        matches!(self, Self::Error { .. })
    }

    pub fn state_name(&self) -> &'static str {
        match self {
            Self::Success { .. } => "success",
            Self::Error { .. } => "error",
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct WebSinkRouteResult {
    sink_id: String,
    state: String,
    error: Option<String>,
}

impl WebSinkRouteResult {
    fn from_sink_route_result(result: &SinkRouteResult) -> Self {
        match result.state() {
            SinkRouteState::Handled => Self {
                sink_id: result.sink_id().to_string(),
                state: "handled".to_string(),
                error: None,
            },
            SinkRouteState::SinkNotFound => Self {
                sink_id: result.sink_id().to_string(),
                state: "sink_not_found".to_string(),
                error: None,
            },
            SinkRouteState::SinkConfigNotFound => Self {
                sink_id: result.sink_id().to_string(),
                state: "sink_config_not_found".to_string(),
                error: None,
            },
            SinkRouteState::HandleError(error) => Self {
                sink_id: result.sink_id().to_string(),
                state: "handle_error".to_string(),
                error: Some(error.to_string()),
            },
        }
    }

    pub fn sink_id(&self) -> &str {
        &self.sink_id
    }

    pub fn state(&self) -> &str {
        &self.state
    }

    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }
}

fn output_error_kind(kind: &OutputErrorKind) -> &'static str {
    match kind {
        OutputErrorKind::Dispatch => "dispatch",
        OutputErrorKind::Function => "function",
        OutputErrorKind::Runtime => "runtime",
    }
}
