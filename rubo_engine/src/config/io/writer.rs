use std::{fs, path::Path};

use serde::Serialize;
use serde_json::{Map, Value};
use tracing::{error, info, info_span, warn};

use crate::ConfigError;
use crate::config::{
    AppConfig, BindingConfig, ConfigFileFormat, DeviceConfig, FuncConfig, RuboConfig, SinkConfig,
    SourceConfig, traits::ConfigAccess,
};
use crate::log::{error_text, success_text, warn_text};

pub struct ConfigWriter;

impl ConfigWriter {
    pub fn write_active_config(
        config_path: impl AsRef<Path>,
        app_config: &AppConfig,
        config: &RuboConfig,
    ) -> Result<(), ConfigError> {
        let config_path = config_path.as_ref();
        let format = app_config.config_format();
        let span = info_span!(
            target: "rubo_engine::config::writer",
            "active_config.write",
            path = %config_path.display(),
            format = ?format
        );
        let _span_guard = span.enter();

        info!(
            "{}",
            success_text(format!(
                "config.active.write.start path={}",
                config_path.display()
            ))
        );
        fs::create_dir_all(config_path).map_err(|error| {
            log_io_error("config.active.write.create_dir.error", config_path, &error);
            io_error(error)
        })?;

        Self::write_source_config(config_path, app_config, config)?;
        Self::write_device_config(config_path, app_config, config)?;
        Self::write_func_config(config_path, app_config, config)?;
        Self::write_sink_config(config_path, app_config, config)?;
        Self::write_binding_config(config_path, app_config, config)?;

        info!(
            "{}",
            success_text(format!(
                "config.active.write.finish path={} sources={} devices={} funcs={} sinks={} bindings={}",
                config_path.display(),
                config.sources().len(),
                config.devices().len(),
                config.funcs().len(),
                config.sinks().len(),
                config.bindings().len()
            ))
        );
        Ok(())
    }

    pub fn write_source_config(
        config_path: impl AsRef<Path>,
        app_config: &AppConfig,
        config: &RuboConfig,
    ) -> Result<(), ConfigError> {
        write_named_module_file(
            config_path.as_ref(),
            app_config.config_format(),
            "source",
            source_module(config.sources()),
        )
    }

    pub fn write_device_config(
        config_path: impl AsRef<Path>,
        app_config: &AppConfig,
        config: &RuboConfig,
    ) -> Result<(), ConfigError> {
        write_named_module_file(
            config_path.as_ref(),
            app_config.config_format(),
            "device",
            device_module(config.devices()),
        )
    }

    pub fn write_func_config(
        config_path: impl AsRef<Path>,
        app_config: &AppConfig,
        config: &RuboConfig,
    ) -> Result<(), ConfigError> {
        write_named_module_file(
            config_path.as_ref(),
            app_config.config_format(),
            "function",
            func_module(config.funcs()),
        )
    }

    pub fn write_sink_config(
        config_path: impl AsRef<Path>,
        app_config: &AppConfig,
        config: &RuboConfig,
    ) -> Result<(), ConfigError> {
        write_named_module_file(
            config_path.as_ref(),
            app_config.config_format(),
            "sink",
            sink_module(config.sinks()),
        )
    }

    pub fn write_binding_config(
        config_path: impl AsRef<Path>,
        app_config: &AppConfig,
        config: &RuboConfig,
    ) -> Result<(), ConfigError> {
        write_named_module_file(
            config_path.as_ref(),
            app_config.config_format(),
            "binding",
            binding_module(config.bindings()),
        )
    }

