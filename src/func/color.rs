use async_trait::async_trait;
use rubo_engine::{
    FuncResult, Function, FunctionCall, FunctionError,
    config::{ConfigAccess, FuncConfig},
};
use serde::Deserialize;

use crate::device::CameraDevice;

#[derive(Debug, Clone, Deserialize)]
pub(super) struct ColorDefinition {
    pub(super) name: String,
    #[serde(default)]
    output: Option<String>,
    hsv_ranges: Vec<[i32; 6]>,
}

impl ColorDefinition {
    #[cfg(feature = "opencv")]
    pub(super) fn output(&self) -> &str {
        self.output.as_deref().unwrap_or(&self.name)
    }

    pub(super) fn validate(&self, function_id: &str) -> Result<(), FunctionError> {
        if self.name.trim().is_empty() {
            return Err(FunctionError::Config {
                message: format!("{function_id} color name cannot be empty"),
            });
        }
        if self
            .output
            .as_ref()
            .is_some_and(|output| output.trim().is_empty())
        {
            return Err(FunctionError::Config {
                message: format!("{function_id} color `{}` output cannot be empty", self.name),
            });
        }
        if self.hsv_ranges.is_empty() {
            return Err(FunctionError::Config {
                message: format!(
                    "{function_id} color `{}` must contain at least one HSV range",
                    self.name
                ),
            });
        }
        for range in &self.hsv_ranges {
            let valid_h = (0..=179).contains(&range[0])
                && (0..=179).contains(&range[1])
                && range[0] <= range[1];
            let valid_s = (0..=255).contains(&range[2])
                && (0..=255).contains(&range[3])
                && range[2] <= range[3];
            let valid_v = (0..=255).contains(&range[4])
                && (0..=255).contains(&range[5])
                && range[4] <= range[5];
            if !valid_h || !valid_s || !valid_v {
                return Err(FunctionError::Config {
                    message: format!(
                        "{function_id} color `{}` contains an invalid HSV range",
                        self.name
                    ),
                });
            }
        }
        Ok(())
    }
}

#[cfg(feature = "opencv")]
pub(super) fn color_mask(
    hsv: &opencv::core::Mat,
    color: &ColorDefinition,
) -> opencv::Result<opencv::core::Mat> {
    use opencv::core;

    let mut mask = crate::vision::util::hsv_mask(hsv, color.hsv_ranges[0])?;
    for range in color.hsv_ranges.iter().skip(1) {
        let range_mask = crate::vision::util::hsv_mask(hsv, *range)?;
        let mut merged = core::Mat::default();
        core::bitwise_or(&mask, &range_mask, &mut merged, &core::no_array())?;
        mask = merged;
    }
    Ok(mask)
}

#[cfg(feature = "opencv")]
#[derive(Debug, Clone, PartialEq, Eq)]
struct ColorDetection {
    name: String,
    output: String,
}

#[derive(Debug, Clone)]
struct ColorParameters {
    device_id: String,
    max_frames: usize,
    confirm_frames: usize,
    radius_ratio: f64,
    min_area_ratio: f64,
    colors: Vec<ColorDefinition>,
}

impl ColorParameters {
    fn from_config(config: &FuncConfig) -> Result<Self, FunctionError> {
        let parameters = Self {
            device_id: config.get("device_id")?,
            max_frames: config.get("max_frames")?,
            confirm_frames: config.get("confirm_frames")?,
            radius_ratio: config.get("radius_ratio")?,
            min_area_ratio: config.get("min_area_ratio")?,
            colors: config.get("colors")?,
        };
        parameters.validate()?;
        Ok(parameters)
    }

    fn validate(&self) -> Result<(), FunctionError> {
        if self.device_id.trim().is_empty() {
            return Err(FunctionError::Config {
                message: "color_detect.device_id cannot be empty".to_string(),
            });
        }
        if self.max_frames == 0 {
            return Err(FunctionError::Config {
                message: "color_detect.max_frames must be greater than 0".to_string(),
            });
        }
        if self.confirm_frames == 0 || self.confirm_frames > self.max_frames {
            return Err(FunctionError::Config {
                message: "color_detect.confirm_frames must be between 1 and max_frames".to_string(),
            });
        }
        if !self.radius_ratio.is_finite() || self.radius_ratio <= 0.0 {
            return Err(FunctionError::Config {
                message: "color_detect.radius_ratio must be a finite number greater than 0"
                    .to_string(),
            });
        }
        if !self.min_area_ratio.is_finite()
            || self.min_area_ratio <= 0.0
            || self.min_area_ratio > 1.0
        {
            return Err(FunctionError::Config {
                message: "color_detect.min_area_ratio must be greater than 0 and at most 1"
                    .to_string(),
            });
        }
        if self.colors.is_empty() {
            return Err(FunctionError::Config {
                message: "color_detect.colors cannot be empty".to_string(),
            });
        }
        for color in &self.colors {
            color.validate("color_detect")?;
        }
        Ok(())
    }
}

