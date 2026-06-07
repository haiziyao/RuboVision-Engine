use std::future::Future;

use anyhow::{Context, Result};
use tokio::sync::mpsc;

use crate::web::WebMessage;

use super::{GpioOutput, TaskOutput};

pub trait MessageSink<M>: Send + Sync {
    fn send(&self, message: M) -> impl Future<Output = Result<()>> + Send;
}

#[derive(Debug, Clone)]
pub struct WebSink {
    sender: mpsc::Sender<WebMessage>,
}

impl WebSink {
    pub fn new(sender: mpsc::Sender<WebMessage>) -> Self {
        Self { sender }
    }
}

impl MessageSink<TaskOutput> for WebSink {
    fn send(&self, output: TaskOutput) -> impl Future<Output = Result<()>> + Send {
        let sender = self.sender.clone();
        async move {
            sender
                .send(WebMessage {
                    id: 0,
                    created_at_ms: 0,
                    code: output.code,
                    text: output.text,
                    image: output.image,
                })
                .await
                .context("failed to send web output")
        }
    }
}

#[derive(Debug, Clone)]
pub struct UartSink {
    sender: mpsc::Sender<Vec<u8>>,
}

impl UartSink {
    pub fn new(sender: mpsc::Sender<Vec<u8>>) -> Self {
        Self { sender }
    }

    pub async fn send_value(&self, value: &str) -> Result<()> {
        let mut bytes = value.as_bytes().to_vec();
        if !bytes.ends_with(b"\n") {
            bytes.push(b'\n');
        }
        self.send(bytes).await
    }
}

impl MessageSink<Vec<u8>> for UartSink {
    fn send(&self, bytes: Vec<u8>) -> impl Future<Output = Result<()>> + Send {
        let sender = self.sender.clone();
        async move {
            sender
                .send(bytes)
                .await
                .context("failed to queue UART output")
        }
    }
}

#[derive(Debug, Clone)]
pub struct GpioSink {
    sender: mpsc::Sender<GpioOutput>,
}

impl GpioSink {
    pub fn new(sender: mpsc::Sender<GpioOutput>) -> Self {
        Self { sender }
    }
}

impl MessageSink<GpioOutput> for GpioSink {
    fn send(&self, output: GpioOutput) -> impl Future<Output = Result<()>> + Send {
        let sender = self.sender.clone();
        async move {
            sender
                .send(output)
                .await
                .context("failed to queue GPIO output")
        }
    }
}
