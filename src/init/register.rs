use crate::config::BindingsConfig;
use crate::device::DeviceMap;
use crate::source::{Event, LoopSource, Source, TimerSource, UartSource, WebSource};

use crate::func::FuncWorkerMap;
use crate::init::{TaskDispatcher, TaskExecutor, TaskListener};
use crate::message::MessageRouter;
use anyhow::Result;
use tracing::{error, info};

pub fn register_source(
    bindings_config: BindingsConfig,
    tx: tokio::sync::mpsc::Sender<Event>,
    uart_incoming: Option<tokio::sync::mpsc::Receiver<Vec<u8>>>,
) -> Result<()> {
    let BindingsConfig {
        // TODO
        // 这个命名给我带来了不小的困扰啊
        // 这下真成了: 我写的代码只能由我自己看懂了
        uart_source,
        timer_source,
        loop_source,
        debug_source,
    } = bindings_config;
    info!("Source initializing ...");
    if !uart_source.is_empty() {
        match uart_incoming {
            Some(incoming) => {
                let mut uart_sourcer = UartSource::new();
                uart_sourcer.set_sender(tx.clone());
                info!("UartSource set to {:?}", uart_source);
                tokio::spawn(async move {
                    if let Err(e) = uart_sourcer.start(uart_source, incoming).await {
                        error!("UartSource work failed: {e:#}");
                    }
                });
            }
            None => error!("UartSource bindings configured but UART transport is unavailable"),
        }
    }
    if !timer_source.is_empty() {
        let mut timer_sourcer = TimerSource::new();
        timer_sourcer.set_sender(tx.clone());
        info!("TimerSource set to {:?}", timer_source);
        tokio::spawn(async move {
            if let Err(e) = timer_sourcer.start(timer_source).await {
                error!("TimerSource work failed: {e:#}");
            }
        });
    }
    if !loop_source.is_empty() {
        let mut loop_sourcer = LoopSource::new();
        loop_sourcer.set_sender(tx.clone());
        info!("LoopSource set to {:?}", loop_source);
        tokio::spawn(async move {
            if let Err(e) = loop_sourcer.start(loop_source).await {
                error!("LoopSource work failed: {e:#}");
            }
        });
    }

    // TODO: 后期需要加上web调试的东西，所以会加一个 tokio::sync::mpsc::channel
    if !debug_source.is_empty() {
        let mut web_sourcer = WebSource::new();
        web_sourcer.set_sender(tx.clone());
        info!("WebSource set to {:?}", debug_source);
        tokio::spawn(async move {
            if let Err(e) = web_sourcer.start(debug_source).await {
                error!("WebSource work failed: {e:#}");
            }
        });
    }

    info!("All the Source has been registered");
    Ok(())
}

pub fn register_listener(
    listener_receiver: tokio::sync::mpsc::Receiver<Event>,
    message_router: MessageRouter,
    func_worker_map: FuncWorkerMap,
    device_map: DeviceMap,
) -> TaskListener {
    let dispatcher = TaskDispatcher::new(func_worker_map, device_map);
    let executor = TaskExecutor::new(message_router);
    TaskListener::new(executor, listener_receiver, dispatcher)
}
