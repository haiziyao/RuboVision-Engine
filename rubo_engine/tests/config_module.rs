use rubo_engine::{
    ConfigError,
    config::{
        AppConfig, BindingConfig, ConfigAccess, ConfigStore, ConfigWriter, DeviceConfig,
        FuncConfig, RuboConfig, SinkConfig, SourceConfig, save_binding_update, save_device_update,
        save_func_update, save_sink_update, save_source_update, save_update, update_binding,
        update_device, update_func, update_sink, update_source,
    },
};
use std::sync::{Arc, Mutex};
use tracing::subscriber::with_default;
use tracing_subscriber::fmt::MakeWriter;

static WRITER_TEST_LOCK: Mutex<()> = Mutex::new(());

#[test]
fn source_config_reads_custom_values() {
    let config = SourceConfig::new("timer")
        .set("interval_ms", 1000_u64)
        .set("description", "tick source");

    assert_eq!(config.id(), "timer");
    assert_eq!(config.get::<u64>("interval_ms").unwrap(), 1000);
    assert_eq!(
        config
            .get_or("missing", "default description".to_string())
            .unwrap(),
        "default description"
    );
    assert!(config.contains("description"));

    let error = config.get::<u64>("missing").unwrap_err();
    assert!(matches!(error, ConfigError::ConfigFormat { .. }));
}

#[test]
fn binding_config_describes_source_func_devices_and_sinks() {
    let binding = BindingConfig::new("inspect_frame")
        .source("timer", "tick")
        .func("inspect")
        .device("demo_camera")
        .sink("console")
        .sink("web")
        .debug(true);

    assert_eq!(binding.id(), "inspect_frame");
    assert_eq!(binding.source_ref().id(), "timer");
    assert_eq!(binding.source_ref().event(), "tick");
    assert_eq!(binding.func_ref(), "inspect");
    assert_eq!(binding.devices(), &["demo_camera".to_string()]);
    assert_eq!(binding.sinks(), &["console".to_string(), "web".to_string()]);
    assert!(binding.debug_enabled());
}

#[test]
fn rubo_config_validate_returns_true_when_binding_references_exist() {
    let config = sample_rubo_config(1000);

    assert!(config.validate());
}

#[test]
fn rubo_config_validate_returns_false_when_binding_is_not_runnable() {
    let mut missing_source = sample_rubo_config(1000);
    update_binding(
        &mut missing_source,
        BindingConfig::new("broken")
            .source("missing_source", "tick")
            .func("inspect")
            .sink("console"),
    );
    let mut missing_event = sample_rubo_config(1000);
    update_binding(
        &mut missing_event,
        BindingConfig::new("missing_event")
            .source("timer", "")
            .func("inspect")
            .sink("console"),
    );

    assert!(!missing_source.validate());
    assert!(!missing_event.validate());
}

#[test]
fn loader_reads_module_config_files() {
    let temp = tempfile::tempdir().unwrap();
    let config_dir = temp.path().join("config");
    std::fs::create_dir_all(&config_dir).unwrap();
    std::fs::write(
        config_dir.join("source.toml"),
        r#"
            [timer]
            kind = "interval"
            interval_ms = 1000
        "#,
    )
    .unwrap();
    std::fs::write(
        config_dir.join("device.toml"),
        r#"
            [demo_camera]
            kind = "virtual_camera"
            width = 640
        "#,
    )
    .unwrap();
    std::fs::write(
        config_dir.join("function.toml"),
        r#"
            [inspect]
            score_scale = 0.8
        "#,
    )
    .unwrap();
    std::fs::write(
        config_dir.join("sink.toml"),
        r#"
            [console]
            kind = "channel"
            prefix = "[console]"
        "#,
    )
    .unwrap();
    std::fs::write(
        config_dir.join("binding.toml"),
        r#"
            [inspect_frame]
            source = { id = "timer", event = "tick" }
            function = "inspect"
            sinks = ["console"]
            debug = true

            devices = ["demo_camera"]
        "#,
    )
    .unwrap();

    let config = ConfigStore::load_active_config(temp.path().join("config")).unwrap();
    let sources = config.sources();
    let devices = config.devices();
    let funcs = config.funcs();
    let sinks = config.sinks();
    let bindings = config.bindings();

    assert_eq!(sources["timer"].get::<u64>("interval_ms").unwrap(), 1000);
    assert_eq!(sources["timer"].kind_ref(), "interval");
    assert_eq!(devices["demo_camera"].kind(), "virtual_camera");
    assert_eq!(devices["demo_camera"].get::<u32>("width").unwrap(), 640);
    assert_eq!(funcs["inspect"].get::<f64>("score_scale").unwrap(), 0.8);
    assert_eq!(
        sinks["console"].get::<String>("prefix").unwrap(),
        "[console]"
    );
    assert_eq!(sinks["console"].kind_ref(), "channel");
    assert_eq!(bindings["inspect_frame"].source_ref().event(), "tick");
    assert_eq!(bindings["inspect_frame"].devices()[0], "demo_camera");
    assert!(bindings["inspect_frame"].debug_enabled());
}

