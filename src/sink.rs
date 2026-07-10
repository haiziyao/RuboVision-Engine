use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};

use async_trait::async_trait;
use rubo_engine::{
    FunctionAspect, Output, OutputState, Sink, SinkError, TaskRequest,
    config::{ConfigAccess, SinkConfig},
};

#[rubo_engine::sink(id = "uart")]
#[derive(Default)]
pub struct UartSink;

#[async_trait]
impl Sink for UartSink {
    async fn handle(&self, output: &Output, sink_config: &SinkConfig) -> Result<(), SinkError> {
        let value = output_value(output);
        #[cfg(all(feature = "hardware", target_os = "linux"))]
        {
            let _ = sink_config;
            return crate::source::send_uart_output(value)
                .map_err(|message| SinkError::Handle { message });
        }

        #[cfg(not(all(feature = "hardware", target_os = "linux")))]
        {
            let _ = sink_config;
            println!("[uart] {value}");
            Ok(())
        }
    }
}

#[rubo_engine::sink(id = "gpio")]
#[derive(Clone)]
pub struct GpioSink {
    state: Arc<GpioState>,
}

#[derive(Default)]
pub(crate) struct HeadlessWebSink;

#[async_trait]
impl Sink for HeadlessWebSink {
    async fn handle(&self, _output: &Output, _sink_config: &SinkConfig) -> Result<(), SinkError> {
        Ok(())
    }
}

impl Default for GpioSink {
    fn default() -> Self {
        Self {
            state: Arc::new(GpioState::default()),
        }
    }
}

impl GpioSink {
    pub(crate) fn from_config(config: &SinkConfig) -> Self {
        let active_low = config.get_or("active_low", true).unwrap_or(true);
        let run_pin = config.get_or("run_pin", 27_u8).unwrap_or(27);
        let signals = config
            .get_or::<serde_json::Value>("signals", serde_json::json!({ "color": 17, "qr": 22 }))
            .unwrap_or_else(|_| serde_json::json!({ "color": 17, "qr": 22 }));
        let mut pins = vec![run_pin];
        if let Some(signals) = signals.as_object() {
            pins.extend(
                signals
                    .values()
                    .filter_map(serde_json::Value::as_u64)
                    .filter_map(|pin| u8::try_from(pin).ok()),
            );
        }
        pins.sort_unstable();
        pins.dedup();
        Self {
            state: Arc::new(GpioState::new(active_low, pins)),
        }
    }

    fn begin(&self) {
        if self.state.active_count.fetch_add(1, Ordering::SeqCst) == 0 {
            self.state.set_active(true);
        }
    }

    fn end(&self) {
        let previous = self
            .state
            .active_count
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |count| {
                count.checked_sub(1)
            })
            .unwrap_or(0);
        if previous == 1 {
            self.state.set_active(false);
        }
    }

    #[cfg(test)]
    fn is_active(&self) -> bool {
        self.state.active.load(Ordering::SeqCst)
    }

    #[cfg(test)]
    fn active_count(&self) -> usize {
        self.state.active_count.load(Ordering::SeqCst)
    }
}

#[async_trait]
impl FunctionAspect for GpioSink {
    async fn before(&self, _task: &TaskRequest) {
        self.begin();
    }

    async fn after(&self, _output: &Output) {
        self.end();
    }
}

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

struct GpioState {
    active_low: bool,
    pins: Vec<u8>,
    active: AtomicBool,
    active_count: AtomicUsize,
    #[cfg(all(feature = "hardware", target_os = "linux"))]
    output_pins: std::sync::Mutex<Option<std::collections::HashMap<u8, rppal::gpio::OutputPin>>>,
}

impl GpioState {
    fn new(active_low: bool, pins: Vec<u8>) -> Self {
        Self {
            active_low,
            pins,
            active: AtomicBool::new(false),
            active_count: AtomicUsize::new(0),
            #[cfg(all(feature = "hardware", target_os = "linux"))]
            output_pins: std::sync::Mutex::new(None),
        }
    }

    fn set_active(&self, active: bool) {
        self.active.store(active, Ordering::SeqCst);
        #[cfg(all(feature = "hardware", target_os = "linux"))]
        if let Err(error) = self.write_pins(active) {
            eprintln!("[gpio] write error: {error}");
        }
        let level = if active == self.active_low {
            "low"
        } else {
            "high"
        };
        println!("[gpio] active={active} level={level} pins={:?}", self.pins);
    }

    #[cfg(all(feature = "hardware", target_os = "linux"))]
    fn write_pins(&self, active: bool) -> Result<(), String> {
        let mut output_pins = self
            .output_pins
            .lock()
            .map_err(|_| "gpio output lock poisoned".to_string())?;
        if output_pins.is_none() {
            let gpio = rppal::gpio::Gpio::new().map_err(|error| error.to_string())?;
            let mut initialized = std::collections::HashMap::new();
            for pin in &self.pins {
                initialized.insert(
                    *pin,
                    gpio.get(*pin)
                        .map_err(|error| format!("failed to access GPIO pin {pin}: {error}"))?
                        .into_output(),
                );
            }
            *output_pins = Some(initialized);
        }
        let high = active != self.active_low;
        for output in output_pins
            .as_mut()
            .expect("gpio pins initialized")
            .values_mut()
        {
            if high {
                output.set_high();
            } else {
                output.set_low();
            }
        }
        Ok(())
    }
}

impl Default for GpioState {
    fn default() -> Self {
        Self::new(true, vec![17, 22, 27])
    }
}

impl Drop for GpioState {
    fn drop(&mut self) {
        if self.active.load(Ordering::SeqCst) {
            self.set_active(false);
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gpio_function_state_test() {
        let gpio = GpioSink::default();

        gpio.begin();
        gpio.begin();
        assert!(gpio.is_active());
        assert_eq!(gpio.active_count(), 2);

        gpio.end();
        assert!(gpio.is_active());
        assert_eq!(gpio.active_count(), 1);

        gpio.end();
        assert!(!gpio.is_active());
        assert_eq!(gpio.active_count(), 0);
    }

    #[cfg(all(feature = "hardware", target_os = "linux"))]
    #[test]
    #[ignore = "requires Ubuntu and configured GPIO pins"]
    fn gpio_test() {
        let config = crate::vision::test::load_config().expect("load config");
        let gpio = GpioSink::from_config(&config.sinks()[crate::config::GPIO_SINK_ID]);
        gpio.begin();
        std::thread::sleep(std::time::Duration::from_secs(3));
        gpio.end();
    }
}
