//! 配置模块
//!
//! 负责读取并解析 `config/param.toml`

use anyhow::{Context, Result};
use serde::Deserialize;
use std::fs;

#[derive(Debug, Deserialize, Clone)]
pub struct DeviceConfig {
    pub title: String,

    pub color_camera_config: ColorCameraConfig,
    pub qr_camera_config: QrCameraConfig,
    pub cross_camera_config: CrossCameraConfig,
    pub gpio_config: GpioConfig,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ColorCameraConfig {
    pub color_camera: String,
    pub colors: Vec<String>,

    pub hsv_red: [i32; 3],
    pub hsv_blue: [i32; 3],
    pub hsv_green: [i32; 3],
    pub hsv_white: [i32; 3],
    pub hsv_black: [i32; 3],
}

#[derive(Debug, Deserialize, Clone)]
pub struct QrCameraConfig {
    pub qr_camera: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct CrossCameraConfig {
    pub cross_camera: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct GpioConfig {
    pub pin: Vec<String>,
}

impl DeviceConfig {
    pub fn from_file(path: &str) -> Result<Self> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("读取配置文件失败：{path}"))?;

        let cfg: DeviceConfig = toml::from_str(&content)
            .with_context(|| format!("解析 TOML 失败：{path}"))?;

        Ok(cfg)
    }
}
