use std::collections::HashMap;
use crate::config::{BindingsConfig};
use crate::device::{Device, DeviceMap};
use crate::web::WebMessage;
use crate::source::{UartSource, TimerSource, WebSource, LoopSource, Source, Event};

use tracing::{info};
use anyhow::Result;
use crate::func::FuncWorkerMap;
use crate::init::{TaskDispatcher, TaskExecutor, TaskListener};



pub fn register_source(bindings_config: BindingsConfig,tx: tokio::sync::mpsc::Sender<Event>) ->Result<()> {
    let BindingsConfig{
        // TODO
        // 这个命名给我带来了不小的困扰啊
        // 这下真成了: 我写的代码只能由我自己看懂了
        uart_source: uart_source,
        timer_source:  timer_source,
        loop_source: loop_source,
        web_source: web_source,
    } = bindings_config;
    info!("Source initializing ...");
    if !uart_source.is_empty(){
        let mut uart_sourcer = UartSource::new();
        uart_sourcer.set_sender(tx.clone());
        info!("UartSource set to {:?}", uart_source);
        tokio::spawn(async move  {
            uart_sourcer.start(uart_source).await.expect("UartSource work failed ...")
        });
    }
    if !timer_source.is_empty() {
        let mut timer_sourcer = TimerSource::new();
        timer_sourcer.set_sender(tx.clone());
        info!("TimerSource set to {:?}", timer_source);
        tokio::spawn(async move{
            timer_sourcer.start(timer_source).await.expect("TimerSource work failed ...")
        });
    }
    if !loop_source.is_empty() {
        let mut loop_sourcer = LoopSource::new();
        loop_sourcer.set_sender(tx.clone());
        info!("LoopSource set to {:?}", loop_source);
        tokio::spawn(async move{
            loop_sourcer.start(loop_source).await.expect("LoopSource work failed ...")
        });
    }

    // TODO: 后期需要加上web调试的东西，所以会加一个 tokio::sync::mpsc::channel
    if !web_source.is_empty() {
        let mut web_sourcer = WebSource::new();
        web_sourcer.set_sender(tx.clone());
        info!("WebSource set to {:?}", web_source);
        tokio::spawn(async move{
           web_sourcer.start(web_source).await.expect("WebSource work failed ...")
        });
    }


    info!("All the Source has been registered");
    Ok(())
}


pub fn register_listener(listener_receiver:tokio::sync::mpsc::Receiver<Event>,
                          exeutor_sender:tokio::sync::mpsc::Sender<WebMessage>,
                          bindings_config: BindingsConfig,func_worker_map: FuncWorkerMap,
                                device_map: DeviceMap) ->TaskListener {
    let dispatcher = TaskDispatcher::new(bindings_config, func_worker_map,device_map);
    let executor = TaskExecutor::new(exeutor_sender);
    TaskListener::new(executor, listener_receiver, dispatcher)
}
