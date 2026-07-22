use async_trait::async_trait;
use rubo_engine::{
    FuncResult, Function, FunctionCall, FunctionError,
    config::{ConfigAccess, FuncConfig},
};
use serde::Deserialize;

use crate::device::CameraDevice;

use super::color::ColorDefinition;
#[cfg(feature = "opencv")]
use super::color::color_mask;

#[derive(Debug, Clone, Copy, Deserialize)]
#[cfg_attr(not(feature = "opencv"), allow(dead_code))]
struct TargetCorrection {
    x: i32,
    y: i32,
}

#[derive(Debug, Clone)]
#[cfg_attr(not(feature = "opencv"), allow(dead_code))]
struct ColorBlockParameters {
    device_id: String,
    max_frames: usize,
    target_correction: TargetCorrection,
    colors: Vec<ColorDefinition>,
    open_kernel_size: i32,
    close_kernel_size: i32,
    min_area: f64,
    max_area_ratio: f64,
    edge_margin: i32,
}

impl ColorBlockParameters {
    fn from_config(config: &FuncConfig) -> Result<Self, FunctionError> {
        let parameters = Self {
            device_id: config.get("device_id")?,
            max_frames: config.get("max_frames")?,
            target_correction: config.get("target_correction")?,
            colors: config.get("colors")?,
            open_kernel_size: config.get("open_kernel_size")?,
            close_kernel_size: config.get("close_kernel_size")?,
            min_area: config.get("min_area")?,
            max_area_ratio: config.get("max_area_ratio")?,
            edge_margin: config.get("edge_margin")?,
        };
        parameters.validate()?;
        Ok(parameters)
    }

    fn validate(&self) -> Result<(), FunctionError> {
        if self.device_id.trim().is_empty() {
            return Err(config_error("color_block_detect.device_id cannot be empty"));
        }
        if self.max_frames == 0 {
            return Err(config_error(
                "color_block_detect.max_frames must be greater than 0",
            ));
        }
        if self.colors.is_empty() {
            return Err(config_error("color_block_detect.colors cannot be empty"));
        }
        for color in &self.colors {
            color.validate("color_block_detect")?;
        }
        for (name, size) in [
            ("open_kernel_size", self.open_kernel_size),
            ("close_kernel_size", self.close_kernel_size),
        ] {
            if size <= 0 || size % 2 == 0 {
                return Err(config_error(format!(
                    "color_block_detect.{name} must be a positive odd number"
                )));
            }
        }
        if !self.min_area.is_finite() || self.min_area <= 0.0 {
            return Err(config_error(
                "color_block_detect.min_area must be a finite number greater than 0",
            ));
        }
        if !self.max_area_ratio.is_finite()
            || self.max_area_ratio <= 0.0
            || self.max_area_ratio > 1.0
        {
            return Err(config_error(
                "color_block_detect.max_area_ratio must be greater than 0 and at most 1",
            ));
        }
        if self.edge_margin < 0 {
            return Err(config_error(
                "color_block_detect.edge_margin cannot be negative",
            ));
        }
        Ok(())
    }
}

fn config_error(message: impl Into<String>) -> FunctionError {
    FunctionError::Config {
        message: message.into(),
    }
}

#[cfg(feature = "opencv")]
struct ColorBlockFrameResult {
    found: bool,
    color: String,
    output: String,
    center_x: i32,
    center_y: i32,
    dx: i32,
    dy: i32,
    area: f64,
    bbox_x: i32,
    bbox_y: i32,
    bbox_width: i32,
    bbox_height: i32,
    mask: opencv::core::Mat,
    frame: opencv::core::Mat,
}

#[cfg(feature = "opencv")]
struct ColorBlockCandidate {
    color: String,
    output: String,
    center: opencv::core::Point,
    rect: opencv::core::Rect,
    area: f64,
    mask: opencv::core::Mat,
}

#[cfg(feature = "opencv")]
fn touches_frame_edge(rect: opencv::core::Rect, size: opencv::core::Size, margin: i32) -> bool {
    rect.x <= margin
        || rect.y <= margin
        || rect.x + rect.width >= size.width - margin
        || rect.y + rect.height >= size.height - margin
}