#[test]
fn store_logs_active_config_load_span() {
    let _guard = WRITER_TEST_LOCK.lock().unwrap();
    let temp = tempfile::tempdir().unwrap();
    let config_dir = temp.path().join("config");
    std::fs::create_dir_all(&config_dir).unwrap();
    let logs = Arc::new(Mutex::new(Vec::new()));
    let subscriber = tracing_subscriber::fmt()
        .with_ansi(false)
        .with_writer(TestLogWriter::new(Arc::clone(&logs)))
        .finish();

    with_default(subscriber, || {
        ConfigStore::load_active_config(&config_dir).unwrap();
    });

    let logs = String::from_utf8(logs.lock().unwrap().clone()).unwrap();
    assert!(logs.contains("active_config.load{"));
    assert!(logs.contains("config.active.load.start"));
    assert!(logs.contains("config.active.load.finish"));
}

#[test]
fn store_logs_app_config_load_span() {
    let _guard = WRITER_TEST_LOCK.lock().unwrap();
    let temp = tempfile::tempdir().unwrap();
    let logs = Arc::new(Mutex::new(Vec::new()));
    let subscriber = tracing_subscriber::fmt()
        .with_ansi(false)
        .with_writer(TestLogWriter::new(Arc::clone(&logs)))
        .finish();

    with_default(subscriber, || {
        ConfigStore::load_app_config(temp.path()).unwrap();
    });

    let logs = String::from_utf8(logs.lock().unwrap().clone()).unwrap();
    assert!(logs.contains("app_config.load{"));
    assert!(logs.contains("config.app.load.start"));
    assert!(logs.contains("config.app.load.default"));
}

#[test]
fn loader_rejects_multiple_formats_for_same_module() {
    let temp = tempfile::tempdir().unwrap();
    let config_dir = temp.path().join("config");
    std::fs::create_dir_all(&config_dir).unwrap();
    std::fs::write(config_dir.join("source.toml"), "").unwrap();
    std::fs::write(config_dir.join("source.yaml"), "").unwrap();

    let error = ConfigStore::load_active_config(config_dir).unwrap_err();

    assert!(matches!(error, ConfigError::ConfigFormat { .. }));
    assert!(error.to_string().contains("multiple source config files"));
}

#[test]
fn update_replaces_existing_config_and_inserts_missing_config() {
    let mut config = RuboConfig::default();

    update_source(
        &mut config,
        SourceConfig::new("timer")
            .kind("interval")
            .set("interval_ms", 1000_u64),
    );
    update_source(
        &mut config,
        SourceConfig::new("timer").set("interval_ms", 2000_u64),
    );
    update_binding(
        &mut config,
        BindingConfig::new("inspect")
            .source("timer", "tick")
            .func("inspect")
            .sink("console"),
    );

    assert_eq!(
        config.sources()["timer"].get::<u64>("interval_ms").unwrap(),
        2000
    );
    assert_eq!(config.bindings()["inspect"].func_ref(), "inspect");
}

#[test]
fn update_logs_config_update_span() {
    let _guard = WRITER_TEST_LOCK.lock().unwrap();
    let mut config = RuboConfig::default();
    let logs = Arc::new(Mutex::new(Vec::new()));
    let subscriber = tracing_subscriber::fmt()
        .with_ansi(false)
        .with_writer(TestLogWriter::new(Arc::clone(&logs)))
        .finish();

    with_default(subscriber, || {
        update_source(&mut config, SourceConfig::new("timer"));
    });

    let logs = String::from_utf8(logs.lock().unwrap().clone()).unwrap();
    assert!(logs.contains("config.update{"));
    assert!(logs.contains("config.update.source.insert"));
}

