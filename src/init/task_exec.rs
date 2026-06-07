use anyhow::{Context, Result};
use tracing::{info, warn};

use crate::config::ReturnTargets;
use crate::device::Device;
use crate::func::FunctionWorker;
use crate::message::{MessageRouter, TaskOutput};

#[derive(Debug)]
pub struct TaskExecutor {
    router: MessageRouter,
}

impl TaskExecutor {
    pub fn new(router: MessageRouter) -> TaskExecutor {
        TaskExecutor { router }
    }

    pub fn get_router(&self) -> MessageRouter {
        self.router.clone()
    }
}

pub fn execute_sync(
    device: Device,
    func_worker: FunctionWorker,
) -> Result<(TaskOutput, ReturnTargets)> {
    let FunctionWorker {
        func_id,
        args,
        func,
        returns,
    } = func_worker;

    info!(
        "{func_id}({args}) is running",
        func_id = func_id,
        args = args.join(" ")
    );

    let result = func(&args, &device, &returns);

    info!("{} has finished execution", func_id);
    Ok((result, returns))
}

pub async fn execute(router: MessageRouter, device: Device, func: FunctionWorker) -> Result<()> {
    let targets = func.returns.clone();
    for error in router.task_started(&targets).await {
        warn!("task pre_func message failed: {error:#}");
    }

    let execution = tokio::task::spawn_blocking(move || execute_sync(device, func))
        .await
        .context("blocking task join failed")
        .and_then(|result| result);

    if let Ok((result, returns)) = &execution {
        for error in router.route(returns, result).await {
            warn!("task result message failed: {error:#}");
        }
    }
    for error in router.task_finished(&targets).await {
        warn!("task after_func message failed: {error:#}");
    }

    info!("task result routing finished");
    execution.map(|_| ())
}

#[cfg(test)]
mod tests {
    use crate::config::ReturnTargets;
    use crate::device::Device;
    use crate::func::FunctionWorker;
    use crate::message::{GpioOutput, GpioSink, MessageRouter, TaskOutput, UartSink, WebSink};
    use anyhow::Result;
    use tokio::sync::mpsc;

    use super::execute;

    fn successful_function(
        _args: &[String],
        _device: &Device,
        _returns: &ReturnTargets,
    ) -> TaskOutput {
        TaskOutput::value("task finished", "42")
    }

    fn panicking_function(
        _args: &[String],
        _device: &Device,
        _returns: &ReturnTargets,
    ) -> TaskOutput {
        panic!("test function panic")
    }

    #[tokio::test]
    async fn execute_routes_output_and_gpio_lifecycle() -> Result<()> {
        let (web_tx, mut web_rx) = mpsc::channel(4);
        let (uart_tx, mut uart_rx) = mpsc::channel(4);
        let (gpio_tx, mut gpio_rx) = mpsc::channel(4);
        let router = MessageRouter::new(
            Some(WebSink::new(web_tx)),
            Some(UartSink::new(uart_tx)),
            Some(GpioSink::new(gpio_tx)),
        );
        let worker = FunctionWorker::new(
            "test",
            successful_function,
            Vec::new(),
            ReturnTargets {
                web: true,
                uart: true,
                gpio: Some("color".to_string()),
            },
        );

        execute(router, Device::None, worker).await?;

        assert_eq!(
            gpio_rx.recv().await,
            Some(GpioOutput::TaskStarted("color".to_string()))
        );
        assert_eq!(
            gpio_rx.recv().await,
            Some(GpioOutput::TaskFinished("color".to_string()))
        );
        assert_eq!(
            web_rx.recv().await.expect("web result").text,
            "task finished"
        );
        assert_eq!(uart_rx.recv().await.expect("UART result"), b"42\n");
        Ok(())
    }

    #[tokio::test]
    async fn execute_finishes_gpio_lifecycle_after_function_panic() {
        let (gpio_tx, mut gpio_rx) = mpsc::channel(4);
        let router = MessageRouter::new(None, None, Some(GpioSink::new(gpio_tx)));
        let worker = FunctionWorker::new(
            "panic_test",
            panicking_function,
            Vec::new(),
            ReturnTargets {
                web: false,
                uart: false,
                gpio: Some("color".to_string()),
            },
        );

        assert!(execute(router, Device::None, worker).await.is_err());
        assert_eq!(
            gpio_rx.recv().await,
            Some(GpioOutput::TaskStarted("color".to_string()))
        );
        assert_eq!(
            gpio_rx.recv().await,
            Some(GpioOutput::TaskFinished("color".to_string()))
        );
    }
}
