use std::path::Path;

use rubo_engine::config::{AppConfig, ConfigFileFormat, ConfigStore};

#[test]
fn app_config_defaults_to_config_directory() {
    let config = AppConfig::default();

    assert_eq!(config.name(), "rubo_engine");
    assert_eq!(config.config_path(), Path::new("config"));
    assert_eq!(config.config_format(), ConfigFileFormat::Json);
    assert!(config.web().enabled());
    assert_eq!(config.web().host(), "127.0.0.1");
    assert_eq!(config.web().port(), 3888);
    assert!(config.log().enabled());
    assert_eq!(config.log().level(), "info");
}

#[test]
fn app_config_loads_application_file() {
    let temp = tempfile::tempdir().unwrap();
    std::fs::write(
        temp.path().join("application.toml"),
        r#"
            name = "demo"
            config_path = "custom_config"
            config_format = "yaml"

            [web]
            enabled = false
            host = "0.0.0.0"
            port = 18080

            [log]
            enabled = true
            level = "debug"
        "#,
    )
    .unwrap();

    let config = ConfigStore::load_app_config(temp.path()).unwrap();

    assert_eq!(config.name(), "demo");
    assert_eq!(config.config_path(), Path::new("custom_config"));
    assert_eq!(config.config_format(), ConfigFileFormat::Yaml);
    assert!(!config.web().enabled());
    assert_eq!(config.web().host(), "0.0.0.0");
    assert_eq!(config.web().port(), 18080);
    assert!(config.log().enabled());
    assert_eq!(config.log().level(), "debug");
}
