use std::collections::VecDeque;

use async_trait::async_trait;

use crate::{
    Message, SourceError, SourceFactory, SourceHandler,
    config::{ConfigAccess, SourceConfig},
};

pub struct UartSource {
    parser: UartFrameParser,
    messages: VecDeque<Message>,
    #[cfg(all(feature = "hardware", target_os = "linux"))]
    receiver: tokio::sync::broadcast::Receiver<Vec<u8>>,
    #[cfg(not(all(feature = "hardware", target_os = "linux")))]
    settings: UartSettings,
}

impl UartSource {
    fn from_config(config: &SourceConfig) -> Result<Self, SourceError> {
        let settings = uart_settings(config).map_err(config_error)?;
        let prefix = config
            .get_or("prefix", Vec::<u8>::new())
            .map_err(config_error)?;
        let suffix = config
            .get_or("suffix", Vec::<u8>::new())
            .map_err(config_error)?;
        let content_bytes = config
            .get_or("content_bytes", 1_usize)
            .map_err(config_error)?;
        let parser = UartFrameParser::new(prefix, suffix, content_bytes)
            .map_err(|message| SourceError::Config { message })?;
        #[cfg(all(feature = "hardware", target_os = "linux"))]
        let receiver = open_uart(settings)
            .map_err(|message| SourceError::SourceHandle { message })?
            .subscribe();
        Ok(Self {
            parser,
            messages: VecDeque::new(),
            #[cfg(all(feature = "hardware", target_os = "linux"))]
            receiver,
            #[cfg(not(all(feature = "hardware", target_os = "linux")))]
            settings,
        })
    }
}

#[async_trait]
impl SourceHandler for UartSource {
    async fn handle(&mut self, _config: &SourceConfig) -> Result<Message, SourceError> {
        if let Some(message) = self.messages.pop_front() {
            return Ok(message);
        }
        #[cfg(all(feature = "hardware", target_os = "linux"))]
        loop {
            match self.receiver.recv().await {
                Ok(bytes) => {
                    self.messages
                        .extend(self.parser.push(&bytes).into_iter().map(|frame| {
                            Message::new(frame.key()).payload(serde_json::json!({
                                "frame": frame.frame,
                                "content": frame.content
                            }))
                        }));
                    if let Some(message) = self.messages.pop_front() {
                        return Ok(message);
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(count)) => {
                    tracing::warn!(target: "rubo_engine::source::uart", count, "uart input lagged");
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                    return Err(SourceError::SourceHandle {
                        message: "uart worker stopped".to_string(),
                    });
                }
            }
        }

        #[cfg(not(all(feature = "hardware", target_os = "linux")))]
        {
            for frame in self.parser.push(&[]) {
                let _ = (frame.key(), frame.frame, frame.content);
            }
            Err(SourceError::SourceHandle {
                message: format!(
                    "hardware feature is disabled; cannot read uart serial={}",
                    self.settings.serial
                ),
            })
        }
    }
}

#[derive(Default)]
pub struct UartSourceFactory;

impl SourceFactory for UartSourceFactory {
    fn build(&self, config: &SourceConfig) -> Result<Box<dyn SourceHandler>, SourceError> {
        Ok(Box::new(UartSource::from_config(config)?))
    }
}

fn config_error(error: crate::ConfigError) -> SourceError {
    SourceError::Config {
        message: error.to_string(),
    }
}

struct UartFrameParser {
    prefix: Vec<u8>,
    suffix: Vec<u8>,
    content_bytes: usize,
    pending: Vec<u8>,
}

impl UartFrameParser {
    fn new(prefix: Vec<u8>, suffix: Vec<u8>, content_bytes: usize) -> Result<Self, String> {
        if content_bytes == 0 {
            return Err("uart content_bytes must be greater than zero".to_string());
        }
        Ok(Self {
            prefix,
            suffix,
            content_bytes,
            pending: Vec::new(),
        })
    }