    pub fn write_chain_json(
        root: impl AsRef<Path>,
        config: &RuboConfig,
    ) -> Result<(), ConfigError> {
        let root = root.as_ref();
        let path = root.join("chain.json");
        let span = info_span!(
            target: "rubo_engine::config::chain_json",
            "chain_json.write",
            path = %path.display()
        );
        let _span_guard = span.enter();

        info!(
            "{}",
            success_text(format!(
                "config.chain_json.write.start path={}",
                path.display()
            ))
        );

        fs::create_dir_all(root).map_err(|error| {
            log_io_error("config.chain_json.write.create_dir.error", root, &error);
            io_error(error)
        })?;

        let mut bindings: Vec<_> = config.bindings().values().collect();
        bindings.sort_by(|left, right| left.id().cmp(right.id()));

        if bindings.is_empty() {
            warn!(
                "{}",
                warn_text(format!(
                    "config.chain_json.write.empty path={}",
                    path.display()
                ))
            );
        }

        let chains: Vec<_> = bindings
            .into_iter()
            .map(|binding| ChainJsonItem {
                binding: binding.id(),
                source: ChainJsonSource {
                    id: binding.source_ref().id(),
                    event: binding.source_ref().event(),
                    kind: config
                        .sources()
                        .get(binding.source_ref().id())
                        .map(|source| source.kind_ref()),
                    config: config
                        .sources()
                        .get(binding.source_ref().id())
                        .map(|source| source.values().clone())
                        .unwrap_or_default(),
                },
                function: ChainJsonFunction {
                    id: binding.func_ref(),
                    config: config
                        .funcs()
                        .get(binding.func_ref())
                        .map(|func| func.values().clone())
                        .unwrap_or_default(),
                },
                devices: binding
                    .devices()
                    .iter()
                    .map(|id| {
                        let device = config.devices().get(id);
                        ChainJsonDevice {
                            id,
                            kind: device.map(|device| device.kind()),
                            config: device
                                .map(|device| device.values().clone())
                                .unwrap_or_default(),
                        }
                    })
                    .collect(),
                sinks: binding
                    .sinks()
                    .iter()
                    .map(|id| ChainJsonEndpoint {
                        id,
                        kind: config.sinks().get(id).map(|sink| sink.kind_ref()),
                        config: config
                            .sinks()
                            .get(id)
                            .map(|sink| sink.values().clone())
                            .unwrap_or_default(),
                    })
                    .collect(),
                debug: binding.debug_enabled(),
            })
            .collect();

        let content = serde_json::to_string_pretty(&chains).map_err(|error| {
            let message = format!("config.chain_json.write.serialize.error error={error}");
            error!("{}", error_text(&message));
            ConfigError::ConfigFormat {
                message: error.to_string(),
            }
        })?;
        fs::write(&path, content).map_err(|error| {
            log_io_error("config.chain_json.write.file.error", &path, &error);
            io_error(error)
        })?;

        info!(
            "{}",
            success_text(format!(
                "config.chain_json.write.finish path={} chains={}",
                path.display(),
                config.bindings().len()
            ))
        );

        Ok(())
    }
}

fn source_module(sources: &std::collections::HashMap<String, SourceConfig>) -> Map<String, Value> {
    let mut ids: Vec<_> = sources.keys().collect();
    ids.sort();
    let mut module = Map::new();
    for id in ids {
        let source = &sources[id];
        let mut values = source.values().clone();
        insert_kind(&mut values, source.kind_ref());
        module.insert(id.clone(), Value::Object(values));
    }
    module
}

fn func_module(funcs: &std::collections::HashMap<String, FuncConfig>) -> Map<String, Value> {
    access_module(funcs)
}

fn sink_module(sinks: &std::collections::HashMap<String, SinkConfig>) -> Map<String, Value> {
    let mut ids: Vec<_> = sinks.keys().collect();
    ids.sort();
    let mut module = Map::new();
    for id in ids {
        let sink = &sinks[id];
        let mut values = sink.values().clone();
        insert_kind(&mut values, sink.kind_ref());
        module.insert(id.clone(), Value::Object(values));
    }
    module
}

fn access_module<C>(items: &std::collections::HashMap<String, C>) -> Map<String, Value>
where
    C: ConfigAccess,
{
    let mut ids: Vec<_> = items.keys().collect();
    ids.sort();
    let mut module = Map::new();
    for id in ids {
        module.insert(id.clone(), Value::Object(items[id].values().clone()));
    }
    module
}

fn device_module(devices: &std::collections::HashMap<String, DeviceConfig>) -> Map<String, Value> {
    let mut ids: Vec<_> = devices.keys().collect();
    ids.sort();
    let mut module = Map::new();
    for id in ids {
        let device = &devices[id];
        let mut values = device.values().clone();
        values.insert("kind".to_string(), Value::String(device.kind().to_string()));
        module.insert(id.clone(), Value::Object(values));
    }
    module
}

