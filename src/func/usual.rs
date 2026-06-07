use crate::config::ReturnTargets;
use crate::device::{
    ColorDetectConfig, CrossDetectConfig, Device, QrDetectConfig, ResponseDeviceConfig,
    TaskLightKind, begin_light_session, run_color_detect, run_cross_detect, run_qr_detect,
    send_response_line,
};
use crate::web::WebMessage;
use anyhow::Result;
use std::thread::sleep;
use std::time::Duration;
use tracing::{debug, warn};

pub fn fn_debug(args: &[String], _device: &Device, _returns: &ReturnTargets) -> WebMessage {
    debug!("debug Function executing");
    sleep(Duration::from_secs(5));
    let args = args.join(",");
    WebMessage::ok(format!("this is the debug function {args}"))
}

pub fn fn_color_detect(args: &[String], device: &Device, returns: &ReturnTargets) -> WebMessage {
    into_web_message("color_detect", color_detect_impl(args, device, returns))
}

pub fn fn_qr_detect(args: &[String], device: &Device, returns: &ReturnTargets) -> WebMessage {
    into_web_message("qr_detect", qr_detect_impl(args, device, returns))
}

pub fn fn_cross_detect(args: &[String], device: &Device, _returns: &ReturnTargets) -> WebMessage {
    into_web_message("cross_detect", cross_detect_impl(args, device))
}

fn color_detect_impl(
    args: &[String],
    device: &Device,
    returns: &ReturnTargets,
) -> Result<WebMessage> {
    let config = color_config(args, device)?;
    let response = response_config(args, device)?;
    let _light = if returns.gpio.is_some() {
        begin_light_session(&response, TaskLightKind::Color)?
    } else {
        None
    };
    let color_name = run_color_detect(&config)?;
    let serial_status = send_result_to_serial("color_detect", &response, &color_name);

    Ok(WebMessage::ok(format!(
        "color_detect finished: {color_name}; {serial_status}"
    )))
}

fn qr_detect_impl(args: &[String], device: &Device, returns: &ReturnTargets) -> Result<WebMessage> {
    let config = qr_config(args, device)?;
    let response = response_config(args, device)?;
    let _light = if returns.gpio.is_some() {
        begin_light_session(&response, TaskLightKind::Qr)?
    } else {
        None
    };
    let task_num = run_qr_detect(&config)?;
    let task_num_text = task_num.to_string();
    let serial_status = send_result_to_serial("qr_detect", &response, &task_num_text);

    Ok(WebMessage::ok(format!(
        "qr_detect finished: {task_num}; {serial_status}"
    )))
}

fn cross_detect_impl(args: &[String], device: &Device) -> Result<WebMessage> {
    let config = cross_config(args, device)?;
    let result = run_cross_detect(&config)?;
    Ok(WebMessage::ok(format!("cross_detect finished: {result}")))
}

fn color_config(args: &[String], device: &Device) -> Result<ColorDetectConfig> {
    match device {
        Device::Camera(camera) => ColorDetectConfig::from_args_with_camera(args, camera),
        Device::None => panic!("color_detect requires camera device"),
    }
}

fn qr_config(args: &[String], device: &Device) -> Result<QrDetectConfig> {
    match device {
        Device::Camera(camera) => QrDetectConfig::from_args_with_camera(args, camera),
        Device::None => panic!("qr_detect requires camera device"),
    }
}

fn cross_config(args: &[String], device: &Device) -> Result<CrossDetectConfig> {
    match device {
        Device::Camera(camera) => CrossDetectConfig::from_args_with_camera(args, camera),
        Device::None => panic!("cross_detect requires camera device"),
    }
}

fn response_config(args: &[String], device: &Device) -> Result<ResponseDeviceConfig> {
    match device {
        Device::Camera(camera) => {
            ResponseDeviceConfig::from_args_with_uart(args, camera.uart.clone())
        }
        Device::None => panic!("vision response requires camera device"),
    }
}

fn into_web_message(task_name: &str, result: Result<WebMessage>) -> WebMessage {
    match result {
        Ok(message) => message,
        Err(e) => WebMessage::error(format!("{task_name} failed: {e:#}")),
    }
}

fn send_result_to_serial(task_name: &str, response: &ResponseDeviceConfig, value: &str) -> String {
    match send_response_line(response, value) {
        Ok(()) => "serial=sent".to_string(),
        Err(e) => {
            warn!("{task_name} serial response failed: {e:#}");
            format!("serial=failed({e:#})")
        }
    }
}