    fn push(&mut self, bytes: &[u8]) -> Vec<UartFrame> {
        self.pending.extend_from_slice(bytes);
        let mut frames = Vec::new();
        let frame_len = self.prefix.len() + self.content_bytes + self.suffix.len();
        loop {
            self.align_prefix();
            if self.pending.len() < frame_len {
                break;
            }
            let suffix_start = self.prefix.len() + self.content_bytes;
            let suffix_end = suffix_start + self.suffix.len();
            if self.pending[suffix_start..suffix_end] != self.suffix {
                self.pending.remove(0);
                continue;
            }
            let frame: Vec<u8> = self.pending.drain(..frame_len).collect();
            let content = frame[self.prefix.len()..suffix_start].to_vec();
            frames.push(UartFrame { frame, content });
        }
        frames
    }

    fn align_prefix(&mut self) {
        if self.prefix.is_empty() {
            return;
        }
        while !self.pending.is_empty() && !self.prefix.starts_with(&self.pending) {
            if self.pending.starts_with(&self.prefix) {
                return;
            }
            self.pending.remove(0);
        }
    }
}

struct UartFrame {
    frame: Vec<u8>,
    content: Vec<u8>,
}

impl UartFrame {
    fn key(&self) -> String {
        self.content
            .iter()
            .map(u8::to_string)
            .collect::<Vec<_>>()
            .join("_")
    }
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct UartSettings {
    serial: String,
    baud: u32,
    data_bit: u8,
    stop_bit: u8,
    parity_bit: bool,
}

impl UartSettings {
    #[cfg(not(all(feature = "hardware", target_os = "linux")))]
    pub(crate) fn serial(&self) -> &str {
        &self.serial
    }
}

pub(crate) fn uart_settings(
    config: &impl ConfigAccess,
) -> Result<UartSettings, crate::ConfigError> {
    let settings = UartSettings {
        serial: config.get_or("serial", "/dev/ttyV0".to_string())?,
        baud: config.get_or("baud", 9600_u32)?,
        data_bit: config.get_or("data_bit", 8_u8)?,
        stop_bit: config.get_or("stop_bit", 1_u8)?,
        parity_bit: config.get_or("parity_bit", false)?,
    };
    if settings.serial.is_empty()
        || settings.baud == 0
        || !(5..=8).contains(&settings.data_bit)
        || !(1..=2).contains(&settings.stop_bit)
    {
        return Err(crate::ConfigError::ConfigFormat {
            message: format!(
                "invalid UART config serial={} baud={} data_bit={} stop_bit={}",
                settings.serial, settings.baud, settings.data_bit, settings.stop_bit
            ),
        });
    }
    Ok(settings)
}

#[cfg(all(feature = "hardware", target_os = "linux"))]
pub(crate) struct UartHandle {
    messages: tokio::sync::broadcast::Sender<Vec<u8>>,
    output: std::sync::mpsc::SyncSender<Vec<u8>>,
}

#[cfg(all(feature = "hardware", target_os = "linux"))]
impl UartHandle {
    fn subscribe(&self) -> tokio::sync::broadcast::Receiver<Vec<u8>> {
        self.messages.subscribe()
    }