#[cfg(feature = "opencv")]
fn analyze_color_block_frame(
    frame: &opencv::core::Mat,
    parameters: &ColorBlockParameters,
) -> opencv::Result<ColorBlockFrameResult> {
    use opencv::{core, imgproc, prelude::*};

    let hsv = crate::vision::util::bgr_to_hsv(frame)?;
    let size = frame.size()?;
    let max_area = size.width as f64 * size.height as f64 * parameters.max_area_ratio;
    let open_kernel = imgproc::get_structuring_element(
        imgproc::MORPH_ELLIPSE,
        core::Size::new(parameters.open_kernel_size, parameters.open_kernel_size),
        core::Point::new(-1, -1),
    )?;
    let close_kernel = imgproc::get_structuring_element(
        imgproc::MORPH_ELLIPSE,
        core::Size::new(parameters.close_kernel_size, parameters.close_kernel_size),
        core::Point::new(-1, -1),
    )?;
    let mut best: Option<ColorBlockCandidate> = None;
    for color in &parameters.colors {
        let mask = color_mask(&hsv, color)?;

        let mut opened = core::Mat::default();
        imgproc::morphology_ex(
            &mask,
            &mut opened,
            imgproc::MORPH_OPEN,
            &open_kernel,
            core::Point::new(-1, -1),
            1,
            core::BORDER_CONSTANT,
            imgproc::morphology_default_border_value()?,
        )?;
        let mut cleaned_mask = core::Mat::default();
        imgproc::morphology_ex(
            &opened,
            &mut cleaned_mask,
            imgproc::MORPH_CLOSE,
            &close_kernel,
            core::Point::new(-1, -1),
            1,
            core::BORDER_CONSTANT,
            imgproc::morphology_default_border_value()?,
        )?;

        let mut contours = core::Vector::<core::Vector<core::Point>>::new();
        imgproc::find_contours(
            &cleaned_mask,
            &mut contours,
            imgproc::RETR_EXTERNAL,
            imgproc::CHAIN_APPROX_SIMPLE,
            core::Point::new(0, 0),
        )?;

        for index in 0..contours.len() {
            let contour = contours.get(index)?;
            let area = imgproc::contour_area(&contour, false)?;
            if area < parameters.min_area || area > max_area {
                continue;
            }
            let rect = imgproc::bounding_rect(&contour)?;
            if rect.width <= 0
                || rect.height <= 0
                || touches_frame_edge(rect, size, parameters.edge_margin)
            {
                continue;
            }
            let moments = imgproc::moments(&contour, false)?;
            let center = if moments.m00.abs() > f64::EPSILON {
                core::Point::new(
                    (moments.m10 / moments.m00).round() as i32,
                    (moments.m01 / moments.m00).round() as i32,
                )
            } else {
                core::Point::new(rect.x + rect.width / 2, rect.y + rect.height / 2)
            };
            let candidate = ColorBlockCandidate {
                color: color.name.clone(),
                output: color.output().to_string(),
                center,
                rect,
                area,
                mask: cleaned_mask.try_clone()?,
            };
            if best
                .as_ref()
                .is_none_or(|current| candidate.area > current.area)
            {
                best = Some(candidate);
            }
        }
    }

    let target = core::Point::new(
        size.width / 2 + parameters.target_correction.x,
        size.height / 2 + parameters.target_correction.y,
    );
    let mut annotated = frame.try_clone()?;
    draw_marker(
        &mut annotated,
        target,
        core::Scalar::new(0.0, 0.0, 255.0, 0.0),
    )?;

    let (found, color, output, center_x, center_y, dx, dy, area, rect, selected_mask) = match best {
        Some(candidate) => {
            imgproc::rectangle(
                &mut annotated,
                candidate.rect,
                core::Scalar::new(0.0, 255.0, 0.0, 0.0),
                3,
                imgproc::LINE_AA,
                0,
            )?;
            draw_marker(
                &mut annotated,
                candidate.center,
                core::Scalar::new(0.0, 255.0, 0.0, 0.0),
            )?;
            imgproc::line(
                &mut annotated,
                target,
                candidate.center,
                core::Scalar::new(0.0, 255.0, 255.0, 0.0),
                2,
                imgproc::LINE_AA,
                0,
            )?;
            (
                true,
                candidate.color,
                candidate.output,
                candidate.center.x,
                candidate.center.y,
                candidate.center.x - target.x,
                candidate.center.y - target.y,
                candidate.area,
                candidate.rect,
                candidate.mask,
            )
        }
        None => (
            false,
            "unknown".to_string(),
            "unknown".to_string(),
            0,
            0,
            0,
            0,
            0.0,
            core::Rect::default(),
            core::Mat::zeros(size.height, size.width, core::CV_8UC1)?.to_mat()?,
        ),
    };
    let value = format_color_block_value(&output, found, dx, dy);
    imgproc::put_text(
        &mut annotated,
        &format!("{value} center=({center_x},{center_y}) area={area:.0}"),
        core::Point::new(10, 32),
        imgproc::FONT_HERSHEY_SIMPLEX,
        0.75,
        core::Scalar::new(255.0, 255.0, 255.0, 0.0),
        2,
        imgproc::LINE_AA,
        false,
    )?;

    Ok(ColorBlockFrameResult {
        found,
        color,
        output,
        center_x,
        center_y,
        dx,
        dy,
        area,
        bbox_x: rect.x,
        bbox_y: rect.y,
        bbox_width: rect.width,
        bbox_height: rect.height,
        mask: selected_mask,
        frame: annotated,
    })
}

