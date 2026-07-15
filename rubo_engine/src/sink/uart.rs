use std::sync::Arc;

use async_trait::async_trait;

use crate::{
    Output, OutputState, Sink, SinkError, SinkFactory, config::SinkConfig,
    source::uart::uart_settings,
};

#[cfg(all(feature = "hardware", target_os = "linux"))]
use crate::source::uart::{UartHandle, open_uart};

pub struct UartSink {
    #[cfg(all(feature = "hardware", target_os = "linux"))]
    uart: UartHandle,
    #[cfg(not(all(feature = "hardware", target_os = "linux")))]
    serial: String,
}

impl UartSink {
    fn from_config(config: &SinkConfig) -> Result<Self, SinkError> {
        let settings = uart_settings(config).map_err(|error| SinkError::Config {
            message: error.to_string(),
        })?;
        #[cfg(all(feature = "hardware", target_os = "linux"))]
        let uart = open_uart(settings).map_err(|message| SinkError::Handle { message })?;
        Ok(Self {
            #[cfg(all(feature = "hardware", target_os = "linux"))]
            uart,
            #[cfg(not(all(feature = "hardware", target_os = "linux")))]
            serial: settings.serial().to_string(),
        })
    }
}

#[async_trait]
impl Sink for UartSink {
    async fn handle(&self, output: &Output, _config: &SinkConfig) -> Result<(), SinkError> {
        let bytes = output_bytes(output);
        #[cfg(all(feature = "hardware", target_os = "linux"))]
        return self
            .uart
            .write(bytes)
            .map_err(|message| SinkError::Handle { message });

        #[cfg(not(all(feature = "hardware", target_os = "linux")))]
        {
            println!(
                "[uart:{}] {}",
                self.serial,
                String::from_utf8_lossy(&bytes).trim_end()
            );
            Ok(())
        }
    }
}

#[derive(Default)]
pub struct UartSinkFactory;

impl SinkFactory for UartSinkFactory {
    fn build(
        &self,
        config: &SinkConfig,
        _resources: &mut crate::RuntimeResources,
    ) -> Result<Arc<dyn Sink>, SinkError> {
        Ok(Arc::new(UartSink::from_config(config)?))
    }
}

fn output_bytes(output: &Output) -> Vec<u8> {
    let text = match output.state() {
        OutputState::Success(result) => {
            value_text(result.value().get("value").unwrap_or(result.value()))
        }
        OutputState::Error(error) => format!("{:?}: {}", error.kind(), error.message()),
    };
    let mut bytes = text.trim_end_matches(['\r', '\n']).as_bytes().to_vec();
    bytes.push(b'\n');
    bytes
}

fn value_text(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(value) => value.clone(),
        _ => value.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use crate::{FuncResult, Output, OutputRoute, OutputTiming};

    use super::*;

    #[test]
    fn uart_sink_encodes_function_value_as_one_line() {
        let output = Output::success(
            OutputRoute::new(None::<String>, "uart", "1", Some("color"), Vec::new()),
            OutputTiming::new(1, 2),
            FuncResult::new(serde_json::json!({ "value": "red", "image": "ignored" })),
        );

        assert_eq!(output_bytes(&output), b"red\n");
    }
}
