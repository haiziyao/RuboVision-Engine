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
    serial: String,
    baud: u32,
    data_bit: u8,
    stop_bit: u8,
    parity_bit: bool,
    #[cfg(feature = "hardware")]
    receiver: tokio::sync::mpsc::Receiver<Result<Message, SourceError>>,
}

impl UartSource {
    fn from_config(config: &SourceConfig) -> Result<Self, SourceError> {
        let serial = config.get_or("serial", "/dev/ttyV0".to_string())?;
        let baud = config.get_or("baud", 9600_u32)?;
        let data_bit = config.get_or("data_bit", 8_u8)?;
        let stop_bit = config.get_or("stop_bit", 1_u8)?;
        let parity_bit = config.get_or("parity_bit", false)?;
        #[cfg(feature = "hardware")]
        let receiver = start_uart_reader(serial.clone(), baud, data_bit, stop_bit, parity_bit)?;
        Ok(Self {
            serial,
            baud,
            data_bit,
            stop_bit,
            parity_bit,
            #[cfg(feature = "hardware")]
            receiver,
        })
    }
}

#[async_trait]
impl SourceHandler for UartSource {
    async fn handle(&mut self, _config: &SourceConfig) -> Result<Message, SourceError> {
        #[cfg(feature = "hardware")]
        {
            return self
                .receiver
                .recv()
                .await
                .ok_or_else(|| SourceError::SourceHandle {
                    message: "uart reader stopped".to_string(),
                })?;
        }

        #[cfg(not(feature = "hardware"))]
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

#[cfg(feature = "hardware")]
fn start_uart_reader(
    serial: String,
    baud: u32,
    data_bit: u8,
    stop_bit: u8,
    parity_bit: bool,
) -> Result<tokio::sync::mpsc::Receiver<Result<Message, SourceError>>, SourceError> {
    let (sender, receiver) = tokio::sync::mpsc::channel(32);
    std::thread::Builder::new()
        .name("rubo-uart-source".to_string())
        .spawn(move || {
            use std::io::{ErrorKind, Read};

            let mut port = match serialport::new(&serial, baud)
                .timeout(std::time::Duration::from_millis(200))
                .open()
            {
                Ok(port) => port,
                Err(error) => {
                    let _ = sender.blocking_send(Err(SourceError::SourceHandle {
                        message: error.to_string(),
                    }));
                    return;
                }
            };

            while !sender.is_closed() {
                let mut byte = [0_u8; 1];
                match port.read_exact(&mut byte) {
                    Ok(()) => {
                        let message =
                            Message::new(byte[0].to_string()).payload(serde_json::json!({
                                "cmd": byte[0],
                                "serial": serial,
                                "baud": baud,
                                "data_bit": data_bit,
                                "stop_bit": stop_bit,
                                "parity_bit": parity_bit
                            }));
                        if sender.blocking_send(Ok(message)).is_err() {
                            break;
                        }
                    }
                    Err(error)
                        if matches!(error.kind(), ErrorKind::TimedOut | ErrorKind::WouldBlock) => {}
                    Err(error) => {
                        let _ = sender.blocking_send(Err(SourceError::SourceHandle {
                            message: error.to_string(),
                        }));
                        break;
                    }
                }
            }
        })
        .map_err(|error| SourceError::SourceHandle {
            message: error.to_string(),
        })?;
    Ok(receiver)
}
