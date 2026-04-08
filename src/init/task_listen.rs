use anyhow::Context;
use log::{info, warn};
use crate::init::{TaskDispatcher, TaskExecutor};
use tokio::sync::mpsc::Receiver;
use tracing::error;
use crate::source::Event;
use crate::init::task_exec::execute;

pub struct TaskListener{
    executor: TaskExecutor,
    listener: Receiver<Event>,
    dispatcher: TaskDispatcher,
    
}



impl TaskListener {
    pub fn new(executor: TaskExecutor, listener: Receiver<Event>,dispatcher:TaskDispatcher) -> Self {
        TaskListener{
            executor,
            listener,
            dispatcher,
        }
    }

    
    // only do that receive message and then 'inform'
    pub async fn run(mut self){
        loop {
            match self.listener.recv().await {
                Some(event) => {
                    info!("[TaskListener] received event: {:?}", event);
                    let device =self.dispatcher.find_device(&event);
                    let func =self.dispatcher.find_func(&event);
                    let sender = self.executor.get_sender();
                    tokio::spawn(async {
                        match execute(sender,device,func).await{
                            Ok(_) => {
                                info!("[TaskListener] executed successfully");
                            },
                            Err(e) =>{
                                error!("[TaskListener] executed failed {:?}", e);
                            }
                        }
                    });
                },
                None => {
                    info!("[TaskListener] received None event");
                    break;
                }
            }
        }
    }


}

