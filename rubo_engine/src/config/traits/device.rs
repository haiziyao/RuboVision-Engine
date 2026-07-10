use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use super::ConfigAccess;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct DeviceConfig {
    id: String,
    kind: String,
    #[serde(flatten)]
    values: Map<String, Value>,
}

impl DeviceConfig {
    pub fn new(id: impl Into<String>, kind: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            kind: kind.into(),
            values: Map::new(),
        }
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn kind(&self) -> &str {
        &self.kind
    }
}

impl ConfigAccess for DeviceConfig {
    fn values(&self) -> &Map<String, Value> {
        &self.values
    }

    fn values_mut(&mut self) -> &mut Map<String, Value> {
        &mut self.values
    }
}
