use rubo_engine::ConfigError;

#[test]
fn config_error_display_keeps_error_category_and_message() {
    let error = ConfigError::ConfigMismatch {
        message: "active config does not match declared config".to_string(),
    };

    assert_eq!(
        error.to_string(),
        "config mismatch: active config does not match declared config"
    );
}

#[test]
fn config_error_converts_to_config_load_error() {
    let error: ConfigError = config::ConfigError::Message("missing source".to_string()).into();

    assert!(matches!(error, ConfigError::ConfigLoad { .. }));
    assert!(error.to_string().contains("missing source"));
}

#[test]
fn io_error_converts_to_config_write_error() {
    let error: ConfigError = std::io::Error::new(std::io::ErrorKind::Other, "disk full").into();

    assert!(matches!(error, ConfigError::ConfigWrite { .. }));
    assert!(error.to_string().contains("disk full"));
}
