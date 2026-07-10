use futures::{StreamExt, stream::FuturesUnordered};
use tokio::sync::mpsc;
use tracing::{Instrument, info, info_span};

use crate::{
    DevicePool, DeviceRegister, FunctionRegister, Message, RuntimeError, SinkRegister, Source,
    SourceError, SourceHandler, SourceRegister, build_device_pool,
    config::{RuboConfig, SourceConfig},
    log::{error_text, success_text, text, warn_text},
    runtime::{RuntimeOutput, RuntimeResources, handle_message},
};

pub async fn run_config(
    config: &RuboConfig,
    source_register: &SourceRegister,
    device_register: &DeviceRegister,
    functions: &FunctionRegister,
    sinks: &SinkRegister,
    channel_size: usize,
) -> Result<Vec<SourceRunOutput>, RuntimeError> {
    async move {
        info!(
            "{}",
            text(format!(
                "runtime.config.start sources={} devices={} funcs={} sinks={} bindings={}",
                config.sources().len(),
                config.devices().len(),
                config.funcs().len(),
                config.sinks().len(),
                config.bindings().len()
            ))
        );
        validate_runtime_config(config, source_register, device_register)?;
        let devices = build_device_pool(config, device_register).await?;
        let outputs = run_config_sources(
            source_register,
            channel_size,
            config,
            functions,
            &devices,
            sinks,
        )
        .await;
        info!(
            "{}",
            success_text(format!("runtime.config.finish sources={}", outputs.len()))
        );
        Ok(outputs)
    }
    .instrument(info_span!("runtime.run_config"))
    .await
}

pub async fn run_config_with_resources(
    config: &RuboConfig,
    source_register: &SourceRegister,
    resources: &mut RuntimeResources,
    device_register: &DeviceRegister,
    functions: &FunctionRegister,
    sinks: &SinkRegister,
    channel_size: usize,
) -> Result<Vec<SourceRunOutput>, RuntimeError> {
    async move {
        info!(
            "{}",
            text(format!(
                "runtime.config.resources.start sources={} devices={} funcs={} sinks={} bindings={}",
                config.sources().len(),
                config.devices().len(),
                config.funcs().len(),
                config.sinks().len(),
                config.bindings().len()
            ))
        );
        validate_runtime_config(config, source_register, device_register)?;
        let devices = build_device_pool(config, device_register).await?;
        let outputs = run_config_sources_with_resources(
            source_register,
            resources,
            channel_size,
            config,
            functions,
            &devices,
            sinks,
        )
        .await;
        info!(
            "{}",
            success_text(format!(
                "runtime.config.resources.finish sources={}",
                outputs.len()
            ))
        );
        Ok(outputs)
    }
    .instrument(info_span!("runtime.run_config_with_resources"))
    .await
}

