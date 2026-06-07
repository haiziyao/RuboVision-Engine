use log::info;
use tokio::sync::mpsc::Receiver;
use tracing::error;

use crate::init::task_exec::execute;
use crate::init::{TaskDispatcher, TaskExecutor};
use crate::source::Event;

pub struct TaskListener {
    executor: TaskExecutor,
    listener: Receiver<Event>,
    dispatcher: TaskDispatcher,
}

impl TaskListener {
    pub fn new(
        executor: TaskExecutor,
        listener: Receiver<Event>,
        dispatcher: TaskDispatcher,
    ) -> Self {
        TaskListener {
            executor,
            listener,
            dispatcher,
        }
    }

    // only do that receive message and then 'inform'
    pub async fn run(mut self) {
        loop {
            match self.listener.recv().await {
                Some(event) => {
                    info!("[TaskListener] received event: {:?}", event);
                    let device = self.dispatcher.find_device(&event);
                    let func = self.dispatcher.find_func(&event);
                    let router = self.executor.get_router();

                    tokio::spawn(async move {
                        match execute(router, device, func).await {
                            Ok(_) => {
                                info!("[TaskListener] executed successfully");
                            }
                            Err(e) => {
                                error!("[TaskListener] executed failed {:?}", e);
                            }
                        }
                    });
                }
                None => {
                    info!("[TaskListener] received None event");
                    break;
                }
            }
        }
    }
}
