use std::path::Path;

use crate::ConfigError;
use crate::config::{
    AppConfig, BindingConfig, DeviceConfig, FuncConfig, RuboConfig, SinkConfig, SourceConfig,
};
use crate::log::{success_text, text};
use tracing::{info, info_span};

use super::writer::ConfigWriter;

pub fn save_update(
    root: impl AsRef<Path>,
    app_config: &AppConfig,
    config: &RuboConfig,
) -> Result<(), ConfigError> {
    let root = root.as_ref();
    let config_path = root.join(app_config.config_path());
    let span = info_span!(
        target: "rubo_engine::config::update",
        "config.update.save",
        root = %root.display(),
        config_path = %config_path.display()
    );
    let _span_guard = span.enter();

    info!(
        "{}",
        success_text(format!(
            "config.update.save.start path={}",
            config_path.display()
        ))
    );
    ConfigWriter::write_active_config(&config_path, app_config, config)?;
    ConfigWriter::write_chain_json(root, config)?;
    info!(
        "{}",
        success_text(format!(
            "config.update.save.finish path={}",
            config_path.display()
        ))
    );
    Ok(())
}

pub fn save_source_update(
    root: impl AsRef<Path>,
    app_config: &AppConfig,
    config: &RuboConfig,
) -> Result<(), ConfigError> {
    save_module_update(
        root,
        app_config,
        config,
        "source",
        |config_path, app_config, config| {
            ConfigWriter::write_source_config(config_path, app_config, config)
        },
    )
}

pub fn save_device_update(
    root: impl AsRef<Path>,
    app_config: &AppConfig,
    config: &RuboConfig,
) -> Result<(), ConfigError> {
    save_module_update(
        root,
        app_config,
        config,
        "device",
        |config_path, app_config, config| {
            ConfigWriter::write_device_config(config_path, app_config, config)
        },
    )
}

pub fn save_func_update(
    root: impl AsRef<Path>,
    app_config: &AppConfig,
    config: &RuboConfig,
) -> Result<(), ConfigError> {
    save_module_update(
        root,
        app_config,
        config,
        "func",
        |config_path, app_config, config| {
            ConfigWriter::write_func_config(config_path, app_config, config)
        },
    )
}

pub fn save_sink_update(
    root: impl AsRef<Path>,
    app_config: &AppConfig,
    config: &RuboConfig,
) -> Result<(), ConfigError> {
    save_module_update(
        root,
        app_config,
        config,
        "sink",
        |config_path, app_config, config| {
            ConfigWriter::write_sink_config(config_path, app_config, config)
        },
    )
}

pub fn save_binding_update(
    root: impl AsRef<Path>,
    app_config: &AppConfig,
    config: &RuboConfig,
) -> Result<(), ConfigError> {
    save_module_update(
        root,
        app_config,
        config,
        "binding",
        |config_path, app_config, config| {
            ConfigWriter::write_binding_config(config_path, app_config, config)
        },
    )
}

fn save_module_update<F>(
    root: impl AsRef<Path>,
    app_config: &AppConfig,
    config: &RuboConfig,
    kind: &str,
    write_module: F,
) -> Result<(), ConfigError>
where
    F: FnOnce(&Path, &AppConfig, &RuboConfig) -> Result<(), ConfigError>,
{
    let root = root.as_ref();
    let config_path = root.join(app_config.config_path());
    let span = info_span!(
        target: "rubo_engine::config::update",
        "config.update.save_module",
        kind,
        root = %root.display(),
        config_path = %config_path.display()
    );
    let _span_guard = span.enter();

    info!(
        "{}",
        success_text(format!(
            "config.update.save.{kind}.start path={}",
            config_path.display()
        ))
    );
    write_module(&config_path, app_config, config)?;
    ConfigWriter::write_chain_json(root, config).map(|_| {
        info!(
            "{}",
            success_text(format!(
                "config.update.save.{kind}.finish path={}",
                config_path.display()
            ))
        );
    })
}

pub fn update_source(config: &mut RuboConfig, source: SourceConfig) {
    let id = source.id().to_string();
    let action = if config.sources().contains_key(&id) {
        "replace"
    } else {
        "insert"
    };
    let span = info_span!(
        target: "rubo_engine::config::update",
        "config.update",
        kind = "source",
        id = %id,
        action
    );
    let _span_guard = span.enter();
    config.sources_mut().insert(id.clone(), source);
    info!("{}", text(format!("config.update.source.{action} id={id}")));
}

pub fn update_device(config: &mut RuboConfig, device: DeviceConfig) {
    let id = device.id().to_string();
    let action = if config.devices().contains_key(&id) {
        "replace"
    } else {
        "insert"
    };
    let span = info_span!(
        target: "rubo_engine::config::update",
        "config.update",
        kind = "device",
        id = %id,
        action
    );
    let _span_guard = span.enter();
    config.devices_mut().insert(id.clone(), device);
    info!("{}", text(format!("config.update.device.{action} id={id}")));
}

pub fn update_func(config: &mut RuboConfig, func: FuncConfig) {
    let id = func.id().to_string();
    let action = if config.funcs().contains_key(&id) {
        "replace"
    } else {
        "insert"
    };
    let span = info_span!(
        target: "rubo_engine::config::update",
        "config.update",
        kind = "func",
        id = %id,
        action
    );
    let _span_guard = span.enter();
    config.funcs_mut().insert(id.clone(), func);
    info!("{}", text(format!("config.update.func.{action} id={id}")));
}

pub fn update_sink(config: &mut RuboConfig, sink: SinkConfig) {
    let id = sink.id().to_string();
    let action = if config.sinks().contains_key(&id) {
        "replace"
    } else {
        "insert"
    };
    let span = info_span!(
        target: "rubo_engine::config::update",
        "config.update",
        kind = "sink",
        id = %id,
        action
    );
    let _span_guard = span.enter();
    config.sinks_mut().insert(id.clone(), sink);
    info!("{}", text(format!("config.update.sink.{action} id={id}")));
}

pub fn update_binding(config: &mut RuboConfig, binding: BindingConfig) {
    let id = binding.id().to_string();
    let action = if config.bindings().contains_key(&id) {
        "replace"
    } else {
        "insert"
    };
    let span = info_span!(
        target: "rubo_engine::config::update",
        "config.update",
        kind = "binding",
        id = %id,
        action
    );
    let _span_guard = span.enter();
    config.bindings_mut().insert(id.clone(), binding);
    info!(
        "{}",
        text(format!("config.update.binding.{action} id={id}"))
    );
}