#[cfg(feature = "opencv")]
struct ColorFrameResult {
    color: Option<ColorDetection>,
    ratio: f64,
    roi: opencv::core::Mat,
    masks: Vec<(String, opencv::core::Mat)>,
    selected_mask: opencv::core::Mat,
    frame: opencv::core::Mat,
}

#[cfg(feature = "opencv")]
struct ColorResult {
    name: String,
    value: String,
    ratio: f64,
    frame: opencv::core::Mat,
}

#[cfg(feature = "opencv")]
fn analyze_color_frame(
    frame: &opencv::core::Mat,
    parameters: &ColorParameters,
) -> opencv::Result<ColorFrameResult> {
    use opencv::{core, imgproc, prelude::*};

    let (roi, roi_mask) = crate::vision::util::circle_roi(frame, parameters.radius_ratio)?;
    let hsv = crate::vision::util::bgr_to_hsv(frame)?;
    let mut masks = Vec::with_capacity(parameters.colors.len());
    let mut best_index = 0;
    let mut best_ratio = -1.0_f64;

    for (index, color) in parameters.colors.iter().enumerate() {
        let merged_mask = color_mask(&hsv, color)?;
        let mask = crate::vision::util::mask_in_roi(&merged_mask, &roi_mask)?;
        let ratio = crate::vision::util::mask_ratio(&mask, &roi_mask)?;
        if ratio > best_ratio {
            best_index = index;
            best_ratio = ratio;
        }
        masks.push((color.name.clone(), mask));
    }

    let color = (best_ratio >= parameters.min_area_ratio).then(|| ColorDetection {
        name: parameters.colors[best_index].name.clone(),
        output: parameters.colors[best_index].output().to_string(),
    });
    let selected_mask = masks[best_index].1.try_clone()?;
    let mut annotated = frame.try_clone()?;
    let size = annotated.size()?;
    let center = core::Point::new(size.width / 2, size.height / 2);
    let radius = (size.width.min(size.height) as f64 * parameters.radius_ratio) as i32;
    let label = color
        .as_ref()
        .map(|detection| detection.name.as_str())
        .unwrap_or("unknown");
    imgproc::circle(
        &mut annotated,
        center,
        radius,
        core::Scalar::new(0.0, 255.0, 0.0, 0.0),
        2,
        imgproc::LINE_8,
        0,
    )?;
    imgproc::put_text(
        &mut annotated,
        &format!("color: {label} ratio: {best_ratio:.2}"),
        core::Point::new(10, 30),
        imgproc::FONT_HERSHEY_SIMPLEX,
        0.8,
        core::Scalar::new(255.0, 255.0, 255.0, 0.0),
        2,
        imgproc::LINE_AA,
        false,
    )?;

    Ok(ColorFrameResult {
        color,
        ratio: best_ratio,
        roi,
        masks,
        selected_mask,
        frame: annotated,
    })
}

#[cfg(feature = "opencv")]
async fn detect_color(
    camera: std::sync::Arc<CameraDevice>,
    parameters: &ColorParameters,
) -> Result<ColorResult, FunctionError> {
    let mut candidate = None;
    let mut consecutive_frames = 0;

    for _ in 0..parameters.max_frames {
        let frame = camera.frame().await?;
        let frame_parameters = parameters.clone();
        let frame_result =
            tokio::task::spawn_blocking(move || analyze_color_frame(&frame, &frame_parameters))
                .await
                .map_err(|error| FunctionError::Call {
                    message: format!("color_detect task failed: {error}"),
                })?
                .map_err(|error| FunctionError::Call {
                    message: error.to_string(),
                })?;

        let Some(color) = frame_result.color.as_ref() else {
            candidate = None;
            consecutive_frames = 0;
            continue;
        };
        if candidate.as_deref() == Some(color.name.as_str()) {
            consecutive_frames += 1;
        } else {
            candidate = Some(color.name.clone());
            consecutive_frames = 1;
        }
        if consecutive_frames >= parameters.confirm_frames {
            return Ok(ColorResult {
                name: color.name.clone(),
                value: color.output.clone(),
                ratio: frame_result.ratio,
                frame: frame_result.frame,
            });
        }
    }

    Err(FunctionError::Call {
        message: format!(
            "color was not confirmed within {} frames",
            parameters.max_frames
        ),
    })
}