#[cfg(feature = "opencv")]
fn draw_marker(
    frame: &mut opencv::core::Mat,
    center: opencv::core::Point,
    color: opencv::core::Scalar,
) -> opencv::Result<()> {
    use opencv::imgproc;

    let half = 12;
    imgproc::line(
        frame,
        opencv::core::Point::new(center.x - half, center.y),
        opencv::core::Point::new(center.x + half, center.y),
        color,
        2,
        imgproc::LINE_AA,
        0,
    )?;
    imgproc::line(
        frame,
        opencv::core::Point::new(center.x, center.y - half),
        opencv::core::Point::new(center.x, center.y + half),
        color,
        2,
        imgproc::LINE_AA,
        0,
    )
}

#[cfg(feature = "opencv")]
fn format_color_block_value(color: &str, found: bool, dx: i32, dy: i32) -> String {
    format!("BLOCK,{color},{},{dx},{dy}", u8::from(found))
}

#[cfg(feature = "opencv")]
async fn detect_color_block(
    camera: std::sync::Arc<CameraDevice>,
    parameters: &ColorBlockParameters,
) -> Result<ColorBlockFrameResult, FunctionError> {
    let mut best = None;
    for _ in 0..parameters.max_frames {
        let frame = camera.frame().await?;
        let frame_parameters = parameters.clone();
        let result = tokio::task::spawn_blocking(move || {
            analyze_color_block_frame(&frame, &frame_parameters)
        })
        .await
        .map_err(|error| FunctionError::Call {
            message: format!("color_block_detect task failed: {error}"),
        })?
        .map_err(|error| FunctionError::Call {
            message: error.to_string(),
        })?;

        if result.found
            && best.as_ref().is_none_or(|current: &ColorBlockFrameResult| {
                !current.found || result.area > current.area
            })
        {
            best = Some(result);
        } else if best.is_none() {
            best = Some(result);
        }
    }
    best.ok_or_else(|| FunctionError::Call {
        message: "color_block_detect did not receive a frame".to_string(),
    })
}

#[rubo_engine::function(id = "color_block_detect")]
#[derive(Default)]
pub struct ColorBlockDetect;

#[async_trait]
impl Function for ColorBlockDetect {
    async fn call(&self, call: FunctionCall<'_>) -> Result<FuncResult, FunctionError> {
        let parameters = ColorBlockParameters::from_config(call.function_config())?;
        let camera = call.devices().get::<CameraDevice>(&parameters.device_id)?;
        run_color_block_detect(camera, &parameters, call.image_enabled()).await
    }
}

#[cfg(not(feature = "opencv"))]
async fn run_color_block_detect(
    _camera: std::sync::Arc<CameraDevice>,
    _parameters: &ColorBlockParameters,
    _image_enabled: bool,
) -> Result<FuncResult, FunctionError> {
    Err(FunctionError::Call {
        message: crate::tool::opencv_disabled_message("color_block_detect"),
    })
}

