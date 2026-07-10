use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use super::ConfigAccess;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct FuncConfig {
    id: String,
    #[serde(flatten)]
    values: Map<String, Value>,
}

impl FuncConfig {
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            values: Map::new(),
        }
    }

    pub fn id(&self) -> &str {
        &self.id
    }
}

impl ConfigAccess for FuncConfig {
    fn values(&self) -> &Map<String, Value> {
        &self.values
    }

    fn values_mut(&mut self) -> &mut Map<String, Value> {
        &mut self.values
    }
}
