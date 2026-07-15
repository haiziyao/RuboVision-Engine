use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};

use async_trait::async_trait;

use crate::{
    FunctionAspect, Output, Sink, SinkError, SinkFactory, TaskRequest,
    config::{ConfigAccess, SinkConfig},
};

#[derive(Clone)]
pub struct GpioSink {
    state: Arc<GpioState>,
}

impl GpioSink {
    pub fn from_config(config: &SinkConfig) -> Result<Self, SinkError> {
        let active_low = config.get_or("active_low", true)?;
        let chip = config.get_or("chip", 0_u8)?;
        let run_pin = config.get::<u32>("run_pin")?;
        let signals = config.get_or::<serde_json::Value>("signals", serde_json::json!({}))?;
        let mut pins = vec![run_pin];
        if let Some(signals) = signals.as_object() {
            pins.extend(
                signals
                    .values()
                    .filter_map(serde_json::Value::as_u64)
                    .filter_map(|pin| u32::try_from(pin).ok()),
            );
        }
        pins.sort_unstable();
        pins.dedup();
        Ok(Self {
            state: Arc::new(GpioState::new(chip, active_low, pins)),
        })
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

impl Default for GpioSink {
    fn default() -> Self {
        Self {
            state: Arc::new(GpioState::new(0, true, Vec::new())),
        }
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
    async fn handle(&self, output: &Output, _config: &SinkConfig) -> Result<(), SinkError> {
        tracing::info!(
            target: "rubo_engine::sink::gpio",
            source = output.route().source_id(),
            key = output.route().key(),
            "gpio sink received output"
        );
        Ok(())
    }
}

#[derive(Default)]
pub struct GpioSinkFactory;

impl SinkFactory for GpioSinkFactory {
    fn build(
        &self,
        config: &SinkConfig,
        _resources: &mut crate::RuntimeResources,
    ) -> Result<Arc<dyn Sink>, SinkError> {
        Ok(Arc::new(GpioSink::from_config(config)?))
    }
}

struct GpioState {
    chip: u8,
    active_low: bool,
    pins: Vec<u32>,
    active: AtomicBool,
    active_count: AtomicUsize,
    #[cfg(all(feature = "hardware", target_os = "linux"))]
    output_lines: std::sync::Mutex<Option<gpiod::Lines<gpiod::Output>>>,
}

impl GpioState {
    fn new(chip: u8, active_low: bool, pins: Vec<u32>) -> Self {
        Self {
            chip,
            active_low,
            pins,
            active: AtomicBool::new(false),
            active_count: AtomicUsize::new(0),
            #[cfg(all(feature = "hardware", target_os = "linux"))]
            output_lines: std::sync::Mutex::new(None),
        }
    }

    fn set_active(&self, active: bool) {
        self.active.store(active, Ordering::SeqCst);
        #[cfg(all(feature = "hardware", target_os = "linux"))]
        if let Err(error) = self.write_pins(active) {
            tracing::warn!(target: "rubo_engine::sink::gpio", %error, "gpio write failed");
        }
        let high = active != self.active_low;
        tracing::info!(
            target: "rubo_engine::sink::gpio",
            active,
            high,
            chip = self.chip,
            pins = ?self.pins,
            "gpio state changed"
        );
    }

    #[cfg(all(feature = "hardware", target_os = "linux"))]
    fn write_pins(&self, active: bool) -> Result<(), String> {
        if self.pins.is_empty() {
            return Ok(());
        }
        let mut output_lines = self
            .output_lines
            .lock()
            .map_err(|_| "gpio output lock poisoned".to_string())?;
        let high = active != self.active_low;
        if output_lines.is_none() {
            let chip_path = format!("/dev/gpiochip{}", self.chip);
            let chip = gpiod::Chip::new(chip_path.as_str()).map_err(|error| error.to_string())?;
            let options = gpiod::Options::output(self.pins.clone())
                .values(vec![high; self.pins.len()])
                .consumer("rubo_engine");
            *output_lines = Some(
                chip.request_lines(options)
                    .map_err(|error| error.to_string())?,
            );
        }
        output_lines
            .as_ref()
            .expect("gpio lines initialized")
            .set_values(vec![high; self.pins.len()])
            .map_err(|error| error.to_string())
    }
}

impl Drop for GpioState {
    fn drop(&mut self) {
        if self.active.load(Ordering::SeqCst) {
            self.set_active(false);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gpio_sink_keeps_lights_active_until_all_functions_finish() {
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
}
