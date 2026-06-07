use anyhow::{Context, Result};
use embed::Assets;
use tokio::sync::mpsc;
use tracing::{info, warn};

use crate::config::RuntimeConfig;
use crate::device::register_device;
use crate::func::register_func;
use crate::init::register_source;
use crate::init::{init_logging, register_listener};
use crate::message::{MessageRouter, UartSink, WebSink, start_gpio_sink, start_uart_transport};
use crate::source::Event;
use crate::web::WebMessage;

mod config;
mod device;
mod embed;
mod func;
mod init;
mod message;
mod source;
mod utils;
mod web;

pub async fn run() -> Result<()> {
    // init logger
    let _guard = init_logging();
    info!("Starting Logger Guard ... ");

    // read config
    let cfg = config::load_config().with_context(|| "failed to load config")?;
    info!("config loaded ... ");

    let RuntimeConfig {
        app: _app_config,
        message,
        bindings: bindings_config,
        functions,
        devices,
    } = cfg;

    let web_config = message.web.clone();
    let uart_config = message.uart.clone();
    let uart_channels = if uart_config.on {
        match start_uart_transport(&uart_config) {
            Ok(channels) => Some(channels),
            Err(error) => {
                warn!("UART transport unavailable: {error:#}");
                None
            }
        }
    } else {
        None
    };
    let (uart_incoming, uart_outgoing) = match uart_channels {
        Some(channels) => (Some(channels.incoming), Some(channels.outgoing)),
        None => (None, None),
    };

    // start RuboVision
    print_banner();

    // TODO: use bindings_config to register
    let (source_sender, listener_receiver) = mpsc::channel::<Event>(32);
    match register_source(bindings_config, source_sender, uart_incoming)
        .with_context(|| "Start Failed ... caused by register_source")
    {
        Err(e) => {
            warn!("register source failed {e}");
        }
        _ => {
            info!("sources are register successfully");
        }
    };

    info!("Register Function Started ... ");
    let func_worker_map = register_func(functions).context("failed to register functions")?;
    info!("Register Device Started ... ");
    let device_map = register_device(devices);

    let (executor_sender, returner_receiver) = mpsc::channel::<WebMessage>(32);
    let gpio_sink = if message.gpio.on {
        match start_gpio_sink(message.gpio.clone()) {
            Ok(sink) => Some(sink),
            Err(error) => {
                warn!("GPIO sink unavailable: {error:#}");
                None
            }
        }
    } else {
        None
    };
    let message_router = MessageRouter::new(
        Some(WebSink::new(executor_sender)),
        uart_outgoing.map(UartSink::new),
        gpio_sink,
    );

    // register listener(dispatcher,executor) returner
    tokio::spawn(async move {
        register_listener(
            listener_receiver,
            message_router,
            func_worker_map,
            device_map,
        )
        .run()
        .await
    });

    // start Web Debugger
    if web_config.on {
        info!("Web Debugger enabled ...  starting ...");
        info!("Web Channel starting ...");
        let _web_handler = tokio::spawn(async move {
            info!("Web handler starting...");
            let _ = web::run(web_config, returner_receiver).await;
        });
    } else {
        info!("Web Debugger disabled ... draining task messages to logs");
        tokio::spawn(async move {
            drain_web_messages(returner_receiver).await;
        });
    }

    // there is a bug (maybe) caused by : Should the new thread to start the listener?

    tokio::signal::ctrl_c().await?;
    Ok(())
}

async fn drain_web_messages(mut rx: mpsc::Receiver<WebMessage>) {
    while let Some(msg) = rx.recv().await {
        info!(
            "web disabled, task message dropped after logging: {:?}",
            msg
        );
    }
}

pub fn print_banner() {
    let file = Assets::get("project/banner.txt").expect("banner not found");

    let content = std::str::from_utf8(file.data.as_ref()).expect("invalid utf8");

    info!("\n{}", content);
}

#[cfg(test)]
#[tokio::test]
#[ignore = "starts the full runtime and waits for ctrl-c"]
async fn test_run() {
    run().await.unwrap();
}
