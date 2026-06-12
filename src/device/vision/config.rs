use crate::config::{BlackRingDetectParams, ColorDetectParams, CrossDetectParams, QrDetectParams};

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TargetCorrection {
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Clone)]
pub struct BlackRingDetectConfig {
    pub path: String,
    pub debug_model: bool,
    pub loop_count: i32,
    pub target_correction: TargetCorrection,
    pub black_threshold: i32,
    pub min_radius: f64,
    pub max_radius: f64,
    pub min_circularity: f64,
    pub min_score: u8,
}

impl BlackRingDetectConfig {
    pub fn from_params(params: &BlackRingDetectParams, camera: &CameraDevice) -> Self {
        Self {
            path: camera.path.clone(),
            debug_model: params.debug_model,
            loop_count: params.loop_count,
            target_correction: TargetCorrection {
                x: params.target_correction.x,
                y: params.target_correction.y,
            },
            black_threshold: params.black_threshold,
            min_radius: params.min_radius,
            max_radius: params.max_radius,
            min_circularity: params.min_circularity,
            min_score: params.min_score,
        }
    }
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
    pub debug_model: bool,
    pub loop_count: i32,
    pub target_correction: TargetCorrection,
    pub black_threshold: i32,
    pub min_radius: f64,
    pub max_radius: f64,
    pub center_tolerance: f64,
    pub min_arc_points: usize,
    pub min_ring_score: u8,
    pub colors: Vec<CrossColor>,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct CrossColor {
    pub id: u8,
    pub name: String,
    pub hsv: [i32; 6],
    pub min_area: f64,
    pub min_circularity: f64,
}

impl CrossDetectConfig {
    pub fn from_params(params: &CrossDetectParams, camera: &CameraDevice) -> Self {
        Self {
            path: camera.path.clone(),
            debug_model: params.debug_model,
            loop_count: params.loop_count,
            target_correction: TargetCorrection {
                x: params.target_correction.x,
                y: params.target_correction.y,
            },
            black_threshold: params.black_threshold,
            min_radius: params.min_radius,
            max_radius: params.max_radius,
            center_tolerance: params.center_tolerance,
            min_arc_points: params.min_arc_points,
            min_ring_score: params.min_ring_score,
            colors: params
                .colors
                .iter()
                .map(|color| CrossColor {
                    id: color.id,
                    name: color.name.clone(),
                    hsv: color.hsv,
                    min_area: color.min_area,
                    min_circularity: color.min_circularity,
                })
                .collect(),
        }
    }
}
