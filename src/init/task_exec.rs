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
    runtime_param: u8,
) -> Result<(TaskOutput, ReturnTargets)> {
    let func_id = func_worker.func_id.clone();
    let returns = func_worker.returns.clone();
    info!("{func_id} is running");
    let result = func_worker.run(runtime_param, &device)?;

    info!("{} has finished execution", func_id);
    Ok((result, returns))
}

pub async fn execute(
    router: MessageRouter,
    device: Device,
    func: FunctionWorker,
    runtime_param: u8,
) -> Result<()> {
    let targets = func.returns.clone();
    let func_id = func.func_id.clone();
    for error in router.task_started(&targets).await {
        warn!("task pre_func message failed: {error:#}");
    }

    let execution = tokio::task::spawn_blocking(move || execute_sync(device, func, runtime_param))
        .await
        .context("blocking task join failed")
        .and_then(|result| result);

    let routed_output = match &execution {
        Ok((result, returns)) => (result.clone(), returns.clone()),
        Err(error) => (
            TaskOutput::error(format!("{func_id} failed: {error:#}")),
            targets.clone(),
        ),
    };
    for error in router.route(&routed_output.1, &routed_output.0).await {
        warn!("task result message failed: {error:#}");
    }
    for error in router.task_finished(&targets).await {
        warn!("task after_func message failed: {error:#}");
    }

    info!("task result routing finished");
    execution.map(|_| ())
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::config::ReturnTargets;
    use crate::device::Device;
    use crate::func::FunctionWorker;
    use crate::message::{GpioOutput, GpioSink, MessageRouter, TaskOutput, UartSink, WebSink};
    use anyhow::Result;
    use tokio::sync::mpsc;

    use super::execute;

    #[test]
    fn function_worker_passes_runtime_param_to_runner() -> Result<()> {
        let worker = FunctionWorker::new(
            "test",
            ReturnTargets::default(),
            Arc::new(|runtime_param, _device| {
                Ok(TaskOutput::value("done", runtime_param.to_string()))
            }),
        );

        let result = worker.run(7, &Device::None)?;
        assert_eq!(result.value.as_deref(), Some("7"));
        Ok(())
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
        let returns = ReturnTargets {
            web: true,
            uart: true,
            gpio: Some("color".to_string()),
        };
        let worker = FunctionWorker::new(
            "test",
            returns,
            Arc::new(|_runtime_param, _device| Ok(TaskOutput::value("task finished", "42"))),
        );

        execute(router, Device::None, worker, 0).await?;

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
        let returns = ReturnTargets {
            web: false,
            uart: false,
            gpio: Some("color".to_string()),
        };
        let worker = FunctionWorker::new(
            "panic_test",
            returns,
            Arc::new(|_runtime_param, _device: &Device| -> Result<TaskOutput> {
                panic!("test function panic")
            }),
        );

        assert!(execute(router, Device::None, worker, 0).await.is_err());
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