#[test]
fn update_save_writes_active_config_and_chain_json_after_memory_update() {
    let _guard = WRITER_TEST_LOCK.lock().unwrap();
    let temp = tempfile::tempdir().unwrap();
    let mut config = RuboConfig::default();

    update_source(
        &mut config,
        SourceConfig::new("timer").set("interval_ms", 3000_u64),
    );
    update_binding(
        &mut config,
        BindingConfig::new("inspect_frame")
            .source("timer", "tick")
            .func("inspect")
            .sink("console"),
    );
    save_update(temp.path(), &AppConfig::default(), &config).unwrap();

    let loaded = ConfigStore::load_active_config(temp.path().join("config")).unwrap();
    assert_eq!(
        loaded.sources()["timer"].get::<u64>("interval_ms").unwrap(),
        3000
    );
    assert!(temp.path().join("chain.json").exists());
}

#[test]
fn fine_grained_update_save_writes_only_requested_module_and_chain_json() {
    let _guard = WRITER_TEST_LOCK.lock().unwrap();
    let temp = tempfile::tempdir().unwrap();
    let config = sample_rubo_config(1000);
    let app_config = AppConfig::default();

    let source_root = temp.path().join("source_root");
    save_source_update(&source_root, &app_config, &config).unwrap();
    assert!(source_root.join("config").join("source.json").exists());
    assert!(!source_root.join("config").join("device.json").exists());
    assert!(source_root.join("chain.json").exists());

    let device_root = temp.path().join("device_root");
    save_device_update(&device_root, &app_config, &config).unwrap();
    assert!(device_root.join("config").join("device.json").exists());
    assert!(!device_root.join("config").join("source.json").exists());
    assert!(device_root.join("chain.json").exists());

    let func_root = temp.path().join("func_root");
    save_func_update(&func_root, &app_config, &config).unwrap();
    assert!(func_root.join("config").join("function.json").exists());
    assert!(!func_root.join("config").join("source.json").exists());
    assert!(func_root.join("chain.json").exists());

    let sink_root = temp.path().join("sink_root");
    save_sink_update(&sink_root, &app_config, &config).unwrap();
    assert!(sink_root.join("config").join("sink.json").exists());
    assert!(!sink_root.join("config").join("source.json").exists());
    assert!(sink_root.join("chain.json").exists());

    let binding_root = temp.path().join("binding_root");
    save_binding_update(&binding_root, &app_config, &config).unwrap();
    assert!(binding_root.join("config").join("binding.json").exists());
    assert!(!binding_root.join("config").join("source.json").exists());
    assert!(binding_root.join("chain.json").exists());
}

#[test]
fn fine_grained_update_save_logs_module_lifecycle() {
    let _guard = WRITER_TEST_LOCK.lock().unwrap();
    let temp = tempfile::tempdir().unwrap();
    let config = sample_rubo_config(1000);
    let logs = Arc::new(Mutex::new(Vec::new()));
    let subscriber = tracing_subscriber::fmt()
        .with_ansi(false)
        .with_writer(TestLogWriter::new(Arc::clone(&logs)))
        .finish();

    with_default(subscriber, || {
        save_source_update(temp.path(), &AppConfig::default(), &config).unwrap();
    });

    let logs = String::from_utf8(logs.lock().unwrap().clone()).unwrap();
    assert!(logs.contains("config.update.save_module{"));
    assert!(logs.contains("kind=\"source\""));
    assert!(logs.contains("config.update.save.source.start"));
    assert!(logs.contains("config.update.save.source.finish"));
}