    pub(crate) fn write(&self, bytes: Vec<u8>) -> Result<(), String> {
        self.output
            .try_send(bytes)
            .map_err(|error| error.to_string())
    }
}

#[cfg(all(feature = "hardware", target_os = "linux"))]
struct UartWorker {
    settings: UartSettings,
    handle: UartHandle,
}

#[cfg(all(feature = "hardware", target_os = "linux"))]
static UART_WORKERS: std::sync::OnceLock<
    std::sync::Mutex<std::collections::HashMap<String, UartWorker>>,
> = std::sync::OnceLock::new();

#[cfg(all(feature = "hardware", target_os = "linux"))]
pub(crate) fn open_uart(settings: UartSettings) -> Result<UartHandle, String> {
    let workers = UART_WORKERS.get_or_init(Default::default);
    let mut workers = workers
        .lock()
        .map_err(|_| "uart worker pool lock poisoned".to_string())?;
    if let Some(worker) = workers.get(&settings.serial) {
        if worker.settings != settings {
            return Err(format!(
                "uart serial {} is already open with different settings",
                settings.serial
            ));
        }
        return Ok(UartHandle {
            messages: worker.handle.messages.clone(),
            output: worker.handle.output.clone(),
        });
    }
    let handle = start_uart_worker(settings.clone());
    workers.insert(
        settings.serial.clone(),
        UartWorker {
            settings,
            handle: UartHandle {
                messages: handle.messages.clone(),
                output: handle.output.clone(),
            },
        },
    );
    Ok(handle)
}

#[cfg(all(feature = "hardware", target_os = "linux"))]
fn start_uart_worker(settings: UartSettings) -> UartHandle {
    let (messages, _) = tokio::sync::broadcast::channel(32);
    let (output, output_receiver) = std::sync::mpsc::sync_channel(32);
    let thread_messages = messages.clone();
    std::thread::spawn(move || run_uart_worker(settings, thread_messages, output_receiver));
    UartHandle { messages, output }
}

#[cfg(all(feature = "hardware", target_os = "linux"))]
fn run_uart_worker(
    settings: UartSettings,
    messages: tokio::sync::broadcast::Sender<Vec<u8>>,
    output: std::sync::mpsc::Receiver<Vec<u8>>,
) {
    use std::io::{self, Read, Write};
    use std::time::Duration;

    use serialport::{DataBits, Parity, StopBits};

    let mut port = None;
    loop {
        if port.is_none() {
            let data_bits = match settings.data_bit {
                5 => DataBits::Five,
                6 => DataBits::Six,
                7 => DataBits::Seven,
                _ => DataBits::Eight,
            };
            let stop_bits = if settings.stop_bit == 2 {
                StopBits::Two
            } else {
                StopBits::One
            };
            let parity = if settings.parity_bit {
                Parity::Even
            } else {
                Parity::None
            };
            match serialport::new(&settings.serial, settings.baud)
                .data_bits(data_bits)
                .stop_bits(stop_bits)
                .parity(parity)
                .timeout(Duration::from_millis(100))
                .open()
            {
                Ok(opened) => {
                    tracing::info!(target: "rubo_engine::source::uart", serial = %settings.serial, "uart connected");
                    port = Some(opened);
                }
                Err(error) => {
                    tracing::warn!(target: "rubo_engine::source::uart", serial = %settings.serial, %error, "uart connect failed");
                    std::thread::sleep(Duration::from_millis(500));
                    continue;
                }
            }
        }
        let Some(opened) = port.as_mut() else {
            continue;
        };
        while let Ok(bytes) = output.try_recv() {
            if let Err(error) = opened.write_all(&bytes).and_then(|_| opened.flush()) {
                tracing::warn!(target: "rubo_engine::source::uart", serial = %settings.serial, %error, "uart write failed");
                port = None;
                break;
            }
        }
        let Some(opened) = port.as_mut() else {
            continue;
        };
        let mut buffer = [0_u8; 64];
        match opened.read(&mut buffer) {
            Ok(count) if count > 0 => {
                let _ = messages.send(buffer[..count].to_vec());
            }
            Ok(_) => {}
            Err(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::TimedOut | io::ErrorKind::WouldBlock
                ) => {}
            Err(error) => {
                tracing::warn!(target: "rubo_engine::source::uart", serial = %settings.serial, %error, "uart read failed");
                port = None;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uart_frame_parser_extracts_content_across_split_and_joined_reads() {
        let mut parser = UartFrameParser::new(vec![170], vec![85], 2).unwrap();

        assert!(parser.push(&[0, 170, 1]).is_empty());
        let frames = parser.push(&[2, 85, 170, 3, 4, 85]);

        assert_eq!(frames.len(), 2);
        assert_eq!(frames[0].frame, vec![170, 1, 2, 85]);
        assert_eq!(frames[0].content, vec![1, 2]);
        assert_eq!(frames[0].key(), "1_2");
        assert_eq!(frames[1].content, vec![3, 4]);
        assert_eq!(frames[1].key(), "3_4");
    }

    #[test]
    fn uart_frame_parser_recovers_after_invalid_suffix() {
        let mut parser = UartFrameParser::new(vec![170], vec![85], 1).unwrap();

        let frames = parser.push(&[170, 9, 0, 170, 7, 85]);

        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0].content, vec![7]);
    }

    #[test]
    fn uart_frame_parser_keeps_one_byte_message_compatibility() {
        let mut parser = UartFrameParser::new(Vec::new(), Vec::new(), 1).unwrap();

        let frames = parser.push(&[1, 2]);

        assert_eq!(frames[0].key(), "1");
        assert_eq!(frames[1].key(), "2");
    }
}
