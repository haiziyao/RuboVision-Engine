use std::sync::{Arc, Mutex};

use anyhow::Result;
use tokio::sync::mpsc;

use crate::config::{GpioConfig, ReturnTargets};

use super::{
    GpioOutput, MessageRouter, MessageSink, PinBackend, TaskOutput, UartSink, WebSink,
    apply_gpio_message, start_gpio_worker_with_backend,
};

#[tokio::test]
async fn router_keeps_web_delivery_when_uart_sink_fails() {
    let (web_tx, mut web_rx) = mpsc::channel(2);
    let (uart_tx, uart_rx) = mpsc::channel(2);
    drop(uart_rx);

    let router = MessageRouter::new(
        Some(WebSink::new(web_tx)),
        Some(UartSink::new(uart_tx)),
        None,
    );
    let targets = ReturnTargets {
        web: true,
        uart: true,
        gpio: None,
    };

    let errors = router
        .route(&targets, &TaskOutput::value("finished", "red"))
        .await;

    assert_eq!(errors.len(), 1);
    assert_eq!(web_rx.recv().await.expect("web message").text, "finished");
}

#[tokio::test]
async fn uart_sink_appends_exactly_one_newline() {
    let (tx, mut rx) = mpsc::channel(2);
    let sink = UartSink::new(tx);

    sink.send_value("42").await.expect("send UART value");
    sink.send_value("ready\n").await.expect("send UART line");

    assert_eq!(rx.recv().await.expect("first UART write"), b"42\n");
    assert_eq!(rx.recv().await.expect("second UART write"), b"ready\n");
}

#[test]
fn gpio_messages_apply_active_low_start_and_finish_levels() {
    let levels = Arc::new(Mutex::new(Vec::new()));
    let mut backend = FakePinBackend {
        levels: levels.clone(),
    };
    let config = GpioConfig {
        on: true,
        active_low: true,
        run_pin: 27,
        signals: [("color".to_string(), 17)].into_iter().collect(),
    };

    apply_gpio_message(
        &mut backend,
        &config,
        GpioOutput::TaskStarted("color".to_string()),
    )
    .expect("start GPIO task");
    apply_gpio_message(
        &mut backend,
        &config,
        GpioOutput::TaskFinished("color".to_string()),
    )
    .expect("finish GPIO task");

    assert_eq!(
        *levels.lock().expect("GPIO levels"),
        vec![(27, false), (17, false), (17, true), (27, true)]
    );
}

#[tokio::test]
async fn gpio_sink_worker_processes_channel_messages() {
    let levels = Arc::new(Mutex::new(Vec::new()));
    let config = GpioConfig {
        on: true,
        active_low: true,
        run_pin: 27,
        signals: [("color".to_string(), 17)].into_iter().collect(),
    };
    let sink = start_gpio_worker_with_backend(
        config,
        FakePinBackend {
            levels: levels.clone(),
        },
    );

    sink.send(GpioOutput::TaskStarted("color".to_string()))
        .await
        .expect("queue task start");
    sink.send(GpioOutput::TaskFinished("color".to_string()))
        .await
        .expect("queue task finish");

    tokio::time::timeout(std::time::Duration::from_secs(1), async {
        loop {
            if levels.lock().expect("GPIO levels").len() >= 4 {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("GPIO worker timeout");

    assert_eq!(
        *levels.lock().expect("GPIO levels"),
        vec![(27, false), (17, false), (17, true), (27, true)]
    );
}

struct FakePinBackend {
    levels: Arc<Mutex<Vec<(u8, bool)>>>,
}

impl PinBackend for FakePinBackend {
    fn set_level(&mut self, pin: u8, high: bool) -> Result<()> {
        self.levels.lock().expect("GPIO levels").push((pin, high));
        Ok(())
    }
}
