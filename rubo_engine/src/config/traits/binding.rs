use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct BindingConfig {
    #[serde(default)]
    id: String,
    source: BindingSourceConfig,
    #[serde(rename = "function")]
    func: String,
    #[serde(default)]
    devices: Vec<String>,
    #[serde(default)]
    sinks: Vec<String>,
    #[serde(default)]
    debug: bool,
}

impl BindingConfig {
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            source: BindingSourceConfig::new("", ""),
            func: String::new(),
            devices: Vec::new(),
            sinks: Vec::new(),
            debug: false,
        }
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn source(mut self, id: impl Into<String>, event: impl Into<String>) -> Self {
        self.source = BindingSourceConfig::new(id, event);
        self
    }

    pub fn func(mut self, func: impl Into<String>) -> Self {
        self.func = func.into();
        self
    }

    pub fn device(mut self, device: impl Into<String>) -> Self {
        self.devices.push(device.into());
        self
    }

    pub fn sink(mut self, sink: impl Into<String>) -> Self {
        self.sinks.push(sink.into());
        self
    }

    pub fn debug(mut self, debug: bool) -> Self {
        self.debug = debug;
        self
    }

    pub fn source_ref(&self) -> &BindingSourceConfig {
        &self.source
    }

    pub fn func_ref(&self) -> &str {
        &self.func
    }

    pub fn devices(&self) -> &[String] {
        &self.devices
    }

    pub fn sinks(&self) -> &[String] {
        &self.sinks
    }

    pub fn debug_enabled(&self) -> bool {
        self.debug
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct BindingSourceConfig {
    id: String,
    event: String,
}

impl BindingSourceConfig {
    pub fn new(id: impl Into<String>, event: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            event: event.into(),
        }
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn event(&self) -> &str {
        &self.event
    }
}