#[rubo_engine::function(id = "color_detect")]
#[derive(Default)]
pub struct ColorDetect;

#[async_trait]
impl Function for ColorDetect {
    async fn call(&self, call: FunctionCall<'_>) -> Result<FuncResult, FunctionError> {
        let parameters = ColorParameters::from_config(call.function_config())?;
        let camera = call.devices().get::<CameraDevice>(&parameters.device_id)?;
        run_color_detect(camera, &parameters, call.image_enabled()).await
    }
}

#[cfg(not(feature = "opencv"))]
async fn run_color_detect(
    _camera: std::sync::Arc<CameraDevice>,
    _parameters: &ColorParameters,
    _image_enabled: bool,
) -> Result<FuncResult, FunctionError> {
    Err(FunctionError::Call {
        message: crate::tool::opencv_disabled_message("color_detect"),
    })
}

#[cfg(feature = "opencv")]
async fn run_color_detect(
    camera: std::sync::Arc<CameraDevice>,
    parameters: &ColorParameters,
    image_enabled: bool,
) -> Result<FuncResult, FunctionError> {
    let result = detect_color(camera, parameters).await?;
    let mut output = serde_json::json!({
        "text": format!("color_detect finished: {}", result.name),
        "name": result.name,
        "value": result.value,
        "ratio": result.ratio
    });
    if image_enabled {
        output["image"] = crate::tool::mat_to_jpeg_data_url(&result.frame)
            .map(serde_json::Value::String)
            .map_err(|error| FunctionError::Call {
                message: error.to_string(),
            })?;
    }
    Ok(FuncResult::new(output))
}

#[cfg(all(test, feature = "opencv"))]
mod tests {
    use opencv::{highgui, prelude::*};

    use super::*;

    #[tokio::test]
    async fn test_color() {
        let (config, camera) = crate::vision::test::load_camera("road")
            .await
            .expect("load color camera");
        let camera = camera.get::<CameraDevice>().expect("get color camera");
        let parameters = ColorParameters::from_config(&config.funcs()["color_detect"])
            .expect("load color config");
        let result = detect_color(camera, &parameters)
            .await
            .expect("detect color");
        println!(
            "color={} output={} ratio={:.4}",
            result.name, result.value, result.ratio
        );
    }

    #[tokio::test]
    async fn test_color_show() {
        let (config, camera) = crate::vision::test::load_camera("road")
            .await
            .expect("load color camera");
        let camera = camera.get::<CameraDevice>().expect("get color camera");
        let parameters = ColorParameters::from_config(&config.funcs()["color_detect"])
            .expect("load color config");
        loop {
            let frame = camera.frame().await.expect("read color frame");
            let frame_for_analysis = frame.try_clone().expect("clone color frame");
            let frame_parameters = parameters.clone();
            let result = tokio::task::spawn_blocking(move || {
                analyze_color_frame(&frame_for_analysis, &frame_parameters)
            })
            .await
            .expect("join color analysis")
            .expect("analyze color frame");

            let value = result
                .color
                .as_ref()
                .map(|color| {
                    format!(
                        "{} -> {} ratio={:.2}",
                        color.name, color.output, result.ratio
                    )
                })
                .unwrap_or_else(|| format!("NOT FOUND ratio={:.2}", result.ratio));
            let display = crate::vision::test::annotate_result(&result.frame, "COLOR", &value)
                .expect("annotate color result");

            highgui::imshow("color.original", &frame).expect("show original frame");
            highgui::imshow("color.roi", &result.roi).expect("show color roi");
            for (name, mask) in &result.masks {
                highgui::imshow(&format!("color.mask.{name}"), mask).expect("show color mask");
            }
            highgui::imshow("color.mask.selected", &result.selected_mask)
                .expect("show selected color mask");
            highgui::imshow("color.result", &display).expect("show color result");
            let key = highgui::wait_key(1).expect("wait for color key") & 0xff;
            if key == 113 || key == 27 {
                break;
            }
        }
    }