fn validate_runtime_config(
    config: &RuboConfig,
    source_register: &SourceRegister,
    device_register: &DeviceRegister,
) -> Result<(), RuntimeError> {
    let _span = info_span!("runtime.validate_config").entered();
    for source in config.sources().values() {
        if source.kind_ref().is_empty() {
            info!(
                "{}",
                error_text(format!(
                    "runtime.config.invalid source={} reason=kind_missing",
                    source.id()
                ))
            );
            return Err(RuntimeError::ConfigInvalid {
                message: format!("source `{}` kind is missing", source.id()),
            });
        }
        if !source_register.contains_kind(source.kind_ref()) {
            info!(
                "{}",
                error_text(format!(
                    "runtime.config.invalid source={} kind={} reason=source_kind_not_registered",
                    source.id(),
                    source.kind_ref()
                ))
            );
            return Err(RuntimeError::ConfigInvalid {
                message: format!(
                    "source `{}` source kind `{}` is not registered",
                    source.id(),
                    source.kind_ref()
                ),
            });
        }
    }
    for binding in config.bindings().values() {
        if !config.sources().contains_key(binding.source_ref().id()) {
            return Err(RuntimeError::ConfigInvalid {
                message: format!(
                    "binding `{}` references missing source `{}`",
                    binding.id(),
                    binding.source_ref().id()
                ),
            });
        }
    }
    for sink in config.sinks().values() {
        if sink.kind_ref().is_empty() {
            info!(
                "{}",
                error_text(format!(
                    "runtime.config.invalid sink={} reason=kind_missing",
                    sink.id()
                ))
            );
            return Err(RuntimeError::ConfigInvalid {
                message: format!("sink `{}` kind is missing", sink.id()),
            });
        }
    }
    for device in config.devices().values() {
        if device.kind().is_empty() {
            info!(
                "{}",
                error_text(format!(
                    "runtime.config.invalid device={} reason=kind_missing",
                    device.id()
                ))
            );
            return Err(RuntimeError::ConfigInvalid {
                message: format!("device `{}` kind is missing", device.id()),
            });
        }
        if !device_register.contains_kind(device.kind()) {
            info!(
                "{}",
                error_text(format!(
                    "runtime.config.invalid device={} kind={} reason=device_kind_not_registered",
                    device.id(),
                    device.kind()
                ))
            );
            return Err(RuntimeError::Device {
                error: crate::DeviceError::KindNotRegistered {
                    kind: device.kind().to_string(),
                },
            });
        }
    }
    info!("{}", success_text("runtime.config.valid"));
    Ok(())
}

pub async fn run_config_sources(
    source_register: &SourceRegister,
    channel_size: usize,
    config: &RuboConfig,
    functions: &FunctionRegister,
    devices: &DevicePool,
    sinks: &SinkRegister,
) -> Vec<SourceRunOutput> {
    async move {
        let mut build_errors = Vec::new();
        let mut source_futures = Vec::new();
        for source_config in config.sources().values() {
            match source_register.build(source_config) {
                Ok(handler) => source_futures.push(run_source(
                    source_config.id(),
                    handler,
                    source_config,
                    channel_size,
                    config,
                    functions,
                    devices,
                    sinks,
                )),
                Err(error) => {
                    info!(
                        "{}",
                        warn_text(format!(
                            "runtime.source.build.error source={} error={}",
                            source_config.id(),
                            error
                        ))
                    );
                    build_errors.push(SourceRunOutput::new(
                        source_config.id().to_string(),
                        Err(error),
                        Vec::new(),
                    ));
                }
            }
        }
        let mut results = futures::future::join_all(source_futures).await;
        build_errors.append(&mut results);
        build_errors
    }
    .instrument(info_span!("runtime.run_config_sources"))
    .await
}

pub async fn run_config_sources_with_resources(
    source_register: &SourceRegister,
    resources: &mut RuntimeResources,
    channel_size: usize,
    config: &RuboConfig,
    functions: &FunctionRegister,
    devices: &DevicePool,
    sinks: &SinkRegister,
) -> Vec<SourceRunOutput> {
    async move {
        let mut build_errors = Vec::new();
        let mut source_futures = Vec::new();
        for source_config in config.sources().values() {
            match source_register.build_with_resources(source_config, resources) {
                Ok(handler) => source_futures.push(run_source(
                    source_config.id(),
                    handler,
                    source_config,
                    channel_size,
                    config,
                    functions,
                    devices,
                    sinks,
                )),
                Err(error) => {
                    info!(
                        "{}",
                        warn_text(format!(
                            "runtime.source.build.resources.error source={} error={}",
                            source_config.id(),
                            error
                        ))
                    );
                    build_errors.push(SourceRunOutput::new(
                        source_config.id().to_string(),
                        Err(error),
                        Vec::new(),
                    ));
                }
            }
        }
        let mut results = futures::future::join_all(source_futures).await;
        build_errors.append(&mut results);
        build_errors
    }
    .instrument(info_span!("runtime.run_config_sources_with_resources"))
    .await
}

