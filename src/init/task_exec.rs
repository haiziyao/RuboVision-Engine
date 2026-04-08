use tokio::sync::mpsc::{Sender,Receiver};
use tracing::info;
use crate::config::DeviceParam;
use crate::device::Device;
use crate::func::FunctionWorker;
use crate::web::WebMessage;
use anyhow::Result;

#[derive(Debug)]
pub struct TaskExecutor{
    sender: Sender<WebMessage>,
}

impl TaskExecutor {
    pub fn new(sender: Sender<WebMessage>) -> TaskExecutor {
        TaskExecutor{
            sender,
        }
    }
    pub fn get_sender(&self) -> Sender<WebMessage> {
        self.sender.clone()
    }
}


pub async fn execute(sender: Sender<WebMessage>,device:Option<Device>,func:Option<FunctionWorker>)->Result<()> {
    let mut device = device.unwrap_or(Device::None);
    if let Some(func_worker) = func {
        let FunctionWorker{
            func_id,
            mut args,
            mut func,
        } = func_worker;
        info!("{func_id}({args})  is running",func_id = func_id,args = args.join(" "));
        let result =func(&mut args, &mut device);
        sender.send(result).await;
        info!("{} has finished execution",func_id);
    }
    info!("Task executor start working");
    Ok(())
}