#[test]
fn writer_generates_root_chain_json_from_bindings() {
    let _guard = WRITER_TEST_LOCK.lock().unwrap();
    let temp = tempfile::tempdir().unwrap();
    let mut config = RuboConfig::default();
    update_source(
        &mut config,
        SourceConfig::new("timer")
            .kind("interval")
            .set("interval_ms", 1000_u64),
    );
    update_device(
        &mut config,
        DeviceConfig::new("demo_camera", "virtual_camera").set("width", 640_u64),
    );
    update_func(
        &mut config,
        FuncConfig::new("inspect").set("score_scale", 0.8_f64),
    );
    update_sink(
        &mut config,
        SinkConfig::new("console")
            .kind("channel")
            .set("prefix", "[console]"),
    );
    update_sink(
        &mut config,
        SinkConfig::new("web").kind("web").set("route", "/events"),
    );
    update_binding(
        &mut config,
        BindingConfig::new("inspect_frame")
            .source("timer", "tick")
            .func("inspect")
            .device("demo_camera")
            .sink("console")
            .sink("web")
            .debug(true),
    );

    ConfigWriter::write_chain_json(temp.path(), &config).unwrap();

    let chain_json = std::fs::read_to_string(temp.path().join("chain.json")).unwrap();
    let chain: serde_json::Value = serde_json::from_str(&chain_json).unwrap();

    assert_eq!(chain.as_array().unwrap().len(), 1);
    assert_eq!(chain[0]["binding"], "inspect_frame");
    assert_eq!(chain[0]["source"]["id"], "timer");
    assert_eq!(chain[0]["source"]["event"], "tick");
    assert_eq!(chain[0]["source"]["kind"], "interval");
    assert_eq!(chain[0]["source"]["config"]["interval_ms"], 1000);
    assert_eq!(chain[0]["function"]["id"], "inspect");
    assert_eq!(chain[0]["function"]["config"]["score_scale"], 0.8);
    assert_eq!(chain[0]["devices"][0]["id"], "demo_camera");
    assert_eq!(chain[0]["devices"][0]["kind"], "virtual_camera");
    assert_eq!(chain[0]["devices"][0]["config"]["width"], 640);
    assert_eq!(chain[0]["sinks"][0]["id"], "console");
    assert_eq!(chain[0]["sinks"][0]["kind"], "channel");
    assert_eq!(chain[0]["sinks"][0]["config"]["prefix"], "[console]");
    assert_eq!(chain[0]["sinks"][1]["id"], "web");
    assert_eq!(chain[0]["sinks"][1]["kind"], "web");
    assert_eq!(chain[0]["sinks"][1]["config"]["route"], "/events");
    assert_eq!(chain[0]["debug"], true);
}

#[test]
fn writer_writes_module_config_files_that_loader_can_read() {
    let _guard = WRITER_TEST_LOCK.lock().unwrap();
    let temp = tempfile::tempdir().unwrap();
    let config_dir = temp.path().join("config");
    let mut config = RuboConfig::default();
    update_source(
        &mut config,
        SourceConfig::new("timer")
            .kind("interval")
            .set("interval_ms", 1000_u64),
    );
    update_device(
        &mut config,
        DeviceConfig::new("demo_camera", "virtual_camera").set("width", 640_u64),
    );
    update_func(
        &mut config,
        FuncConfig::new("inspect").set("score_scale", 0.8_f64),
    );
    update_sink(
        &mut config,
        SinkConfig::new("console")
            .kind("channel")
            .set("prefix", "[console]"),
    );
    update_binding(
        &mut config,
        BindingConfig::new("inspect_frame")
            .source("timer", "tick")
            .func("inspect")
            .device("demo_camera")
            .sink("console")
            .debug(true),
    );

    ConfigWriter::write_active_config(&config_dir, &AppConfig::default(), &config).unwrap();

    assert!(config_dir.join("source.json").exists());
    assert!(config_dir.join("device.json").exists());
    assert!(config_dir.join("function.json").exists());
    assert!(config_dir.join("sink.json").exists());
    assert!(config_dir.join("binding.json").exists());
    let source_json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(config_dir.join("source.json")).unwrap())
            .unwrap();
    assert!(source_json["timer"].get("id").is_none());
    assert_eq!(source_json["timer"]["kind"], "interval");

    let loaded = ConfigStore::load_active_config(&config_dir).unwrap();
    assert_eq!(
        loaded.sources()["timer"].get::<u64>("interval_ms").unwrap(),
        1000
    );
    assert_eq!(loaded.sources()["timer"].kind_ref(), "interval");
    assert_eq!(loaded.devices()["demo_camera"].kind(), "virtual_camera");
    assert_eq!(
        loaded.devices()["demo_camera"].get::<u64>("width").unwrap(),
        640
    );
    assert_eq!(
        loaded.funcs()["inspect"].get::<f64>("score_scale").unwrap(),
        0.8
    );
    assert_eq!(
        loaded.sinks()["console"].get::<String>("prefix").unwrap(),
        "[console]"
    );
    assert_eq!(loaded.sinks()["console"].kind_ref(), "channel");
    assert_eq!(
        loaded.bindings()["inspect_frame"].source_ref().id(),
        "timer"
    );
    assert_eq!(loaded.bindings()["inspect_frame"].func_ref(), "inspect");
}