pub async fn run_source(
    source_id: impl Into<String>,
    handler: impl SourceHandler,
    source_config: &SourceConfig,
    channel_size: usize,
    config: &RuboConfig,
    functions: &FunctionRegister,
    devices: &DevicePool,
    sinks: &SinkRegister,
) -> SourceRunOutput {
    let source_id = source_id.into();
    let source_span_id = source_id.clone();
    async move {
        info!(
            "{}",
            text(format!("runtime.source.start source={source_id}"))
        );
        let (sender, receiver) = mpsc::channel(channel_size);
        let mut source = Source::new(&source_id, sender, handler);
        let source_config = source_config.clone();
        let source_future = async move {
            let result = source.start(&source_config).await;
            drop(source);
            result
        };
        let messages_future =
            run_source_messages(&source_id, receiver, config, functions, devices, sinks);
        let (source_result, runtime_outputs) = tokio::join!(source_future, messages_future);
        info!(
            "{}",
            success_text(format!(
                "runtime.source.finish source={} outputs={}",
                source_id,
                runtime_outputs.len()
            ))
        );
        SourceRunOutput::new(source_id, source_result, runtime_outputs)
    }
    .instrument(info_span!("runtime.source", source_id = %source_span_id))
    .await
}

pub async fn run_source_messages(
    source_id: impl Into<String>,
    mut receiver: mpsc::Receiver<Message>,
    config: &RuboConfig,
    functions: &FunctionRegister,
    devices: &DevicePool,
    sinks: &SinkRegister,
) -> Vec<RuntimeOutput> {
    let source_id = source_id.into();
    let source_span_id = source_id.clone();
    async move {
        let max_concurrent = receiver.max_capacity().max(1);
        let source_id_ref = source_id.as_str();
        let mut active = FuturesUnordered::new();
        let mut results = Vec::new();
        let mut next_sequence = 0;
        let mut receiver_closed = false;
        loop {
            if receiver_closed && active.is_empty() {
                break;
            }
            tokio::select! {
                result = active.next(), if !active.is_empty() => {
                    if let Some(result) = result {
                        results.push(result);
                    }
                }
                message = receiver.recv(), if !receiver_closed && active.len() < max_concurrent => {
                    match message {
                        Some(message) => {
                            let key = message.key().to_string();
                            info!(
                                "{}",
                                text(format!(
                                    "runtime.message.receive source={source_id_ref} key={key}"
                                ))
                            );
                            let sequence = next_sequence;
                            next_sequence += 1;
                            active.push(async move {
                                let output = handle_message(
                                    source_id_ref,
                                    message,
                                    config,
                                    functions,
                                    devices,
                                    sinks,
                                )
                                .await;
                                (sequence, output)
                            });
                        }
                        None => receiver_closed = true,
                    }
                }
            }
        }
        results.sort_by_key(|(sequence, _)| *sequence);
        results.into_iter().map(|(_, output)| output).collect()
    }
    .instrument(info_span!(
        "runtime.source_messages",
        source_id = %source_span_id
    ))
    .await
}

pub struct SourceRunOutput {
    source_id: String,
    source_result: Result<(), SourceError>,
    runtime_outputs: Vec<RuntimeOutput>,
}

impl SourceRunOutput {
    pub fn new(
        source_id: impl Into<String>,
        source_result: Result<(), SourceError>,
        runtime_outputs: Vec<RuntimeOutput>,
    ) -> Self {
        Self {
            source_id: source_id.into(),
            source_result,
            runtime_outputs,
        }
    }

    pub fn source_id(&self) -> &str {
        &self.source_id
    }

    pub fn source_result(&self) -> &Result<(), SourceError> {
        &self.source_result
    }

    pub fn runtime_outputs(&self) -> &[RuntimeOutput] {
        &self.runtime_outputs
    }
}
