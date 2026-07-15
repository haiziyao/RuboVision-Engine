use std::{
    collections::{BTreeSet, HashMap},
    fs,
    path::{Path, PathBuf},
};

use serde::Serialize;
use serde_json::{Map, Value};
use tracing::{error, info, info_span, warn};

use crate::ConfigError;
use crate::config::{
    AppConfig, BindingConfig, DeviceConfig, FuncConfig, RuboConfig, SinkConfig, SourceConfig,
    traits::ConfigAccess,
};
use crate::log::{error_text, hidden_text, hint_text, success_text, text, warn_text};

use super::writer::ConfigWriter;

pub struct ConfigStore {
    _root: PathBuf,
}

impl ConfigStore {
    pub fn load_or_init_config(
        root: impl AsRef<Path>,
        app_config: &AppConfig,
        declared_config: &RuboConfig,
    ) -> Result<RuboConfig, ConfigError> {
        let root = root.as_ref();
        let config_path = root.join(app_config.config_dir());
        let span = info_span!(
            target: "rubo_engine::config::store",
            "config.load_or_init",
            root = %root.display(),
            config_path = %config_path.display()
        );
        let _span_guard = span.enter();

        info!(
            "{}",
            success_text(format!(
                "config.load_or_init.start path={}",
                config_path.display()
            ))
        );
        if !has_active_config_files(&config_path) {
            info!(
                "{}",
                hidden_text(format!(
                    "config.load_or_init.generate path={}",
                    config_path.display()
                ))
            );
            ConfigWriter::write_active_config(&config_path, app_config, declared_config)?;
            ConfigWriter::write_chain_json(root, declared_config)?;
            info!(
                "{}",
                success_text(format!(
                    "config.load_or_init.finish mode=generated path={}",
                    config_path.display()
                ))
            );
            return Ok(declared_config.clone());
        }

        let active_config = match Self::load_active_config(&config_path) {
            Ok(config) => config,
            Err(error) => {
                let snapshot_path = config_path.join("code_config");
                ConfigWriter::write_active_config(&snapshot_path, app_config, declared_config)?;
                ConfigWriter::write_chain_json(root, declared_config)?;
                let message = format!(
                    "active config load failed; startup continues with declared config. error={error}; declared config snapshot: {}",
                    snapshot_path.display()
                );
                error!("{}", error_text(&message));
                warn!(
                    "{}",
                    warn_text(format!(
                        "config.load_or_init.active_load.error path={} error={error}",
                        config_path.display()
                    ))
                );
                info!(
                    "{}",
                    hint_text(format!(
                        "config.load_or_init.hint snapshot={}",
                        snapshot_path.display()
                    ))
                );
                info!(
                    "{}",
                    success_text(format!(
                        "config.load_or_init.finish mode=declared_after_error path={}",
                        config_path.display()
                    ))
                );
                return Ok(declared_config.clone());
            }
        };
        if active_config == *declared_config {
            ConfigWriter::write_chain_json(root, &active_config)?;
            info!(
                "{}",
                success_text(format!(
                    "config.load_or_init.finish mode=matched path={}",
                    config_path.display()
                ))
            );
            return Ok(active_config);
        }

        let snapshot_path = config_path.join("code_config");
        ConfigWriter::write_active_config(&snapshot_path, app_config, declared_config)?;
        let diff = config_diff(&active_config, declared_config);
        let message = format!(
            "active config does not match declared config; delete code config registration or delete active config files, then restart. declared config snapshot: {}; diff: {}",
            snapshot_path.display(),
            diff.join("; ")
        );
        error!("{}", error_text(&message));
        info!(
            "{}",
            hint_text(format!(
                "config.load_or_init.hint snapshot={}",
                snapshot_path.display()
            ))
        );
        Err(ConfigError::ConfigMismatch { message })
    }

    pub fn load_active_config(config_path: impl AsRef<Path>) -> Result<RuboConfig, ConfigError> {
        let config_path = config_path.as_ref();
        let span = info_span!(
            target: "rubo_engine::config::store",
            "active_config.load",
            path = %config_path.display()
        );
        let _span_guard = span.enter();

        info!(
            "{}",
            success_text(format!(
                "config.active.load.start path={}",
                config_path.display()
            ))
        );

        let mut config = RuboConfig::default();

        *config.sources_mut() = load_source_configs(config_path)?;
        *config.devices_mut() = load_device_configs(config_path)?;
        *config.funcs_mut() = load_config_map(config_path, "function", FuncConfig::new)?;
        *config.sinks_mut() = load_sink_configs(config_path)?;
        *config.bindings_mut() = load_binding_configs(config_path)?;

        info!(
            "{}",
            success_text(format!(
                "config.active.load.finish path={} sources={} devices={} funcs={} sinks={} bindings={}",
                config_path.display(),
                config.sources().len(),
                config.devices().len(),
                config.funcs().len(),
                config.sinks().len(),
                config.bindings().len()
            ))
        );

        Ok(config)
    }

