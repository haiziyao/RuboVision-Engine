use std::thread;
use std::time::Duration;

use anyhow::{Context, Result, anyhow};
use rppal::uart::{Parity, Uart};
use tokio::sync::mpsc;
use tracing::{error, warn};

use crate::config::UartConfig;

pub struct UartChannels {
    pub incoming: mpsc::Receiver<Vec<u8>>,
    pub outgoing: mpsc::Sender<Vec<u8>>,
}

pub fn start_uart_transport(config: &UartConfig) -> Result<UartChannels> {
    validate_uart_config(config)?;
    let mut uart = Uart::with_path(
        config.serial.clone(),
        config.baud,
        if config.parity_bit {
            Parity::Even
        } else {
            Parity::None
        },
        config.data_bit,
        config.stop_bit,
    )
    .with_context(|| format!("failed to open UART {}", config.serial))?;
    uart.set_read_mode(0, Duration::from_millis(20))?;
    uart.set_write_mode(true)?;

    let (incoming_tx, incoming) = mpsc::channel(32);
    let (outgoing, mut outgoing_rx) = mpsc::channel::<Vec<u8>>(32);
    let thread_name = format!("uart-{}", config.serial.replace('/', "_"));

    thread::Builder::new()
        .name(thread_name)
        .spawn(move || {
            let mut buffer = [0u8; 64];
            loop {
                while let Ok(bytes) = outgoing_rx.try_recv() {
                    if let Err(error) = write_uart_all(&mut uart, &bytes) {
                        error!("UART transport write failed: {error:#}");
                    }
                }

                if incoming_tx.is_closed() && outgoing_rx.is_closed() {
                    break;
                }

                match uart.read(&mut buffer) {
                    Ok(0) => {}
                    Ok(count) => {
                        if incoming_tx.blocking_send(buffer[..count].to_vec()).is_err() {
                            break;
                        }
                    }
                    Err(error) => {
                        warn!("UART transport read failed: {error}");
                        thread::sleep(Duration::from_millis(100));
                    }
                }
            }
        })
        .context("failed to spawn UART transport thread")?;

    Ok(UartChannels { incoming, outgoing })
}

fn validate_uart_config(config: &UartConfig) -> Result<()> {
    if config.serial.is_empty() {
        return Err(anyhow!("invalid UART serial path: value is empty"));
    }
    if config.baud == 0 {
        return Err(anyhow!("invalid UART baud: must be greater than 0"));
    }
    if !(5..=8).contains(&config.data_bit) {
        return Err(anyhow!("invalid UART data_bit: must be in 5..=8"));
    }
    if !(1..=2).contains(&config.stop_bit) {
        return Err(anyhow!("invalid UART stop_bit: must be 1 or 2"));
    }
    Ok(())
}

fn write_uart_all(uart: &mut Uart, mut bytes: &[u8]) -> Result<()> {
    while !bytes.is_empty() {
        let written = uart.write(bytes)?;
        if written == 0 {
            return Err(anyhow!("UART write returned 0 bytes"));
        }
        bytes = &bytes[written..];
    }
    uart.drain()?;
    Ok(())
}
