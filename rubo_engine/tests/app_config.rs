use std::path::Path;

use rubo_engine::config::{AppConfig, ConfigFileFormat, ConfigStore};

#[test]
fn app_config_defaults_to_config_directory() {
    let config = AppConfig::default();

    assert_eq!(config.name(), "rubo_engine");
    assert_eq!(config.config_path(), Path::new("config"));
    assert_eq!(config.profile(), "");
    assert_eq!(config.config_dir(), Path::new("config"));
    assert_eq!(config.config_format(), ConfigFileFormat::Json);
    assert!(config.web().enabled());
    assert!(config.web().output_image());
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
            profile = "orangepi"
            config_format = "yaml"

            [web]
            enabled = false
            output_image = false
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
    assert_eq!(config.profile(), "orangepi");
    assert_eq!(config.config_dir(), Path::new("custom_config/orangepi"));
    assert_eq!(config.config_format(), ConfigFileFormat::Yaml);
    assert!(!config.web().enabled());
    assert!(!config.web().output_image());
    assert_eq!(config.web().host(), "0.0.0.0");
    assert_eq!(config.web().port(), 18080);
    assert!(config.log().enabled());
    assert_eq!(config.log().level(), "debug");
}

#[test]
fn app_config_updates_profile_and_discovers_profile_directories() {
    let temp = tempfile::tempdir().unwrap();
    let application_dir = temp.path().join("config");
    std::fs::create_dir_all(application_dir.join("orangepi")).unwrap();
    std::fs::create_dir_all(application_dir.join("raspberrypi")).unwrap();
    std::fs::create_dir_all(application_dir.join("empty")).unwrap();
    std::fs::write(
        application_dir.join("application.yaml"),
        "name: demo\nconfig_path: config\nprofile: orangepi\nconfig_format: toml\n",
    )
    .unwrap();
    std::fs::write(application_dir.join("orangepi/source.toml"), "").unwrap();
    std::fs::write(application_dir.join("raspberrypi/sink.toml"), "").unwrap();

    let mut config = ConfigStore::load_app_config(&application_dir).unwrap();
    assert_eq!(
        ConfigStore::list_profiles(temp.path(), &config).unwrap(),
        vec!["orangepi".to_string(), "raspberrypi".to_string()]
    );

    config.set_profile("raspberrypi");
    ConfigStore::save_app_config(&application_dir, &config).unwrap();
    let saved = ConfigStore::load_app_config(&application_dir).unwrap();

    assert_eq!(saved.profile(), "raspberrypi");
    assert_eq!(saved.config_dir(), Path::new("config/raspberrypi"));
}