#[test]
fn writer_uses_app_config_format_for_generated_config_files() {
    let _guard = WRITER_TEST_LOCK.lock().unwrap();
    let temp = tempfile::tempdir().unwrap();
    let toml_root = temp.path().join("toml_root");
    let yaml_root = temp.path().join("yaml_root");
    std::fs::create_dir_all(&toml_root).unwrap();
    std::fs::create_dir_all(&yaml_root).unwrap();
    std::fs::write(
        toml_root.join("application.toml"),
        r#"config_format = "toml""#,
    )
    .unwrap();
    std::fs::write(
        yaml_root.join("application.toml"),
        r#"config_format = "yaml""#,
    )
    .unwrap();
    let toml_app_config = ConfigStore::load_app_config(&toml_root).unwrap();
    let yaml_app_config = ConfigStore::load_app_config(&yaml_root).unwrap();
    let config = sample_rubo_config(1000);

    ConfigWriter::write_active_config(toml_root.join("config"), &toml_app_config, &config).unwrap();
    ConfigWriter::write_active_config(yaml_root.join("config"), &yaml_app_config, &config).unwrap();

    assert!(toml_root.join("config").join("source.toml").exists());
    assert!(!toml_root.join("config").join("source.json").exists());
    assert!(yaml_root.join("config").join("source.yaml").exists());
    assert!(!yaml_root.join("config").join("source.json").exists());
    assert_eq!(
        ConfigStore::load_active_config(toml_root.join("config"))
            .unwrap()
            .sources()["timer"]
            .get::<u64>("interval_ms")
            .unwrap(),
        1000
    );
    assert_eq!(
        ConfigStore::load_active_config(yaml_root.join("config"))
            .unwrap()
            .sources()["timer"]
            .get::<u64>("interval_ms")
            .unwrap(),
        1000
    );
}

#[test]
fn store_generates_active_config_when_config_files_are_missing() {
    let _guard = WRITER_TEST_LOCK.lock().unwrap();
    let temp = tempfile::tempdir().unwrap();
    let declared = sample_rubo_config(1000);

    let loaded =
        ConfigStore::load_or_init_config(temp.path(), &AppConfig::default(), &declared).unwrap();

    assert_eq!(loaded, declared);
    assert!(temp.path().join("config").join("source.json").exists());
    assert!(temp.path().join("chain.json").exists());
}

#[test]
fn store_accepts_matching_active_config() {
    let _guard = WRITER_TEST_LOCK.lock().unwrap();
    let temp = tempfile::tempdir().unwrap();
    let declared = sample_rubo_config(1000);
    ConfigWriter::write_active_config(temp.path().join("config"), &AppConfig::default(), &declared)
        .unwrap();

    let loaded =
        ConfigStore::load_or_init_config(temp.path(), &AppConfig::default(), &declared).unwrap();

    assert_eq!(loaded, declared);
}

#[test]
fn store_rejects_mismatched_active_config_and_writes_code_config_snapshot() {
    let _guard = WRITER_TEST_LOCK.lock().unwrap();
    let temp = tempfile::tempdir().unwrap();
    let active = sample_rubo_config(1000);
    let declared = sample_rubo_config(2000);
    ConfigWriter::write_active_config(temp.path().join("config"), &AppConfig::default(), &active)
        .unwrap();

    let error = ConfigStore::load_or_init_config(temp.path(), &AppConfig::default(), &declared)
        .unwrap_err();

    assert!(matches!(error, ConfigError::ConfigMismatch { .. }));
    assert!(
        error
            .to_string()
            .contains("active config does not match declared config")
    );
    assert!(error.to_string().contains("source.timer.interval_ms"));
    assert!(error.to_string().contains("active=1000"));
    assert!(error.to_string().contains("declared=2000"));
    assert!(
        temp.path()
            .join("config")
            .join("code_config")
            .join("source.json")
            .exists()
    );
    let snapshot =
        ConfigStore::load_active_config(temp.path().join("config").join("code_config")).unwrap();
    assert_eq!(snapshot, declared);
}

