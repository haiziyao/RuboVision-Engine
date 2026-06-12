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
                    let router = self.executor.get_router();
                    let device = match self.dispatcher.find_device(&event) {
                        Ok(device) => device,
                        Err(error) => {
                            tracing::error!("[TaskListener] device dispatch failed: {error:#}");
                            for route_error in router
                                .report_error(format!("task dispatch failed: {error:#}"))
                                .await
                            {
                                tracing::error!(
                                    "[TaskListener] dispatch error message failed: {route_error:#}"
                                );
                            }
                            continue;
                        }
                    };
                    let func = match self.dispatcher.find_func(&event) {
                        Ok(func) => func,
                        Err(error) => {
                            tracing::error!("[TaskListener] function dispatch failed: {error:#}");
                            for route_error in router
                                .report_error(format!("task dispatch failed: {error:#}"))
                                .await
                            {
                                tracing::error!(
                                    "[TaskListener] dispatch error message failed: {route_error:#}"
                                );
                            }
                            continue;
                        }
                    };
                    let runtime_param = self.dispatcher.runtime_param(&event);

                    tokio::spawn(async move {
                        match execute(router, device, func, runtime_param).await {
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
