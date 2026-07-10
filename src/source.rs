use async_trait::async_trait;
use rubo_engine::{
    Message, RuntimeResources, SourceError, SourceFactory, SourceHandler,
    config::{ConfigAccess, SourceConfig},
};

#[rubo_engine::source(kind = "uart")]
#[derive(Default)]
pub struct UartSourceFactory;

impl SourceFactory for UartSourceFactory {
    fn build(&self, config: &SourceConfig) -> Result<Box<dyn SourceHandler>, SourceError> {
        Ok(Box::new(UartSource::from_config(config)?))
    }

    fn build_with_resources(
        &self,
        config: &SourceConfig,
        _resources: &mut RuntimeResources,
    ) -> Result<Box<dyn SourceHandler>, SourceError> {
        self.build(config)
    }
}

pub struct UartSource {
    #[cfg(not(all(feature = "hardware", target_os = "linux")))]
    serial: String,
    #[cfg(not(all(feature = "hardware", target_os = "linux")))]
    baud: u32,
    #[cfg(not(all(feature = "hardware", target_os = "linux")))]
    data_bit: u8,
    #[cfg(not(all(feature = "hardware", target_os = "linux")))]
    stop_bit: u8,
    #[cfg(not(all(feature = "hardware", target_os = "linux")))]
    parity_bit: bool,
    #[cfg(all(feature = "hardware", target_os = "linux"))]
    receiver: tokio::sync::broadcast::Receiver<Message>,
}

impl UartSource {
    fn from_config(config: &SourceConfig) -> Result<Self, SourceError> {
        let serial = config.get_or("serial", "/dev/ttyV0".to_string())?;
        let baud = config.get_or("baud", 9600_u32)?;
        let data_bit = config.get_or("data_bit", 8_u8)?;
        let stop_bit = config.get_or("stop_bit", 1_u8)?;
        let parity_bit = config.get_or("parity_bit", false)?;
        if serial.is_empty()
            || baud == 0
            || !(5..=8).contains(&data_bit)
            || !(1..=2).contains(&stop_bit)
        {
            return Err(SourceError::SourceHandle {
                message: format!(
                    "invalid UART config serial={serial} baud={baud} data_bit={data_bit} stop_bit={stop_bit}"
                ),
            });
        }
        #[cfg(all(feature = "hardware", target_os = "linux"))]
        let receiver = uart_receiver(UartSettings {
            serial: serial.clone(),
            baud,
            data_bit,
            stop_bit,
            parity_bit,
        });
        Ok(Self {
            #[cfg(not(all(feature = "hardware", target_os = "linux")))]
            serial,
            #[cfg(not(all(feature = "hardware", target_os = "linux")))]
            baud,
            #[cfg(not(all(feature = "hardware", target_os = "linux")))]
            data_bit,
            #[cfg(not(all(feature = "hardware", target_os = "linux")))]
            stop_bit,
            #[cfg(not(all(feature = "hardware", target_os = "linux")))]
            parity_bit,
            #[cfg(all(feature = "hardware", target_os = "linux"))]
            receiver,
        })
    }
}

#[async_trait]
impl SourceHandler for UartSource {
    async fn handle(&mut self, _config: &SourceConfig) -> Result<Message, SourceError> {
        #[cfg(all(feature = "hardware", target_os = "linux"))]
        {
            loop {
                match self.receiver.recv().await {
                    Ok(message) => return Ok(message),
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                        return Err(SourceError::SourceHandle {
                            message: "uart worker stopped".to_string(),
                        });
                    }
                }
            }
        }

        #[cfg(not(all(feature = "hardware", target_os = "linux")))]
        {
            Err(SourceError::SourceHandle {
                message: format!(
                    "hardware feature is disabled; cannot read uart serial={} baud={} data_bit={} stop_bit={} parity_bit={}",
                    self.serial, self.baud, self.data_bit, self.stop_bit, self.parity_bit
                ),
            })
        }
    }
}

#[cfg(all(feature = "hardware", target_os = "linux"))]
#[derive(Clone, PartialEq, Eq)]
struct UartSettings {
    serial: String,
    baud: u32,
    data_bit: u8,
    stop_bit: u8,
    parity_bit: bool,
}

#[cfg(all(feature = "hardware", target_os = "linux"))]
struct UartWorker {
    settings: std::sync::Arc<std::sync::Mutex<UartSettings>>,
    messages: tokio::sync::broadcast::Sender<Message>,
    output: std::sync::mpsc::SyncSender<String>,
}

#[cfg(all(feature = "hardware", target_os = "linux"))]
static UART_WORKER: std::sync::OnceLock<UartWorker> = std::sync::OnceLock::new();