    pub fn load_app_config(_root: impl AsRef<Path>) -> Result<AppConfig, ConfigError> {
        let root = _root.as_ref();
        let span = info_span!(
            target: "rubo_engine::config::store",
            "app_config.load",
            root = %root.display()
        );
        let _span_guard = span.enter();

        info!(
            "{}",
            success_text(format!("config.app.load.start root={}", root.display()))
        );

        let Some(path) = find_module_file(root, "application")? else {
            info!(
                "{}",
                text(format!("config.app.load.default root={}", root.display()))
            );
            return Ok(AppConfig::default());
        };

        let config: AppConfig = config::Config::builder()
            .add_source(config::File::from(path))
            .build()?
            .try_deserialize()
            .map_err(|error| ConfigError::ConfigFormat {
                message: error.to_string(),
            })?;

        info!(
            "{}",
            success_text(format!("config.app.load.finish root={}", root.display()))
        );

        Ok(config)
    }

    pub fn save_app_config(
        application_dir: impl AsRef<Path>,
        app_config: &AppConfig,
    ) -> Result<(), ConfigError> {
        let application_dir = application_dir.as_ref();
        fs::create_dir_all(application_dir)?;
        let path = find_module_file(application_dir, "application")?
            .unwrap_or_else(|| application_dir.join("application.yaml"));
        let content = match path.extension().and_then(|extension| extension.to_str()) {
            Some("json") => serde_json::to_string_pretty(app_config).map_err(format_error)?,
            Some("toml") => toml::to_string_pretty(app_config).map_err(format_error)?,
            Some("yaml" | "yml") => serde_yaml_bw::to_string(app_config).map_err(format_error)?,
            _ => {
                return Err(ConfigError::ConfigFormat {
                    message: format!("unsupported application config format: {}", path.display()),
                });
            }
        };
        fs::write(path, content)?;
        Ok(())
    }

    pub fn list_profiles(
        root: impl AsRef<Path>,
        app_config: &AppConfig,
    ) -> Result<Vec<String>, ConfigError> {
        let base = root.as_ref().join(app_config.config_path());
        let entries = match fs::read_dir(&base) {
            Ok(entries) => entries,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(error) => {
                return Err(ConfigError::ConfigLoad {
                    message: error.to_string(),
                });
            }
        };
        let mut profiles = Vec::new();
        for entry in entries {
            let entry = entry.map_err(|error| ConfigError::ConfigLoad {
                message: error.to_string(),
            })?;
            let path = entry.path();
            if path.is_dir() && has_active_config_files(&path) {
                profiles.push(entry.file_name().to_string_lossy().into_owned());
            }
        }
        profiles.sort();
        Ok(profiles)
    }
}

fn format_error(error: impl std::fmt::Display) -> ConfigError {
    ConfigError::ConfigFormat {
        message: error.to_string(),
    }
}

fn has_active_config_files(config_path: &Path) -> bool {
    ["source", "device", "function", "sink", "binding"]
        .into_iter()
        .any(|module| {
            ["toml", "yaml", "yml", "json"]
                .into_iter()
                .any(|extension| config_path.join(format!("{module}.{extension}")).exists())
        })
}

fn load_config_map<C, F>(
    config_path: &Path,
    module: &str,
    factory: F,
) -> Result<std::collections::HashMap<String, C>, ConfigError>
where
    C: ConfigAccess,
    F: Fn(String) -> C,
{
    let Some(value) = load_module_value(config_path, module)? else {
        return Ok(std::collections::HashMap::new());
    };
    let object = module_object(module, value)?;
    let mut result = std::collections::HashMap::new();

    for (id, value) in object {
        let values = object_value(module, &id, value)?;
        let mut item = factory(id.clone());
        item.values_mut().extend(values);
        result.insert(id, item);
    }

    Ok(result)
}

fn load_device_configs(
    config_path: &Path,
) -> Result<std::collections::HashMap<String, DeviceConfig>, ConfigError> {
    let Some(value) = load_module_value(config_path, "device")? else {
        return Ok(std::collections::HashMap::new());
    };
    let object = module_object("device", value)?;
    let mut result = std::collections::HashMap::new();

    for (id, value) in object {
        let mut values = object_value("device", &id, value)?;
        let kind = values
            .remove("kind")
            .ok_or_else(|| ConfigError::ConfigFormat {
                message: format!("device `{id}` missing kind"),
            })?
            .as_str()
            .ok_or_else(|| ConfigError::ConfigFormat {
                message: format!("device `{id}` kind must be string"),
            })?
            .to_string();
        let mut item = DeviceConfig::new(id.clone(), kind);
        item.values_mut().extend(values);
        result.insert(id, item);
    }

    Ok(result)
}

