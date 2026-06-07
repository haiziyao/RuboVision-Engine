use std::collections::{HashMap, HashSet};

use anyhow::{Context, Result, anyhow};
use rppal::gpio::{Gpio, OutputPin};
use tokio::sync::mpsc;
use tracing::error;

use crate::config::GpioConfig;

use super::{GpioOutput, GpioSink};

pub trait PinBackend {
    fn set_level(&mut self, pin: u8, high: bool) -> Result<()>;
}

pub fn apply_gpio_message(
    backend: &mut impl PinBackend,
    config: &GpioConfig,
    message: GpioOutput,
) -> Result<()> {
    let active = !config.active_low;
    let inactive = config.active_low;

    match message {
        GpioOutput::TaskStarted(signal) => {
            let pin = signal_pin(config, &signal)?;
            backend.set_level(config.run_pin, active)?;
            backend.set_level(pin, active)?;
        }
        GpioOutput::TaskFinished(signal) => {
            let pin = signal_pin(config, &signal)?;
            backend.set_level(pin, inactive)?;
            backend.set_level(config.run_pin, inactive)?;
        }
        GpioOutput::Reset => {
            for pin in config.signals.values() {
                backend.set_level(*pin, inactive)?;
            }
            backend.set_level(config.run_pin, inactive)?;
        }
    }

    Ok(())
}

fn signal_pin(config: &GpioConfig, signal: &str) -> Result<u8> {
    config
        .signals
        .get(signal)
        .copied()
        .ok_or_else(|| anyhow!("unknown GPIO signal `{signal}`"))
}

pub fn start_gpio_sink(config: GpioConfig) -> Result<GpioSink> {
    let mut backend = RppalPinBackend::new(&config)?;
    apply_gpio_message(&mut backend, &config, GpioOutput::Reset)?;
    Ok(start_gpio_worker_with_backend(config, backend))
}

pub fn start_gpio_worker_with_backend(
    config: GpioConfig,
    mut backend: impl PinBackend + Send + 'static,
) -> GpioSink {
    let (sender, mut receiver) = mpsc::channel(32);
    std::thread::Builder::new()
        .name("gpio-message-sink".to_string())
        .spawn(move || {
            while let Some(message) = receiver.blocking_recv() {
                if let Err(error) = apply_gpio_message(&mut backend, &config, message) {
                    error!("failed to apply GPIO output: {error:#}");
                }
            }
            if let Err(error) = apply_gpio_message(&mut backend, &config, GpioOutput::Reset) {
                error!("failed to reset GPIO outputs: {error:#}");
            }
        })
        .expect("failed to spawn GPIO message sink thread");

    GpioSink::new(sender)
}

struct RppalPinBackend {
    pins: HashMap<u8, OutputPin>,
}

impl RppalPinBackend {
    fn new(config: &GpioConfig) -> Result<Self> {
        let gpio = Gpio::new().context("failed to access GPIO")?;
        let mut pin_numbers: HashSet<u8> = config.signals.values().copied().collect();
        pin_numbers.insert(config.run_pin);

        let mut pins = HashMap::new();
        for pin_number in pin_numbers {
            let pin = gpio
                .get(pin_number)
                .with_context(|| format!("failed to access GPIO pin {pin_number}"))?
                .into_output();
            pins.insert(pin_number, pin);
        }
        Ok(Self { pins })
    }
}

impl PinBackend for RppalPinBackend {
    fn set_level(&mut self, pin: u8, high: bool) -> Result<()> {
        let output = self
            .pins
            .get_mut(&pin)
            .ok_or_else(|| anyhow!("GPIO pin {pin} was not initialized"))?;
        if high {
            output.set_high();
        } else {
            output.set_low();
        }
        Ok(())
    }
}
