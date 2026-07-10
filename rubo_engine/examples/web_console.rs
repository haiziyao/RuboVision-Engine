use rubo_engine::{Engine, config::AppConfig, config::RuboConfig};

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let mut engine = Engine::new(".", AppConfig::default(), RuboConfig::default());
    if let Err(error) = engine.serve_web().await {
        eprintln!("web server failed: {error:?}");
    }
}