#[cfg(feature = "opencv")]
async fn run_color_block_detect(
    camera: std::sync::Arc<CameraDevice>,
    parameters: &ColorBlockParameters,
    image_enabled: bool,
) -> Result<FuncResult, FunctionError> {
    let result = detect_color_block(camera, parameters).await?;
    let value = format_color_block_value(&result.output, result.found, result.dx, result.dy);
    let mut output = serde_json::json!({
        "text": format!(
            "color_block_detect finished: color={} found={} center=({}, {}) offset=({}, {}) area={:.0}",
            result.color, result.found, result.center_x, result.center_y, result.dx, result.dy, result.area
        ),
        "value": value,
        "found": result.found,
        "color": result.color,
        "color_output": result.output,
        "center_x": result.center_x,
        "center_y": result.center_y,
        "dx": result.dx,
        "dy": result.dy,
        "area": result.area,
        "bbox": {
            "x": result.bbox_x,
            "y": result.bbox_y,
            "width": result.bbox_width,
            "height": result.bbox_height
        }
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
    use opencv::highgui;

    use super::*;

    #[test]
    fn format_color_block_value_test() {
        assert_eq!(
            format_color_block_value("blue", true, -90, -170),
            "BLOCK,blue,1,-90,-170"
        );
        assert_eq!(
            format_color_block_value("unknown", false, 0, 0),
            "BLOCK,unknown,0,0,0"
        );
    }

    #[test]
    fn detects_every_configured_color_test() {
        use opencv::{core, imgproc};

        let config = rubo_engine::config::ConfigStore::load_active_config("config/ubuntu")
            .expect("load ubuntu config");
        let parameters = ColorBlockParameters::from_config(&config.funcs()["color_block_detect"])
            .expect("load color block config");
        let colors = [
            ("red", core::Scalar::new(0.0, 0.0, 255.0, 0.0)),
            ("blue", core::Scalar::new(255.0, 0.0, 0.0, 0.0)),
            ("green", core::Scalar::new(0.0, 255.0, 0.0, 0.0)),
            ("black", core::Scalar::new(0.0, 0.0, 0.0, 0.0)),
            ("white", core::Scalar::new(255.0, 255.0, 255.0, 0.0)),
        ];

        for (name, bgr) in colors {
            let mut frame = core::Mat::new_rows_cols_with_default(
                480,
                640,
                core::CV_8UC3,
                core::Scalar::new(100.0, 100.0, 100.0, 0.0),
            )
            .expect("create synthetic frame");
            imgproc::circle(
                &mut frame,
                core::Point::new(240, 160),
                80,
                bgr,
                -1,
                imgproc::LINE_AA,
                0,
            )
            .expect("draw synthetic color block");

            let result = analyze_color_block_frame(&frame, &parameters)
                .expect("analyze synthetic color block");
            assert!(result.found, "{name} block should be found");
            assert_eq!(result.color, name);
            assert!((result.center_x - 240).abs() <= 1);
            assert!((result.center_y - 160).abs() <= 1);
        }
    }

    #[tokio::test]
    async fn test_color_block() {
        let (config, camera) = crate::vision::test::load_camera("road")
            .await
            .expect("load color block camera");
        let camera = camera
            .get::<CameraDevice>()
            .expect("get color block camera");
        let parameters = ColorBlockParameters::from_config(&config.funcs()["color_block_detect"])
            .expect("load color block config");
        let result = detect_color_block(camera, &parameters)
            .await
            .expect("detect color block");
        println!(
            "{} center=({}, {}) bbox=({}, {}, {}, {}) area={:.0}",
            format_color_block_value(&result.output, result.found, result.dx, result.dy),
            result.center_x,
            result.center_y,
            result.bbox_x,
            result.bbox_y,
            result.bbox_width,
            result.bbox_height,
            result.area
        );
    }

    #[tokio::test]
    async fn test_color_block_show() {
        let (config, camera) = crate::vision::test::load_camera("road")
            .await
            .expect("load color block camera");
        let camera = camera
            .get::<CameraDevice>()
            .expect("get color block camera");
        let parameters = ColorBlockParameters::from_config(&config.funcs()["color_block_detect"])
            .expect("load color block config");
        loop {
            let result = detect_color_block(camera.clone(), &parameters)
                .await
                .expect("detect color block");
            highgui::imshow("color_block.mask", &result.mask).expect("show color block mask");
            highgui::imshow("color_block.result", &result.frame).expect("show color block result");
            let key = highgui::wait_key(1).expect("wait for color block key") & 0xff;
            if key == 113 || key == 27 {
                break;
            }
        }
    }
}
