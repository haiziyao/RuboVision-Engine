use anyhow::{Context, Result};
use tokio::sync::mpsc::Sender;
use tracing::info;

use crate::config::ReturnTargets;
use crate::device::Device;
use crate::func::FunctionWorker;
use crate::web::WebMessage;

#[derive(Debug)]
pub struct TaskExecutor {
    sender: Sender<WebMessage>,
}

impl TaskExecutor {
    pub fn new(sender: Sender<WebMessage>) -> TaskExecutor {
        TaskExecutor { sender }
    }

    pub fn get_sender(&self) -> Sender<WebMessage> {
        self.sender.clone()
    }
}

pub fn execute_sync(
    device: Device,
    func_worker: FunctionWorker,
) -> Result<(WebMessage, ReturnTargets)> {
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

pub async fn execute(
    sender: Sender<WebMessage>,
    device: Device,
    func: FunctionWorker,
) -> Result<()> {
    let (result, returns) = tokio::task::spawn_blocking(move || execute_sync(device, func))
        .await
        .context("blocking task join failed")??;

    if returns.web {
        sender
            .send(result)
            .await
            .context("failed to send web message")?;

        info!("task result sent to web channel");
    } else {
        info!("task result web return disabled: {:?}", result);
    }
    Ok(())
}
