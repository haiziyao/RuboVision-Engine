use tracing::info;
use tracing_subscriber::EnvFilter;

use super::success_text;

const STARTUP_BANNER: &str = r#"
 ____        _           _____             _
|  _ \ _   _| |__   ___| ____|_ __   __ _(_)_ __   ___
| |_) | | | | '_ \ / _ \  _| | '_ \ / _` | | '_ \ / _ \
|  _ <| |_| | |_) | (_) | |___| | | | (_| | | | | |  __/
|_| \_\\__,_|_.__/ \___/|_____|_| |_|\__, |_|_| |_|\___|
                                      |___/

                            RuboEngine"#;

pub fn init_tracing(default_filter: impl AsRef<str>) {
    let filter =
        EnvFilter::try_new(default_filter.as_ref()).unwrap_or_else(|_| EnvFilter::new("info"));

    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_ansi(true)
        .try_init();
    info!("{}", success_text(STARTUP_BANNER));
}

#[cfg(test)]
mod tests {
    use super::STARTUP_BANNER;

    #[test]
    fn startup_banner_names_rubo_engine() {
        assert!(STARTUP_BANNER.contains("RuboEngine"));
        assert!(!STARTUP_BANNER.contains("RuboVision"));
    }
}
