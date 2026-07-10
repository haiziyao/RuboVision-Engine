use std::time::{SystemTime, UNIX_EPOCH};

use crate::{
    DevicePool, DispatchOutput, FunctionCall, FunctionDevices, FunctionRegister, Output,
    OutputError, OutputErrorKind, OutputRoute, OutputTiming, TaskRequest,
    config::RuboConfig,
    log::{error_text, success_text, text},
};
use tracing::{Instrument, info, info_span};

pub async fn execute(
    dispatch_output: DispatchOutput,
    config: &RuboConfig,
    functions: &FunctionRegister,
    devices: &DevicePool,
) -> Output {
    async move {
        match dispatch_output {
            DispatchOutput::Task(task) => execute_task(task, config, functions, devices).await,
            DispatchOutput::Error(error) => {
                info!(
                    "{}",
                    error_text(format!(
                        "executor.dispatch.error source={} key={} kind={:?}",
                        error.source_id(),
                        error.key(),
                        error.kind()
                    ))
                );
                let started_at_ms = error.message().started_at_ms_ref().unwrap_or_else(now_ms);
                let finished_at_ms = now_ms();
                Output::error(
                    OutputRoute::new(
                        None::<String>,
                        error.source_id(),
                        error.key(),
                        None::<String>,
                        Vec::new(),
                    ),
                    OutputTiming::new(started_at_ms, finished_at_ms),
                    OutputError::new(OutputErrorKind::Dispatch, format!("{:?}", error.kind())),
                )
            }
        }
    }
    .instrument(info_span!("executor.execute"))
    .await
}

async fn execute_task(
    task: TaskRequest,
    config: &RuboConfig,
    functions: &FunctionRegister,
    devices: &DevicePool,
) -> Output {
    let span_binding_id = task.binding_id().to_string();
    let span_func_id = task.func_id().to_string();
    let span = info_span!(
        "executor.task",
        binding_id = %span_binding_id,
        func_id = %span_func_id
    );
    async move {
        info!(
            "{}",
            text(format!(
                "executor.task.start binding={} func={}",
                task.binding_id(),
                task.func_id()
            ))
        );
        let started_at_ms = task.message().started_at_ms_ref().unwrap_or_else(now_ms);
        let route = OutputRoute::new(
            Some(task.binding_id()),
            task.source_id(),
            task.key(),
            Some(task.func_id()),
            task.sink_ids().to_vec(),
        );

        let function = match functions.get(task.func_id()) {
            Some(function) => function,
            None => {
                return runtime_error(
                    route,
                    started_at_ms,
                    format!("function `{}` missing", task.func_id()),
                );
            }
        };

        let function_config = match config.funcs().get(task.func_id()) {
            Some(function_config) => function_config,
            None => {
                return runtime_error(
                    route,
                    started_at_ms,
                    format!("function config `{}` missing", task.func_id()),
                );
            }
        };

        let mut function_devices = FunctionDevices::new();
        for id in task.device_ids() {
            match devices.get(id) {
                Some(device) => function_devices.insert(id, device),
                None => {
                    return runtime_error(route, started_at_ms, format!("device `{id}` missing"));
                }
            }
        }

        let function_call = FunctionCall::new(function_config, task.message(), function_devices);
        match function.call(function_call).await {
            Ok(result) => {
                info!(
                    "{}",
                    success_text(format!(
                        "executor.task.finish binding={} func={}",
                        span_binding_id, span_func_id
                    ))
                );
                Output::success(route, OutputTiming::new(started_at_ms, now_ms()), result)
            }
            Err(error) => Output::error(
                route,
                OutputTiming::new(started_at_ms, now_ms()),
                OutputError::new(OutputErrorKind::Function, error.to_string()),
            ),
        }
    }
    .instrument(span)
    .await
}

fn runtime_error(route: OutputRoute, started_at_ms: u64, message: String) -> Output {
    Output::error(
        route,
        OutputTiming::new(started_at_ms, now_ms()),
        OutputError::new(OutputErrorKind::Runtime, message),
    )
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}
