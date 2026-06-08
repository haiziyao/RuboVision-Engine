use std::env;

use anyhow::{Context, Result, anyhow};
use opencv::{
    core::{Mat, Scalar},
    imgproc,
    prelude::*,
    videoio,
};

use crate::config::{
    BlackRingDetectParams, ColorDetectParams, CrossDetectParams, QrDetectParams, RuntimeConfig,
    WebConfig, load_config,
};

use super::super::{
    BlackRingDetectConfig, CameraDevice, ColorDetectConfig, CrossDetectConfig, QrDetectConfig,
};

pub(super) fn color_detect_config_from_config() -> Result<ColorDetectConfig> {
    let cfg = load_config().context("failed to load runtime config")?;
    let mut camera = camera_from_config(&cfg, "color_camera")?;
    if let Ok(override_path) = env::var("RUBO_TEST_COLOR_CAMERA") {
        camera.path = override_path;
    }
    if let Ok(override_path) = env::var("RUBO_TEST_CAMERA") {
        camera.path = override_path;
    }

    let params: ColorDetectParams = function_params(&cfg, "color_detect")?;
    Ok(ColorDetectConfig::from_params(&params, &camera))
}

pub(super) fn qr_detect_config_from_config() -> Result<QrDetectConfig> {
    let cfg = load_config().context("failed to load runtime config")?;
    let mut camera = camera_from_config(&cfg, "qr_camera")?;
    if let Ok(override_path) = env::var("RUBO_TEST_QR_CAMERA") {
        camera.path = override_path;
    }
    if let Ok(override_path) = env::var("RUBO_TEST_CAMERA") {
        camera.path = override_path;
    }

    let params: QrDetectParams = function_params(&cfg, "qr_detect")?;
    Ok(QrDetectConfig::from_params(&params, &camera))
}

pub(super) fn black_ring_detect_config_from_config() -> Result<BlackRingDetectConfig> {
    let cfg = load_config().context("failed to load runtime config")?;
    let mut camera = camera_from_config(&cfg, "color_camera")?;
    if let Ok(override_path) = env::var("RUBO_TEST_BLACK_RING_CAMERA") {
        camera.path = override_path;
    }
    if let Ok(override_path) = env::var("RUBO_TEST_CAMERA") {
        camera.path = override_path;
    }

    let params: BlackRingDetectParams = function_params(&cfg, "black_ring_detect")?;
    Ok(BlackRingDetectConfig::from_params(&params, &camera))
}

pub(super) fn cross_detect_config_from_config() -> Result<CrossDetectConfig> {
    let cfg = load_config().context("failed to load runtime config")?;
    let mut camera = camera_from_config(&cfg, "cross_camera")?;
    if let Ok(override_path) = env::var("RUBO_TEST_CROSS_CAMERA") {
        camera.path = override_path;
    }
    if let Ok(override_path) = env::var("RUBO_TEST_CAMERA") {
        camera.path = override_path;
    }

    let params: CrossDetectParams = function_params(&cfg, "cross_detect")?;
    Ok(CrossDetectConfig::from_params(&params, &camera))
}

pub(super) fn configured_device_path(device_id: &str) -> Result<String> {
    let cfg = load_config().context("failed to load runtime config")?;
    Ok(camera_from_config(&cfg, device_id)?.path)
}

pub(super) fn configured_color_names() -> Result<Vec<String>> {
    let cfg = load_config().context("failed to load runtime config")?;
    let params: ColorDetectParams = function_params(&cfg, "color_detect")?;
    Ok(params
        .color_ranges
        .iter()
        .map(|range| range.name.clone())
        .collect())
}

pub(super) fn web_config_from_config() -> Result<WebConfig> {
    let cfg = load_config().context("failed to load runtime config")?;
    Ok(cfg.message.web)
}

pub(super) fn open_camera(path: &str) -> Result<videoio::VideoCapture> {
    let cam = videoio::VideoCapture::from_file(path, videoio::CAP_V4L2)
        .with_context(|| format!("failed to open camera {path}"))?;
    if !videoio::VideoCapture::is_opened(&cam)? {
        return Err(anyhow!("camera is not opened: {path}"));
    }
    Ok(cam)
}

pub(super) fn read_non_empty_frame(cam: &mut videoio::VideoCapture) -> Result<Mat> {
    for _ in 0..30 {
        let mut frame = Mat::default();
        cam.read(&mut frame)?;
        if !frame.empty() {
            return Ok(frame);
        }
    }

    Err(anyhow!("camera returned empty frames"))
}

pub(super) fn draw_label(frame: &mut Mat, text: &str, x: i32, y: i32) -> Result<()> {
    imgproc::put_text(
        frame,
        text,
        opencv::core::Point::new(x, y),
        imgproc::FONT_HERSHEY_SIMPLEX,
        0.8,
        Scalar::new(255.0, 255.0, 255.0, 0.0),
        2,
        imgproc::LINE_AA,
        false,
    )?;
    Ok(())
}

fn camera_from_config(cfg: &RuntimeConfig, device_id: &str) -> Result<CameraDevice> {
    let device = cfg
        .devices
        .list
        .iter()
        .find(|device| device.device_id == device_id)
        .ok_or_else(|| anyhow!("config does not contain device_id={device_id}"))?;
    Ok(CameraDevice::new(&device.path))
}

fn function_params<T>(cfg: &RuntimeConfig, function_id: &str) -> Result<T>
where
    T: serde::de::DeserializeOwned,
{
    let func = cfg
        .functions
        .entries
        .iter()
        .find(|function| function.function_id == function_id)
        .ok_or_else(|| anyhow!("config does not contain function_id={function_id}"))?;

    func.params
        .clone()
        .try_into()
        .with_context(|| format!("invalid {function_id} params"))
}