fn binding_module(
    bindings: &std::collections::HashMap<String, BindingConfig>,
) -> Map<String, Value> {
    let mut ids: Vec<_> = bindings.keys().collect();
    ids.sort();
    let mut module = Map::new();
    for id in ids {
        let binding = &bindings[id];
        let source = BindingSourceJson {
            id: binding.source_ref().id(),
            event: binding.source_ref().event(),
        };
        let value = BindingModuleJson {
            source,
            function: binding.func_ref(),
            devices: binding.devices(),
            sinks: binding.sinks(),
            debug: binding.debug_enabled(),
        };
        let json = serde_json::to_value(value)
            .unwrap_or_else(|error| Value::String(format!("binding serialize failed: {error}")));
        module.insert(id.clone(), json);
    }
    module
}

#[derive(Serialize)]
struct BindingModuleJson<'a> {
    source: BindingSourceJson<'a>,
    #[serde(rename = "function")]
    function: &'a str,
    devices: &'a [String],
    sinks: &'a [String],
    debug: bool,
}

#[derive(Serialize)]
struct BindingSourceJson<'a> {
    id: &'a str,
    event: &'a str,
}

fn write_module_file(
    path: &Path,
    format: ConfigFileFormat,
    value: Map<String, Value>,
) -> Result<(), ConfigError> {
    let value = Value::Object(value);
    let content = match format {
        ConfigFileFormat::Json => {
            serde_json::to_string_pretty(&value).map_err(|error| ConfigError::ConfigFormat {
                message: error.to_string(),
            })?
        }
        ConfigFileFormat::Toml => {
            toml::to_string_pretty(&value).map_err(|error| ConfigError::ConfigFormat {
                message: error.to_string(),
            })?
        }
        ConfigFileFormat::Yaml => {
            serde_yaml_bw::to_string(&value).map_err(|error| ConfigError::ConfigFormat {
                message: error.to_string(),
            })?
        }
    };
    fs::write(path, content).map_err(|error| {
        log_io_error("config.active.write.file.error", path, &error);
        io_error(error)
    })
}

fn write_named_module_file(
    config_path: &Path,
    format: ConfigFileFormat,
    module: &str,
    value: Map<String, Value>,
) -> Result<(), ConfigError> {
    fs::create_dir_all(config_path).map_err(|error| {
        log_io_error("config.module.write.create_dir.error", config_path, &error);
        io_error(error)
    })?;
    write_module_file(
        &config_path.join(format!("{module}.{}", format.extension())),
        format,
        value,
    )
}

#[derive(Serialize)]
struct ChainJsonItem<'a> {
    binding: &'a str,
    source: ChainJsonSource<'a>,
    #[serde(rename = "function")]
    function: ChainJsonFunction<'a>,
    devices: Vec<ChainJsonDevice<'a>>,
    sinks: Vec<ChainJsonEndpoint<'a>>,
    debug: bool,
}

#[derive(Serialize)]
struct ChainJsonSource<'a> {
    id: &'a str,
    event: &'a str,
    kind: Option<&'a str>,
    config: Map<String, Value>,
}

#[derive(Serialize)]
struct ChainJsonFunction<'a> {
    id: &'a str,
    config: Map<String, Value>,
}

#[derive(Serialize)]
struct ChainJsonDevice<'a> {
    id: &'a str,
    kind: Option<&'a str>,
    config: Map<String, Value>,
}

#[derive(Serialize)]
struct ChainJsonEndpoint<'a> {
    id: &'a str,
    kind: Option<&'a str>,
    config: Map<String, Value>,
}

fn insert_kind(values: &mut Map<String, Value>, kind: &str) {
    if !kind.is_empty() {
        values.insert("kind".to_string(), Value::String(kind.to_string()));
    }
}

fn io_error(error: std::io::Error) -> ConfigError {
    ConfigError::ConfigWrite {
        message: error.to_string(),
    }
}

fn log_io_error(event: &str, path: &Path, error: &std::io::Error) {
    error!(
        "{}",
        error_text(format!("{event} path={} error={error}", path.display()))
    );
}
