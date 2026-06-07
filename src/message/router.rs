use anyhow::Error;

use crate::config::ReturnTargets;

use super::{GpioOutput, GpioSink, MessageSink, TaskOutput, UartSink, WebSink};

#[derive(Debug, Clone, Default)]
pub struct MessageRouter {
    web: Option<WebSink>,
    uart: Option<UartSink>,
    gpio: Option<GpioSink>,
}

impl MessageRouter {
    pub fn new(web: Option<WebSink>, uart: Option<UartSink>, gpio: Option<GpioSink>) -> Self {
        Self { web, uart, gpio }
    }

    pub async fn route(&self, targets: &ReturnTargets, output: &TaskOutput) -> Vec<Error> {
        let web_output = output.clone();
        let uart_value = output
            .value
            .as_deref()
            .unwrap_or(output.text.as_str())
            .to_string();

        let web = async {
            if !targets.web {
                return Ok(());
            }
            match self.web.as_ref() {
                Some(sink) => sink.send(web_output).await,
                None => Err(anyhow::anyhow!(
                    "web output is enabled but no WebSink exists"
                )),
            }
        };
        let uart = async {
            if !targets.uart {
                return Ok(());
            }
            match self.uart.as_ref() {
                Some(sink) => sink.send_value(&uart_value).await,
                None => Err(anyhow::anyhow!(
                    "UART output is enabled but no UartSink exists"
                )),
            }
        };

        let (web_result, uart_result) = tokio::join!(web, uart);
        [web_result, uart_result]
            .into_iter()
            .filter_map(Result::err)
            .collect()
    }

    pub async fn task_started(&self, targets: &ReturnTargets) -> Vec<Error> {
        self.route_gpio(targets, GpioOutput::TaskStarted).await
    }

    pub async fn task_finished(&self, targets: &ReturnTargets) -> Vec<Error> {
        self.route_gpio(targets, GpioOutput::TaskFinished).await
    }

    async fn route_gpio(
        &self,
        targets: &ReturnTargets,
        output: impl FnOnce(String) -> GpioOutput,
    ) -> Vec<Error> {
        let Some(signal) = targets.gpio.clone() else {
            return Vec::new();
        };
        let result = match self.gpio.as_ref() {
            Some(sink) => sink.send(output(signal)).await,
            None => Err(anyhow::anyhow!(
                "GPIO output is enabled but no GpioSink exists"
            )),
        };
        result.err().into_iter().collect()
    }
}
