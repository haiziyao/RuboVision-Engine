use anyhow::{Context, Result, anyhow};
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

impl FunctionEntryConfig {
    pub fn legacy_args(&self) -> Result<Vec<String>> {
        let mut args = Vec::new();
        let table = self
            .params
            .as_table()
            .ok_or_else(|| anyhow!("function `{}` params must be a table", self.function_id))?;

        for (key, value) in table {
            if key == "color_ranges" {
                let ranges = value.as_array().ok_or_else(|| {
                    anyhow!(
                        "function `{}` parameter `color_ranges` must be an array",
                        self.function_id
                    )
                })?;
                for range in ranges {
                    let range = range.as_table().ok_or_else(|| {
                        anyhow!(
                            "function `{}` color range must be a table",
                            self.function_id
                        )
                    })?;
                    let name = range
                        .get("name")
                        .and_then(toml::Value::as_str)
                        .context("color range missing string `name`")?;
                    let hsv = range
                        .get("hsv")
                        .and_then(toml::Value::as_array)
                        .context("color range missing array `hsv`")?;
                    let values = hsv
                        .iter()
                        .map(value_text)
                        .collect::<Result<Vec<_>>>()?
                        .join(",");
                    args.push(format!("color.{name}={values}"));
                }
                continue;
            }

            args.push(format!("{key}={}", value_text(value)?));
        }

        Ok(args)
    }
}

fn empty_params() -> toml::Value {
    toml::Value::Table(toml::map::Map::new())
}

fn value_text(value: &toml::Value) -> Result<String> {
    match value {
        toml::Value::String(value) => Ok(value.clone()),
        toml::Value::Integer(value) => Ok(value.to_string()),
        toml::Value::Float(value) => Ok(value.to_string()),
        toml::Value::Boolean(value) => Ok(value.to_string()),
        _ => Err(anyhow!("parameter value must be a scalar")),
    }
}
