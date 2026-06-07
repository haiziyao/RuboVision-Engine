use crate::config::ReturnTargets;
use crate::device::{
    ColorDetectConfig, CrossDetectConfig, Device, QrDetectConfig, run_color_detect,
    run_cross_detect, run_qr_detect,
};
use crate::message::TaskOutput;
use anyhow::{Result, anyhow};
use std::thread::sleep;
use std::time::Duration;
use tracing::debug;

pub fn fn_debug(args: &[String], _device: &Device, _returns: &ReturnTargets) -> TaskOutput {
    debug!("debug Function executing");
    sleep(Duration::from_secs(5));
    TaskOutput::ok(format!("this is the debug function {}", args.join(",")))
}

pub fn fn_color_detect(args: &[String], device: &Device, _returns: &ReturnTargets) -> TaskOutput {
    into_task_output("color_detect", color_detect_impl(args, device))
}

pub fn fn_qr_detect(args: &[String], device: &Device, _returns: &ReturnTargets) -> TaskOutput {
    into_task_output("qr_detect", qr_detect_impl(args, device))
}

pub fn fn_cross_detect(args: &[String], device: &Device, _returns: &ReturnTargets) -> TaskOutput {
    into_task_output("cross_detect", cross_detect_impl(args, device))
}

fn color_detect_impl(args: &[String], device: &Device) -> Result<TaskOutput> {
    let config = color_config(args, device)?;
    let color_name = run_color_detect(&config)?;
    Ok(TaskOutput::value(
        format!("color_detect finished: {color_name}"),
        color_name,
    ))
}

fn qr_detect_impl(args: &[String], device: &Device) -> Result<TaskOutput> {
    let config = qr_config(args, device)?;
    let task_num = run_qr_detect(&config)?;
    Ok(TaskOutput::value(
        format!("qr_detect finished: {task_num}"),
        task_num.to_string(),
    ))
}

fn cross_detect_impl(args: &[String], device: &Device) -> Result<TaskOutput> {
    let config = cross_config(args, device)?;
    let result = run_cross_detect(&config)?;
    Ok(TaskOutput::value(
        format!("cross_detect finished: {result}"),
        result,
    ))
}

fn color_config(args: &[String], device: &Device) -> Result<ColorDetectConfig> {
    match device {
        Device::Camera(camera) => ColorDetectConfig::from_args_with_camera(args, camera),
        Device::None => Err(anyhow!("color_detect requires camera device")),
    }
}

fn qr_config(args: &[String], device: &Device) -> Result<QrDetectConfig> {
    match device {
        Device::Camera(camera) => QrDetectConfig::from_args_with_camera(args, camera),
        Device::None => Err(anyhow!("qr_detect requires camera device")),
    }
}

fn cross_config(args: &[String], device: &Device) -> Result<CrossDetectConfig> {
    match device {
        Device::Camera(camera) => CrossDetectConfig::from_args_with_camera(args, camera),
        Device::None => Err(anyhow!("cross_detect requires camera device")),
    }
}

fn into_task_output(task_name: &str, result: Result<TaskOutput>) -> TaskOutput {
    match result {
        Ok(output) => output,
        Err(error) => TaskOutput::error(format!("{task_name} failed: {error:#}")),
    }
}
