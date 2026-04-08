
use anyhow::{Context, Result};
use tracing::{info, debug, span};
use tokio::sync::mpsc;
use embed::Assets;


use crate::config::{RuntimeConfig, WebConfig};
use crate::init::{init_logging, register_listener};
use crate::web::WebMessage;
use crate::source::Event;
use crate::init::register_source;
use crate::config::FuncParamConfig;
use crate::config::DeviceParamConfig;
use crate::device::register_device;
use crate::func::register_func;

mod config;
mod source;
mod embed;
mod init;
mod web;
mod utils;
mod func;
mod device;

pub async fn run() -> Result<()> {

    // init logger
    let _guard = init_logging();
    info!("Starting Logger Guard ... ");

    // read config
    let cfg = config::load_config()
        .with_context(|| "failed to load config")?;
    info!("config loaded ... ");

    let RuntimeConfig {
        app: app_config,
        web: web_config,
        bindings: bindings_config,
        func_param_config: func_param_config,
        device_param_config: device_param_config,
    } = cfg;

    // start RuboVision
    print_banner();
    
    // TODO: use bindings_config to register
    let (source_sender,listener_receiver) = mpsc::channel::<Event>(32);
    register_source(bindings_config.clone(),source_sender).with_context(||"Start Failed ... caused by register_source").unwrap();

    info!("Register Function Started ... ");
    let func_worker_map = register_func(func_param_config);
    info!("Register Device Started ... ");
    let device_map = register_device(device_param_config);

    // register listener(dispatcher,executor) returner
    let (executor_sender,returner_receiver) = mpsc::channel::<WebMessage>(32);
    tokio::spawn(async move {register_listener(listener_receiver,executor_sender,
                                   bindings_config.clone(),func_worker_map,device_map)
                     .run().await});

    // start Web Debugger
    if(web_config.on) {
        info!("Web Debugger enabled ...  starting ...");
        info!("Web Channel starting ...");
        let web_handler = tokio::spawn(async move {
            info!("Web handler starting...");
            web::run(web_config,returner_receiver).await;
        });
    }

    // there is a bug (maybe) caused by : Should the new thread to start the listener?

    tokio::signal::ctrl_c().await?;
    Ok(())
}





pub fn print_banner() {
    let file = Assets::get("project/banner.txt")
        .expect("banner not found");

    let content = std::str::from_utf8(file.data.as_ref())
        .expect("invalid utf8");

    info!("\n{}", content);
     
}




#[cfg(test)]
#[tokio::test]
async fn test_run(){
    run().await.unwrap();
}