#[test]
fn store_continues_with_declared_config_when_active_config_cannot_load() {
    let _guard = WRITER_TEST_LOCK.lock().unwrap();
    let temp = tempfile::tempdir().unwrap();
    let config_dir = temp.path().join("config");
    std::fs::create_dir_all(&config_dir).unwrap();
    std::fs::write(config_dir.join("source.json"), "{invalid json").unwrap();
    let declared = sample_rubo_config(1000);
    let logs = Arc::new(Mutex::new(Vec::new()));
    let subscriber = tracing_subscriber::fmt()
        .with_ansi(false)
        .with_writer(TestLogWriter::new(Arc::clone(&logs)))
        .finish();

    let loaded = with_default(subscriber, || {
        ConfigStore::load_or_init_config(temp.path(), &AppConfig::default(), &declared).unwrap()
    });

    assert_eq!(loaded, declared);
    assert!(
        temp.path()
            .join("config")
            .join("code_config")
            .join("source.json")
            .exists()
    );
    let logs = String::from_utf8(logs.lock().unwrap().clone()).unwrap();
    assert!(logs.contains("config.load_or_init.active_load.error"));
    assert!(logs.contains("config.load_or_init.finish mode=declared_after_error"));
}

#[test]
fn writer_logs_chain_json_generation_lifecycle() {
    let _guard = WRITER_TEST_LOCK.lock().unwrap();
    let temp = tempfile::tempdir().unwrap();
    let logs = Arc::new(Mutex::new(Vec::new()));
    let subscriber = tracing_subscriber::fmt()
        .with_ansi(false)
        .with_writer(TestLogWriter::new(Arc::clone(&logs)))
        .finish();

    with_default(subscriber, || {
        ConfigWriter::write_chain_json(temp.path(), &RuboConfig::default()).unwrap();
    });

    let logs = String::from_utf8(logs.lock().unwrap().clone()).unwrap();
    assert!(logs.contains("chain_json.write{"));
    assert!(logs.contains("config.chain_json.write.start"));
    assert!(logs.contains("config.chain_json.write.empty"));
    assert!(logs.contains("config.chain_json.write.finish"));
}

#[derive(Clone)]
struct TestLogWriter {
    logs: Arc<Mutex<Vec<u8>>>,
}

impl TestLogWriter {
    fn new(logs: Arc<Mutex<Vec<u8>>>) -> Self {
        Self { logs }
    }
}

impl<'a> MakeWriter<'a> for TestLogWriter {
    type Writer = TestLogBuffer;

    fn make_writer(&'a self) -> Self::Writer {
        TestLogBuffer {
            logs: Arc::clone(&self.logs),
        }
    }
}

struct TestLogBuffer {
    logs: Arc<Mutex<Vec<u8>>>,
}

impl std::io::Write for TestLogBuffer {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.logs.lock().unwrap().extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn sample_rubo_config(interval_ms: u64) -> RuboConfig {
    let mut config = RuboConfig::default();
    update_source(
        &mut config,
        SourceConfig::new("timer")
            .kind("interval")
            .set("interval_ms", interval_ms),
    );
    update_device(
        &mut config,
        DeviceConfig::new("demo_camera", "virtual_camera").set("width", 640_u64),
    );
    update_func(
        &mut config,
        FuncConfig::new("inspect").set("score_scale", 0.8_f64),
    );
    update_sink(
        &mut config,
        SinkConfig::new("console")
            .kind("channel")
            .set("prefix", "[console]"),
    );
    update_binding(
        &mut config,
        BindingConfig::new("inspect_frame")
            .source("timer", "tick")
            .func("inspect")
            .device("demo_camera")
            .sink("console")
            .debug(true),
    );
    config
}
