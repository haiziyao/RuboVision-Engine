use crate::config::{ColorDetectParams, CrossDetectParams, QrDetectParams};

#[derive(Debug, Clone)]
pub struct CameraDevice {
    pub path: String,
}

impl CameraDevice {
    pub fn new(path: impl Into<String>) -> Self {
        Self { path: path.into() }
    }
}

#[derive(Debug, Clone)]
pub struct ColorDetectConfig {
    pub path: String,
    pub debug_model: bool,
    pub loop_count: i32,
    pub radius_ratio: f64,
    pub detect_area_access_rate: f64,
    pub color_ranges: Vec<ColorRange>,
}

impl ColorDetectConfig {
    pub fn from_params(params: &ColorDetectParams, camera: &CameraDevice) -> Self {
        Self {
            path: camera.path.clone(),
            debug_model: params.debug_model,
            loop_count: params.loop_count,
            radius_ratio: params.radius_ratio,
            detect_area_access_rate: params.detect_area_access_rate,
            color_ranges: params
                .color_ranges
                .iter()
                .map(|range| ColorRange {
                    name: range.name.clone(),
                    hsv: range.hsv,
                })
                .collect(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ColorRange {
    pub name: String,
    pub hsv: [i32; 6],
}

#[derive(Debug, Clone)]
pub struct QrDetectConfig {
    pub path: String,
    pub debug_model: bool,
}

impl QrDetectConfig {
    pub fn from_params(params: &QrDetectParams, camera: &CameraDevice) -> Self {
        Self {
            path: camera.path.clone(),
            debug_model: params.debug_model,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CrossDetectConfig {
    pub path: String,
}

impl CrossDetectConfig {
    pub fn from_params(_params: &CrossDetectParams, camera: &CameraDevice) -> Self {
        Self {
            path: camera.path.clone(),
        }
    }
}
