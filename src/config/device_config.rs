//! 配置模块
//!
//! 负责读取并解析 `config/param.toml`
#![allow(dead_code)]

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
    pub light_config:LightConfig,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ColorCameraConfig {
    pub color_camera: String,
    pub debug_model:bool,
    pub loop_count:i32,
    pub radius_ratio:f64,
    pub detect_area_access_rate:f64,
    pub colors: Vec<String>,
    pub hsv_red: [i32; 6],
    pub hsv_blue: [i32; 6],
    pub hsv_green: [i32; 6],
    pub hsv_black: [i32; 6],
    pub hsv_white: [i32; 6],
}

#[derive(Debug, Deserialize, Clone)]
pub struct QrCameraConfig {
    pub qr_camera: String,
    pub debug_model:bool,
}

 
#[derive(Debug, Deserialize, Clone)]
pub struct CrossCameraConfig {
    pub cross_camera: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct GpioConfig {
    pub serial: String,
 
    pub baud: u32,
    pub data_bit:u8,
    pub stop_bit:u8,
    pub parity_bit:bool,
 
}
#[derive(Debug, Deserialize, Clone)]
pub struct LightConfig{
    pub color_light_pin:u8,
    pub qr_light_pin:u8,
    pub gpio_light_pin:u8,
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