    #[tokio::test]
    async fn test_hsv() {
        let (config, camera) = crate::vision::test::load_camera("road")
            .await
            .expect("load color camera");
        let camera = camera.get::<CameraDevice>().expect("get color camera");
        let parameters = ColorParameters::from_config(&config.funcs()["color_detect"])
            .expect("load color config");
        let selected_name = std::env::var("HSV_COLOR").unwrap_or_else(|_| "red".to_string());
        let selected_color = parameters
            .colors
            .iter()
            .find(|color| color.name == selected_name)
            .unwrap_or_else(|| {
                panic!(
                    "unknown HSV_COLOR={selected_name}; expected one of: {}",
                    parameters
                        .colors
                        .iter()
                        .map(|color| color.name.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            });
        let selected_range = std::env::var("HSV_RANGE")
            .ok()
            .map(|value| {
                value
                    .parse::<usize>()
                    .expect("HSV_RANGE must be an integer")
            })
            .unwrap_or(0);
        let initial = *selected_color
            .hsv_ranges
            .get(selected_range)
            .unwrap_or_else(|| {
                panic!(
                    "HSV_RANGE={selected_range} is out of range for {}; available: 0..{}",
                    selected_color.name,
                    selected_color.hsv_ranges.len()
                )
            });
        println!(
            "tuning HSV color: {} range: {}",
            selected_color.name, selected_range
        );
        let mut h_min = initial[0];
        let mut h_max = initial[1];
        let mut s_min = initial[2];
        let mut s_max = initial[3];
        let mut v_min = initial[4];
        let mut v_max = initial[5];

        highgui::named_window("hsv.controls", highgui::WINDOW_AUTOSIZE)
            .expect("create HSV controls");
        highgui::create_trackbar("H min", "hsv.controls", Some(&mut h_min), 179, None)
            .expect("create H min trackbar");
        highgui::create_trackbar("H max", "hsv.controls", Some(&mut h_max), 179, None)
            .expect("create H max trackbar");
        highgui::create_trackbar("S min", "hsv.controls", Some(&mut s_min), 255, None)
            .expect("create S min trackbar");
        highgui::create_trackbar("S max", "hsv.controls", Some(&mut s_max), 255, None)
            .expect("create S max trackbar");
        highgui::create_trackbar("V min", "hsv.controls", Some(&mut v_min), 255, None)
            .expect("create V min trackbar");
        highgui::create_trackbar("V max", "hsv.controls", Some(&mut v_max), 255, None)
            .expect("create V max trackbar");

        let mut previous = None;
        loop {
            let frame = camera.frame().await.expect("read HSV frame");
            let range = [
                highgui::get_trackbar_pos("H min", "hsv.controls").expect("read H min"),
                highgui::get_trackbar_pos("H max", "hsv.controls").expect("read H max"),
                highgui::get_trackbar_pos("S min", "hsv.controls").expect("read S min"),
                highgui::get_trackbar_pos("S max", "hsv.controls").expect("read S max"),
                highgui::get_trackbar_pos("V min", "hsv.controls").expect("read V min"),
                highgui::get_trackbar_pos("V max", "hsv.controls").expect("read V max"),
            ];
            if previous != Some(range) {
                println!(
                    "{} hsv_ranges[{selected_range}] = {range:?}",
                    selected_color.name
                );
                previous = Some(range);
            }
            let (roi, roi_mask) = crate::vision::util::circle_roi(&frame, parameters.radius_ratio)
                .expect("build HSV roi");
            let hsv = crate::vision::util::bgr_to_hsv(&frame).expect("convert HSV frame");
            let mask = crate::vision::util::hsv_mask(&hsv, range).expect("build HSV mask");
            let mask =
                crate::vision::util::mask_in_roi(&mask, &roi_mask).expect("apply HSV roi mask");
            let result = crate::vision::util::apply_mask(&frame, &mask).expect("build HSV result");

            highgui::imshow("hsv.original", &frame).expect("show HSV original");
            highgui::imshow("hsv.roi", &roi).expect("show HSV roi");
            highgui::imshow("hsv.mask", &mask).expect("show HSV mask");
            highgui::imshow("hsv.result", &result).expect("show HSV result");
            let key = highgui::wait_key(1).expect("wait for HSV key") & 0xff;
            if key == 113 || key == 27 {
                break;
            }
        }
    }
}