#[cfg(all(feature = "hardware", target_os = "linux"))]
fn uart_receiver(settings: UartSettings) -> tokio::sync::broadcast::Receiver<Message> {
    let worker = UART_WORKER.get_or_init(|| UartWorker::start(settings.clone()));
    *worker.settings.lock().expect("uart settings lock poisoned") = settings;
    worker.messages.subscribe()
}

#[cfg(all(feature = "hardware", target_os = "linux"))]
pub(crate) fn send_uart_output(value: String) -> Result<(), String> {
    UART_WORKER
        .get()
        .ok_or_else(|| "uart worker is not initialized".to_string())?
        .output
        .try_send(value)
        .map_err(|error| error.to_string())
}

#[cfg(all(feature = "hardware", target_os = "linux"))]
impl UartWorker {
    fn start(settings: UartSettings) -> Self {
        let settings = std::sync::Arc::new(std::sync::Mutex::new(settings));
        let (messages, _) = tokio::sync::broadcast::channel(32);
        let (output, output_receiver) = std::sync::mpsc::sync_channel(32);
        let thread_settings = settings.clone();
        let thread_messages = messages.clone();
        std::thread::spawn(move || {
            run_uart_worker(thread_settings, thread_messages, output_receiver)
        });
        Self {
            settings,
            messages,
            output,
        }
    }
}

#[cfg(all(feature = "hardware", target_os = "linux"))]
fn run_uart_worker(
    settings: std::sync::Arc<std::sync::Mutex<UartSettings>>,
    messages: tokio::sync::broadcast::Sender<Message>,
    output: std::sync::mpsc::Receiver<String>,
) {
    use std::time::Duration;

    use rppal::uart::{Parity, Queue, Uart};

    let mut active_settings = None;
    let mut port = None;
    loop {
        let current = settings
            .lock()
            .expect("uart settings lock poisoned")
            .clone();
        if active_settings.as_ref() != Some(&current) {
            active_settings = Some(current.clone());
            port = None;
        }
        if port.is_none() {
            while output.try_recv().is_ok() {}
            match Uart::with_path(
                &current.serial,
                current.baud,
                if current.parity_bit {
                    Parity::Even
                } else {
                    Parity::None
                },
                current.data_bit,
                current.stop_bit,
            ) {
                Ok(mut opened) => {
                    if let Err(error) = opened
                        .set_read_mode(0, Duration::from_millis(100))
                        .and_then(|_| opened.set_write_mode(true))
                    {
                        eprintln!("[uart] configure error: {error}");
                        std::thread::sleep(Duration::from_millis(500));
                        continue;
                    }
                    port = Some(opened);
                }
                Err(error) => {
                    eprintln!("[uart] connect error: {error}");
                    std::thread::sleep(Duration::from_millis(500));
                    continue;
                }
            }
        }

        let Some(opened) = port.as_mut() else {
            continue;
        };
        while let Ok(value) = output.try_recv() {
            if let Err(error) = write_uart_value(opened, &value) {
                eprintln!("[uart] write error: {error}");
                port = None;
                break;
            }
        }
        let Some(opened) = port.as_mut() else {
            continue;
        };
        let mut byte = [0_u8; 1];
        match opened.read(&mut byte) {
            Ok(1) => {
                let message = Message::new(byte[0].to_string()).payload(serde_json::json!({
                    "cmd": byte[0],
                    "serial": current.serial,
                    "baud": current.baud,
                    "data_bit": current.data_bit,
                    "stop_bit": current.stop_bit,
                    "parity_bit": current.parity_bit
                }));
                let _ = messages.send(message);
            }
            Ok(_) => {}
            Err(error) => {
                eprintln!("[uart] read error: {error}");
                port = None;
            }
        }
    }

    fn write_uart_value(uart: &mut Uart, value: &str) -> Result<(), String> {
        for bytes in [value.as_bytes(), b"\n".as_slice()] {
            let mut remaining = bytes;
            while !remaining.is_empty() {
                let written = uart.write(remaining).map_err(|error| error.to_string())?;
                if written == 0 {
                    return Err("uart write returned zero bytes".to_string());
                }
                remaining = &remaining[written..];
            }
        }
        uart.flush(Queue::Output).map_err(|error| error.to_string())
    }
}

#[cfg(all(test, feature = "hardware", target_os = "linux"))]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore = "requires Ubuntu and a configured UART device"]
    async fn uart_test() {
        let config = crate::vision::test::load_config().expect("load config");
        let source_config = &config.sources()[crate::config::UART_SOURCE_ID];
        let mut source = UartSource::from_config(source_config).expect("start UART worker");
        eprintln!("send one UART byte within 30 seconds");
        let message = tokio::time::timeout(
            std::time::Duration::from_secs(30),
            source.handle(source_config),
        )
        .await
        .expect("UART input timed out")
        .expect("read UART input");
        eprintln!("received UART key={}", message.key());
        send_uart_output("debug success".to_string()).expect("write UART output");
    }
}
