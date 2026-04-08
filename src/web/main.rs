use anyhow::{Context, Result};
use tracing::info;
use crate::config::WebConfig;
use crate::web::router::router;
use tokio::sync::mpsc;
use crate::web::state::WebState;
use super::WebMessage;

async fn consume_web_messages(
    mut rx: mpsc::Receiver<WebMessage>,
    state: WebState,
) {
    info!(target: "web", "Web message consumer started");

    while let Some(msg) = rx.recv().await {
        state.push_message(msg).await;
    }

    info!(target: "web", "Web message channel closed");
}

pub async fn run(config: WebConfig, rx: mpsc::Receiver<WebMessage>) -> Result<()> {
    let state = WebState::new(20);

    let consumer_state = state.clone();
    tokio::spawn(async move {
        consume_web_messages(rx, consumer_state).await;
    });

    let app = router(state);
    info!(target: "web", "Web app starting...");

    let addr = format!("{}:{}", config.host, config.port);
    info!(target: "web", "Web server listening on: {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .with_context(|| format!("failed to bind web listener on {addr}"))?;
    info!(target: "web", "Web server is listening ...");

    axum::serve(listener, app)
        .await
        .context("web server exited unexpectedly")?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use tokio::sync::mpsc;
    use tracing::info;
    use crate::config::WebConfig;
    use crate::utils::web_tools::image_to_data_url;
    use crate::web::main::run;
    use crate::web::WebMessage;

    #[tokio::test]
    async fn test_run_server() {
        let config = WebConfig {
            on: true,
            host: "127.0.0.1".to_string(),
            port: 3000,
        };

        let (tx, rx) = mpsc::channel::<WebMessage>(32);

        let handler = tokio::spawn(async move {
            let _ = run(config, rx).await;
        });

        info!("open http://127.0.0.1:3000");

        tx.send(WebMessage::ok(String::from("hello world")))
            .await.expect("not send");

        let image_base64 =
            image_to_data_url("static/image/a.jpg").unwrap();
        let image2 =
            image_to_data_url("static/image/b.jpg").unwrap();

        tx.send(WebMessage::ok("first message: text only"))
            .await
            .expect("send first message failed");

        tx.send(WebMessage::with_image(
            "second message: with image",
            image_base64,
        ))
            .await
            .expect("send second message failed");

        tx.send(WebMessage::closed())
            .await
            .expect("send closed message failed");
        for _i in 0..5 {
            tx.send(WebMessage::with_image(
                "second message: with image",
                image2.clone(),
            ))
                .await
                .expect("send second message failed");
        }

        tokio::time::sleep(std::time::Duration::from_secs(300)).await;

        handler.await.expect("panicked");
    }
}
