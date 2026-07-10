use async_trait::async_trait;
use rubo_engine::{
    Output, OutputState, Sink, SinkError,
    config::{ConfigAccess, SinkConfig},
};

#[rubo_engine::sink(id = "uart")]
#[derive(Default)]
pub struct UartSink;

#[async_trait]
impl Sink for UartSink {
    async fn handle(&self, output: &Output, sink_config: &SinkConfig) -> Result<(), SinkError> {
        let value = output_value(output);
        #[cfg(feature = "hardware")]
        {
            use std::io::Write;
            let serial = sink_config.get_or("serial", "/dev/ttyV0".to_string())?;
            let baud = sink_config.get_or("baud", 9600_u32)?;
            let _data_bit = sink_config.get_or::<u8>("data_bit", 8)?;
            let _stop_bit = sink_config.get_or::<u8>("stop_bit", 1)?;
            let _parity_bit = sink_config.get_or::<bool>("parity_bit", false)?;
            let mut port =
                serialport::new(serial, baud)
                    .open()
                    .map_err(|error| SinkError::Handle {
                        message: error.to_string(),
                    })?;
            port.write_all(value.as_bytes())
                .and_then(|_| port.write_all(b"\n"))
                .map_err(|error| SinkError::Handle {
                    message: error.to_string(),
                })?;
            return Ok(());
        }

        #[cfg(not(feature = "hardware"))]
        {
            let _ = sink_config;
            println!("[uart] {value}");
            Ok(())
        }
    }
}

#[rubo_engine::sink(id = "gpio")]
#[derive(Default)]
pub struct GpioSink;

#[async_trait]
impl Sink for GpioSink {
    async fn handle(&self, output: &Output, sink_config: &SinkConfig) -> Result<(), SinkError> {
        let active_low = sink_config.get_or("active_low", true)?;
        let run_pin = sink_config.get_or("run_pin", 27_u8)?;
        let signals = sink_config
            .get_or::<serde_json::Value>("signals", serde_json::json!({ "color": 17, "qr": 22 }))?;
        println!(
            "[gpio] active_low={active_low} run_pin={run_pin} signals={} value={}",
            signals,
            output_value(output)
        );
        Ok(())
    }
}

fn output_value(output: &Output) -> String {
    match output.state() {
        OutputState::Success(result) => result
            .value()
            .get("value")
            .and_then(serde_json::Value::as_str)
            .map(ToString::to_string)
            .unwrap_or_else(|| result.value().to_string()),
        OutputState::Error(error) => format!("{:?}: {}", error.kind(), error.message()),
    }
}
