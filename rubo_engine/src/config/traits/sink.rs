use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use super::ConfigAccess;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct SinkConfig {
    id: String,
    #[serde(default)]
    kind: String,
    #[serde(flatten)]
    values: Map<String, Value>,
}

impl SinkConfig {
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            kind: String::new(),
            values: Map::new(),
        }
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn kind(mut self, kind: impl Into<String>) -> Self {
        self.kind = kind.into();
        self
    }

    pub fn kind_ref(&self) -> &str {
        &self.kind
    }
}

impl ConfigAccess for SinkConfig {
    fn values(&self) -> &Map<String, Value> {
        &self.values
    }

    fn values_mut(&mut self) -> &mut Map<String, Value> {
        &mut self.values
    }
}
