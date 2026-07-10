use crate::FuncResult;

#[derive(Debug, Clone, PartialEq)]
pub struct Output {
    route: OutputRoute,
    timing: OutputTiming,
    state: OutputState,
}

impl Output {
    pub fn success(route: OutputRoute, timing: OutputTiming, result: FuncResult) -> Self {
        Self {
            route,
            timing,
            state: OutputState::Success(result),
        }
    }

    pub fn error(route: OutputRoute, timing: OutputTiming, error: OutputError) -> Self {
        Self {
            route,
            timing,
            state: OutputState::Error(error),
        }
    }

    pub fn route(&self) -> &OutputRoute {
        &self.route
    }

    pub fn timing(&self) -> &OutputTiming {
        &self.timing
    }

    pub fn state(&self) -> &OutputState {
        &self.state
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutputRoute {
    binding_id: Option<String>,
    source_id: String,
    key: String,
    func_id: Option<String>,
    sink_ids: Vec<String>,
}

impl OutputRoute {
    pub fn new(
        binding_id: Option<impl Into<String>>,
        source_id: impl Into<String>,
        key: impl Into<String>,
        func_id: Option<impl Into<String>>,
        sink_ids: Vec<String>,
    ) -> Self {
        Self {
            binding_id: binding_id.map(Into::into),
            source_id: source_id.into(),
            key: key.into(),
            func_id: func_id.map(Into::into),
            sink_ids,
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

    pub fn func_id(&self) -> Option<&str> {
        self.func_id.as_deref()
    }

    pub fn sink_ids(&self) -> &[String] {
        &self.sink_ids
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutputTiming {
    started_at_ms: u64,
    finished_at_ms: u64,
    duration_ms: u64,
}

impl OutputTiming {
    pub fn new(started_at_ms: u64, finished_at_ms: u64) -> Self {
        Self {
            started_at_ms,
            finished_at_ms,
            duration_ms: finished_at_ms.saturating_sub(started_at_ms),
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

#[derive(Debug, Clone, PartialEq)]
pub enum OutputState {
    Success(FuncResult),
    Error(OutputError),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutputError {
    kind: OutputErrorKind,
    message: String,
}

impl OutputError {
    pub fn new(kind: OutputErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }

    pub fn kind(&self) -> &OutputErrorKind {
        &self.kind
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OutputErrorKind {
    Dispatch,
    Function,
    Runtime,
}
