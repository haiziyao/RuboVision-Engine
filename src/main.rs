use std::path::Path;

use rubo_engine::{
    config::ConfigStore,
    log::{error_text, init_tracing, warn_text},
    serve,
};
use rubo_vision::{build_engine, default_app_config, default_rubo_config};

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let root = Path::new(".");
    let app_config = match ConfigStore::load_app_config(root.join("config")) {
        Ok(config) => config,
        Err(error) => {
            eprintln!(
                "{}",
                error_text(format!(
                    "rubo_vision.app_config.load.error error={error}; using defaults"
                ))
            );
            default_app_config()
        }
    };
    if app_config.log().enabled() {
        init_tracing(app_config.log().level());
    }

    let declared_config = match default_rubo_config(&app_config) {
        Ok(config) => config,
        Err(error) => {
            eprintln!(
                "{}",
                error_text(format!("rubo_vision.declared_config.error error={error}"))
            );
            return;
        }
    };
    let active_config = match ConfigStore::load_or_init_config(root, &app_config, &declared_config)
    {
        Ok(config) => config,
        Err(error) => {
            eprintln!(
                "{}",
                error_text(format!("rubo_vision.config.load.error error={error}"))
            );
            return;
        }
    };
    let web_enabled = app_config.web().enabled();
    let mut engine = build_engine(root, app_config, active_config);
    if !web_enabled {
        let config_valid = engine.config().validate();
        if !config_valid {
            eprintln!(
                "{}",
                error_text("rubo_vision.config.invalid; headless runtime cannot start")
            );
            return;
        }
        if let Err(error) = engine.run(1024).await {
            eprintln!(
                "{}",
                error_text(format!("rubo_vision.runtime.error error={error}"))
            );
        }
        return;
    }
    engine.prepare_web();
    let Some(web_state) = engine.web_state().cloned() else {
        eprintln!("{}", error_text("rubo_vision.web.disabled"));
        return;
    };
    let runtime_config = web_state.runtime_config();
    let config_valid = runtime_config
        .read()
        .expect("rubo_vision runtime config lock poisoned")
        .validate();

    let mut runtime = engine.runtime(1024);
    if config_valid {
        runtime.start();
    } else {
        eprintln!(
            "{}",
            warn_text("rubo_vision.config.invalid; web remains available and runtime is stopped")
        );
    }

    if let Err(error) = serve(web_state).await {
        eprintln!(
            "{}",
            error_text(format!("rubo_vision.web.serve.error error={error:?}"))
        );
    }
    runtime.stop().await;
}
