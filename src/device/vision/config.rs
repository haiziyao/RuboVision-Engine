use std::collections::HashMap;

use anyhow::{Context, Result, anyhow};

#[derive(Debug, Clone)]
pub struct CameraDevice {
    pub path: String,
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

#[derive(Debug, Clone)]
pub struct CrossDetectConfig {
    pub path: String,
}

impl CameraDevice {
    pub fn from_args(args: &[String]) -> Result<Self> {
        let map = arg_map(args);
        Ok(Self {
            path: string_value(&map, "path")?,
        })
    }
}

impl ColorDetectConfig {
    pub fn from_args_with_camera(args: &[String], camera: &CameraDevice) -> Result<Self> {
        let map = arg_map(args);
        let color_ranges = color_ranges_from_args(args)?;

        let loop_count = i32_value(&map, "loop_count")?;
        let radius_ratio = f64_value(&map, "radius_ratio")?;
        let detect_area_access_rate = f64_value(&map, "detect_area_access_rate")?;

        Ok(Self {
            path: camera.path.clone(),
            debug_model: bool_value(&map, "debug_model")?,
            loop_count,
            radius_ratio,
            detect_area_access_rate,
            color_ranges,
        })
    }
}

impl QrDetectConfig {
    pub fn from_args_with_camera(args: &[String], camera: &CameraDevice) -> Result<Self> {
        let map = arg_map(args);
        Ok(Self {
            path: camera.path.clone(),
            debug_model: bool_value(&map, "debug_model")?,
        })
    }
}

impl CrossDetectConfig {
    pub fn from_args_with_camera(_args: &[String], camera: &CameraDevice) -> Result<Self> {
        Ok(Self {
            path: camera.path.clone(),
        })
    }
}

pub(super) fn arg_map(args: &[String]) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for arg in args {
        if let Some((key, value)) = arg.split_once('=') {
            map.insert(key.trim().to_string(), value.trim().to_string());
        }
    }
    map
}

pub(super) fn string_value(map: &HashMap<String, String>, key: &str) -> Result<String> {
    let value = map
        .get(key)
        .ok_or_else(|| anyhow!("missing required parameter `{key}`"))?;
    Ok(value.to_string())
}

fn bool_value(map: &HashMap<String, String>, key: &str) -> Result<bool> {
    string_value(map, key)?
        .parse()
        .with_context(|| format!("invalid bool parameter `{key}`"))
}

fn i32_value(map: &HashMap<String, String>, key: &str) -> Result<i32> {
    let value = string_value(map, key)?;
    value
        .parse()
        .with_context(|| format!("invalid i32 parameter `{key}`: got `{value}`"))
}

fn f64_value(map: &HashMap<String, String>, key: &str) -> Result<f64> {
    let value = string_value(map, key)?;
    value
        .parse()
        .with_context(|| format!("invalid f64 parameter `{key}`: got `{value}`"))
}

fn color_ranges_from_args(args: &[String]) -> Result<Vec<ColorRange>> {
    let mut ranges = Vec::new();

    for arg in args {
        let Some((key, value)) = arg.split_once('=') else {
            continue;
        };
        let key = key.trim();
        let Some(name) = key.strip_prefix("color.") else {
            continue;
        };

        let name = name.trim();
        let hsv = hsv_value_from_str(key, value)?;
        ranges.push(ColorRange {
            name: name.to_string(),
            hsv,
        });
    }

    if ranges.is_empty() {
        return Err(anyhow!("missing required parameter `color.<name>`"));
    }

    Ok(ranges)
}

fn hsv_value_from_str(key: &str, value: &str) -> Result<[i32; 6]> {
    let parts: Result<Vec<i32>> = value
        .trim_matches('[')
        .trim_matches(']')
        .split(',')
        .map(|item| {
            let item = item.trim();
            item.parse()
                .with_context(|| format!("invalid hsv parameter `{key}` item: got `{item}`"))
        })
        .collect();
    let parts = parts?;

    if parts.len() != 6 {
        return Err(anyhow!(
            "invalid hsv parameter `{key}`: expected 6 values, got {}",
            parts.len()
        ));
    }

    Ok([parts[0], parts[1], parts[2], parts[3], parts[4], parts[5]])
}