fn load_source_configs(
    config_path: &Path,
) -> Result<std::collections::HashMap<String, SourceConfig>, ConfigError> {
    let Some(value) = load_module_value(config_path, "source")? else {
        return Ok(std::collections::HashMap::new());
    };
    let object = module_object("source", value)?;
    let mut result = std::collections::HashMap::new();

    for (id, value) in object {
        let mut values = object_value("source", &id, value)?;
        let kind = optional_kind("source", &id, &mut values)?;
        let mut item = SourceConfig::new(id.clone()).kind(kind);
        item.values_mut().extend(values);
        result.insert(id, item);
    }

    Ok(result)
}

fn load_sink_configs(
    config_path: &Path,
) -> Result<std::collections::HashMap<String, SinkConfig>, ConfigError> {
    let Some(value) = load_module_value(config_path, "sink")? else {
        return Ok(std::collections::HashMap::new());
    };
    let object = module_object("sink", value)?;
    let mut result = std::collections::HashMap::new();

    for (id, value) in object {
        let mut values = object_value("sink", &id, value)?;
        let kind = optional_kind("sink", &id, &mut values)?;
        let mut item = SinkConfig::new(id.clone()).kind(kind);
        item.values_mut().extend(values);
        result.insert(id, item);
    }

    Ok(result)
}

fn load_binding_configs(
    config_path: &Path,
) -> Result<std::collections::HashMap<String, BindingConfig>, ConfigError> {
    let Some(value) = load_module_value(config_path, "binding")? else {
        return Ok(std::collections::HashMap::new());
    };
    let object = module_object("binding", value)?;
    let mut result = std::collections::HashMap::new();

    for (id, value) in object {
        let loaded: BindingConfig =
            serde_json::from_value(value).map_err(|error| ConfigError::ConfigFormat {
                message: error.to_string(),
            })?;
        let mut binding = BindingConfig::new(id.clone())
            .source(loaded.source_ref().id(), loaded.source_ref().event())
            .func(loaded.func_ref())
            .debug(loaded.debug_enabled());
        for device in loaded.devices() {
            binding = binding.device(device);
        }
        for sink in loaded.sinks() {
            binding = binding.sink(sink);
        }
        result.insert(id, binding);
    }

    Ok(result)
}

fn load_module_value(
    config_path: &Path,
    module: &str,
) -> Result<Option<config::Config>, ConfigError> {
    let Some(path) = find_module_file(config_path, module)? else {
        return Ok(None);
    };
    config::Config::builder()
        .add_source(config::File::from(path))
        .build()
        .map(Some)
        .map_err(ConfigError::from)
}

fn find_module_file(config_path: &Path, module: &str) -> Result<Option<PathBuf>, ConfigError> {
    let candidates = [
        config_path.join(format!("{module}.toml")),
        config_path.join(format!("{module}.yaml")),
        config_path.join(format!("{module}.yml")),
        config_path.join(format!("{module}.json")),
    ];
    let found: Vec<PathBuf> = candidates
        .into_iter()
        .filter(|path| path.exists())
        .collect();

    match found.len() {
        0 => Ok(None),
        1 => Ok(found.into_iter().next()),
        _ => {
            let message = format!("multiple {module} config files found");
            error!("{}", error_text(&message));
            Err(ConfigError::ConfigFormat { message })
        }
    }
}

fn module_object(module: &str, value: config::Config) -> Result<Map<String, Value>, ConfigError> {
    value
        .try_deserialize::<Value>()
        .map_err(|error| ConfigError::ConfigFormat {
            message: error.to_string(),
        })?
        .as_object()
        .cloned()
        .ok_or_else(|| ConfigError::ConfigFormat {
            message: format!("{module} config must be object"),
        })
}

fn object_value(module: &str, id: &str, value: Value) -> Result<Map<String, Value>, ConfigError> {
    let object = value.as_object().ok_or_else(|| ConfigError::ConfigFormat {
        message: format!("{module} `{id}` must be object"),
    })?;
    Ok(object.clone())
}

fn optional_kind(
    module: &str,
    id: &str,
    values: &mut Map<String, Value>,
) -> Result<String, ConfigError> {
    match values.remove("kind") {
        Some(Value::String(kind)) => Ok(kind),
        Some(_) => Err(ConfigError::ConfigFormat {
            message: format!("{module} `{id}` kind must be string"),
        }),
        None => Ok(String::new()),
    }
}

