use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(default)]
pub struct ReturnTargets {
    pub web: bool,
    pub uart: bool,
    pub gpio: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FunctionEntryConfig {
    pub function_id: String,
    pub returns: ReturnTargets,
    #[serde(default = "empty_params")]
    pub params: toml::Value,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(default)]
pub struct FunctionsConfig {
    pub entries: Vec<FunctionEntryConfig>,
}

fn empty_params() -> toml::Value {
    toml::Value::Table(toml::map::Map::new())
}
