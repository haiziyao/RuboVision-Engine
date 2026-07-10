use serde_json::Value;

#[derive(Debug, Clone, PartialEq)]
pub struct Message {
    key: String,
    description: String,
    started_at_ms: Option<u64>,
    payload: Value,
}

impl Message {
    pub fn new(key: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            description: String::new(),
            started_at_ms: None,
            payload: Value::Null,
        }
    }

    pub fn key(&self) -> &str {
        &self.key
    }

    pub fn description_ref(&self) -> &str {
        &self.description
    }

    pub fn started_at_ms_ref(&self) -> Option<u64> {
        self.started_at_ms
    }

    pub fn payload_ref(&self) -> &Value {
        &self.payload
    }

    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    pub fn started_at_ms(mut self, started_at_ms: u64) -> Self {
        self.started_at_ms = Some(started_at_ms);
        self
    }

    pub fn payload(mut self, payload: Value) -> Self {
        self.payload = payload;
        self
    }
}