fn config_diff(active: &RuboConfig, declared: &RuboConfig) -> Vec<String> {
    let mut diffs = Vec::new();
    diff_source_module(active.sources(), declared.sources(), &mut diffs);
    diff_device_module(active.devices(), declared.devices(), &mut diffs);
    diff_access_module("function", active.funcs(), declared.funcs(), &mut diffs);
    diff_sink_module(active.sinks(), declared.sinks(), &mut diffs);
    diff_serialized_module(
        "binding",
        active.bindings(),
        declared.bindings(),
        &mut diffs,
    );
    if diffs.is_empty() {
        diffs.push("config differs but no field-level diff was produced".to_string());
    }
    diffs
}

fn diff_access_module<C>(
    module: &str,
    active: &HashMap<String, C>,
    declared: &HashMap<String, C>,
    diffs: &mut Vec<String>,
) where
    C: ConfigAccess,
{
    diff_module(module, active, declared, diffs, |item| {
        Value::Object(item.values().clone())
    });
}

fn diff_device_module(
    active: &HashMap<String, DeviceConfig>,
    declared: &HashMap<String, DeviceConfig>,
    diffs: &mut Vec<String>,
) {
    diff_module("device", active, declared, diffs, |device| {
        let mut value = device.values().clone();
        value.insert("kind".to_string(), Value::String(device.kind().to_string()));
        Value::Object(value)
    });
}

fn diff_source_module(
    active: &HashMap<String, SourceConfig>,
    declared: &HashMap<String, SourceConfig>,
    diffs: &mut Vec<String>,
) {
    diff_module("source", active, declared, diffs, |source| {
        let mut value = source.values().clone();
        value.insert(
            "kind".to_string(),
            Value::String(source.kind_ref().to_string()),
        );
        Value::Object(value)
    });
}

fn diff_sink_module(
    active: &HashMap<String, SinkConfig>,
    declared: &HashMap<String, SinkConfig>,
    diffs: &mut Vec<String>,
) {
    diff_module("sink", active, declared, diffs, |sink| {
        let mut value = sink.values().clone();
        value.insert(
            "kind".to_string(),
            Value::String(sink.kind_ref().to_string()),
        );
        Value::Object(value)
    });
}

fn diff_serialized_module<C>(
    module: &str,
    active: &HashMap<String, C>,
    declared: &HashMap<String, C>,
    diffs: &mut Vec<String>,
) where
    C: Serialize,
{
    diff_module(module, active, declared, diffs, |item| {
        let mut value = serde_json::to_value(item).unwrap_or(Value::Null);
        if let Some(object) = value.as_object_mut() {
            object.remove("id");
        }
        value
    });
}

fn diff_module<C, F>(
    module: &str,
    active: &HashMap<String, C>,
    declared: &HashMap<String, C>,
    diffs: &mut Vec<String>,
    to_value: F,
) where
    F: Fn(&C) -> Value,
{
    let ids: BTreeSet<_> = active.keys().chain(declared.keys()).collect();
    for id in ids {
        match (active.get(id), declared.get(id)) {
            (Some(active_item), Some(declared_item)) => {
                diff_value(
                    &format!("{module}.{id}"),
                    Some(&to_value(active_item)),
                    Some(&to_value(declared_item)),
                    diffs,
                );
            }
            (Some(_), None) => diffs.push(format!("{module}.{id} active=present declared=missing")),
            (None, Some(_)) => diffs.push(format!("{module}.{id} active=missing declared=present")),
            (None, None) => {}
        }
    }
}

fn diff_value(
    path: &str,
    active: Option<&Value>,
    declared: Option<&Value>,
    diffs: &mut Vec<String>,
) {
    match (active, declared) {
        (Some(Value::Object(active_object)), Some(Value::Object(declared_object))) => {
            let keys: BTreeSet<_> = active_object.keys().chain(declared_object.keys()).collect();
            for key in keys {
                diff_value(
                    &format!("{path}.{key}"),
                    active_object.get(key),
                    declared_object.get(key),
                    diffs,
                );
            }
        }
        (Some(active_value), Some(declared_value)) if active_value != declared_value => {
            diffs.push(format!(
                "{path} active={} declared={}",
                format_diff_value(active_value),
                format_diff_value(declared_value)
            ));
        }
        (Some(active_value), None) => {
            diffs.push(format!(
                "{path} active={} declared=missing",
                format_diff_value(active_value)
            ));
        }
        (None, Some(declared_value)) => {
            diffs.push(format!(
                "{path} active=missing declared={}",
                format_diff_value(declared_value)
            ));
        }
        _ => {}
    }
}

fn format_diff_value(value: &Value) -> String {
    match value {
        Value::String(value) => value.clone(),
        other => serde_json::to_string(other).unwrap_or_else(|_| "<unprintable>".to_string()),
    }
}
