use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::config::{AppConfig, BindingsConfig, DevicesConfig, FunctionsConfig, WebConfig};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeConfig {
    pub app: AppConfig,
    pub message: MessageConfig,
    pub bindings: BindingsConfig,
    pub devices: DevicesConfig,
    pub functions: FunctionsConfig,
}

impl RuntimeConfig {
    pub fn validate(&self) -> Result<(), config::ConfigError> {
        validate_unique(
            self.devices
                .list
                .iter()
                .map(|device| device.device_id.as_str()),
            "device id",
        )?;
        validate_unique(
            self.functions
                .entries
                .iter()
                .map(|function| function.function_id.as_str()),
            "function id",
        )?;
        validate_unique(
            self.bindings
                .uart_source
                .iter()
                .map(|binding| binding.source_key),
            "UART source key",
        )?;
        validate_unique(
            self.bindings
                .debug_source
                .iter()
                .map(|binding| binding.source_key.as_str()),
            "debug source key",
        )?;

        let device_ids: HashSet<&str> = self
            .devices
            .list
            .iter()
            .map(|device| device.device_id.as_str())
            .collect();
        let function_ids: HashSet<&str> = self
            .functions
            .entries
            .iter()
            .map(|function| function.function_id.as_str())
            .collect();

        for binding in self.bindings.iter_task_bindings() {
            if binding.device_id != "none" && !device_ids.contains(binding.device_id) {
                return Err(config::ConfigError::Message(format!(
                    "binding `{}` references unknown device `{}`",
                    binding.task_id, binding.device_id
                )));
            }
            if !function_ids.contains(binding.function_id) {
                return Err(config::ConfigError::Message(format!(
                    "binding `{}` references unknown function `{}`",
                    binding.task_id, binding.function_id
                )));
            }
        }

        for function in &self.functions.entries {
            if let Some(signal) = function.returns.gpio.as_deref()
                && !self.message.gpio.signals.contains_key(signal)
            {
                return Err(config::ConfigError::Message(format!(
                    "function `{}` references unknown GPIO signal `{signal}`",
                    function.function_id
                )));
            }
        }

        Ok(())
    }
}

fn validate_unique<T>(
    items: impl IntoIterator<Item = T>,
    label: &str,
) -> Result<(), config::ConfigError>
where
    T: Eq + std::hash::Hash + std::fmt::Display,
{
    let mut seen = HashSet::new();
    for item in items {
        if !seen.insert(item.to_string()) {
            return Err(config::ConfigError::Message(format!(
                "duplicate {label} `{item}`"
            )));
        }
    }
    Ok(())
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MessageConfig {
    pub web: WebConfig,
    pub uart: UartConfig,
    pub gpio: GpioConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct UartConfig {
    pub on: bool,
    pub serial: String,
    pub baud: u32,
    pub data_bit: u8,
    pub stop_bit: u8,
    pub parity_bit: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct GpioConfig {
    pub on: bool,
    pub active_low: bool,
    pub run_pin: u8,
    pub signals: std::collections::HashMap<String, u8>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ColorDetectParams {
    pub debug_model: bool,
    pub loop_count: i32,
    pub radius_ratio: f64,
    pub detect_area_access_rate: f64,
    pub color_ranges: Vec<ColorRangeConfig>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ColorRangeConfig {
    pub name: String,
    pub hsv: [i32; 6],
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct QrDetectParams {
    pub debug_model: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct CrossDetectParams {}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DebugParams {
    pub message: String,
}
