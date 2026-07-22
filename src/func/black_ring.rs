use async_trait::async_trait;
use rubo_engine::{
    FuncResult, Function, FunctionCall, FunctionError,
    config::{ConfigAccess, FuncConfig},
};
use serde::Deserialize;

use crate::device::CameraDevice;

#[derive(Debug, Clone, Copy, Deserialize)]
#[cfg_attr(not(feature = "opencv"), allow(dead_code))]
struct TargetCorrection {
    x: i32,
    y: i32,
}

#[derive(Debug, Clone)]
#[cfg_attr(not(feature = "opencv"), allow(dead_code))]
struct BlackRingParameters {
    device_id: String,
    max_frames: usize,
    target_correction: TargetCorrection,
    black_threshold: i32,
    min_radius: f64,
    max_radius: f64,
    min_circularity: f64,
    min_score: u8,
}

impl BlackRingParameters {
    fn from_config(config: &FuncConfig) -> Result<Self, FunctionError> {
        let parameters = Self {
            device_id: config.get("device_id")?,
            max_frames: config.get("max_frames")?,
            target_correction: config.get("target_correction")?,
            black_threshold: config.get("black_threshold")?,
            min_radius: config.get("min_radius")?,
            max_radius: config.get("max_radius")?,
            min_circularity: config.get("min_circularity")?,
            min_score: config.get("min_score")?,
        };
        parameters.validate()?;
        Ok(parameters)
    }

    fn validate(&self) -> Result<(), FunctionError> {
        if self.device_id.trim().is_empty() {
            return Err(FunctionError::Config {
                message: "black_ring_detect.device_id cannot be empty".to_string(),
            });
        }
        if self.max_frames == 0 {
            return Err(FunctionError::Config {
                message: "black_ring_detect.max_frames must be greater than 0".to_string(),
            });
        }
        if !(0..=255).contains(&self.black_threshold) {
            return Err(FunctionError::Config {
                message: "black_ring_detect.black_threshold must be between 0 and 255".to_string(),
            });
        }
        if !self.min_radius.is_finite()
            || !self.max_radius.is_finite()
            || self.min_radius <= 0.0
            || self.max_radius < self.min_radius
        {
            return Err(FunctionError::Config {
                message: "black_ring_detect radii must be finite, positive, and ordered"
                    .to_string(),
            });
        }
        if !self.min_circularity.is_finite()
            || self.min_circularity <= 0.0
            || self.min_circularity > 1.0
        {
            return Err(FunctionError::Config {
                message: "black_ring_detect.min_circularity must be greater than 0 and at most 1"
                    .to_string(),
            });
        }
        if self.min_score > 100 {
            return Err(FunctionError::Config {
                message: "black_ring_detect.min_score must be at most 100".to_string(),
            });
        }
        Ok(())
    }
}

#[cfg(feature = "opencv")]
struct BlackRingFrameResult {
    found: bool,
    dx: i32,
    dy: i32,
    score: u8,
    gray: opencv::core::Mat,
    black_mask: opencv::core::Mat,
    frame: opencv::core::Mat,
}

#[cfg(feature = "opencv")]
struct BlackRingCandidate {
    center: opencv::core::Point2f,
    radius: f32,
    score: u8,
}

#[cfg(feature = "opencv")]
fn analyze_black_ring_frame(
    frame: &opencv::core::Mat,
    parameters: &BlackRingParameters,
) -> opencv::Result<BlackRingFrameResult> {
    use opencv::{core, imgproc, prelude::*, types};
    use std::f64::consts::PI;

    let gray = crate::vision::util::bgr_to_gray(frame)?;
    let mut black_mask = core::Mat::default();
    imgproc::threshold(
        &gray,
        &mut black_mask,
        parameters.black_threshold as f64,
        255.0,
        imgproc::THRESH_BINARY_INV,
    )?;
    let mut contours = types::VectorOfVectorOfPoint::new();
    imgproc::find_contours(
        &black_mask,
        &mut contours,
        imgproc::RETR_EXTERNAL,
        imgproc::CHAIN_APPROX_SIMPLE,
        core::Point::new(0, 0),
    )?;

    let mut best = None;
    for index in 0..contours.len() {
        let contour = contours.get(index)?;
        if contour.len() < 5 {
            continue;
        }
        let area = imgproc::contour_area(&contour, false)?;
        let perimeter = imgproc::arc_length(&contour, true)?;
        if area <= 0.0 || perimeter <= 0.0 {
            continue;
        }
        let rect = imgproc::bounding_rect(&contour)?;
        if rect.width <= 0 || rect.height <= 0 {
            continue;
        }
        let aspect = rect.width as f64 / rect.height as f64;
        if !(0.65..=1.45).contains(&aspect) {
            continue;
        }
        let mut center = core::Point2f::default();
        let mut radius = 0.0_f32;
        imgproc::min_enclosing_circle(&contour, &mut center, &mut radius)?;
        if (radius as f64) < parameters.min_radius || (radius as f64) > parameters.max_radius {
            continue;
        }
        let circularity = (4.0 * PI * area / (perimeter * perimeter)).clamp(0.0, 1.0);
        if circularity < parameters.min_circularity {
            continue;
        }
        let score = (circularity * 100.0).round().clamp(0.0, 100.0) as u8;
        if score < parameters.min_score {
            continue;
        }
        let candidate = BlackRingCandidate {
            center,
            radius,
            score,
        };
        if best
            .as_ref()
            .is_none_or(|current: &BlackRingCandidate| candidate.score > current.score)
        {
            best = Some(candidate);
        }
    }

    let size = frame.size()?;
    let target = core::Point::new(
        size.width / 2 + parameters.target_correction.x,
        size.height / 2 + parameters.target_correction.y,
    );
    let mut annotated = frame.try_clone()?;
    draw_marker(
        &mut annotated,
        target,
        core::Scalar::new(255.0, 0.0, 0.0, 0.0),
    )?;

    let (found, dx, dy, score) = match best {
        Some(candidate) => {
            let point = core::Point::new(
                candidate.center.x.round() as i32,
                candidate.center.y.round() as i32,
            );
            imgproc::circle(
                &mut annotated,
                point,
                candidate.radius.round() as i32,
                core::Scalar::new(0.0, 255.0, 0.0, 0.0),
                2,
                imgproc::LINE_AA,
                0,
            )?;
            draw_marker(
                &mut annotated,
                point,
                core::Scalar::new(0.0, 255.0, 0.0, 0.0),
            )?;
            (
                true,
                point.x - target.x,
                point.y - target.y,
                candidate.score,
            )
        }
        None => (false, 0, 0, 0),
    };
    let value = format_black_ring_value(found, dx, dy, score);
    imgproc::put_text(
        &mut annotated,
        &value,
        core::Point::new(10, 30),
        imgproc::FONT_HERSHEY_SIMPLEX,
        0.8,
        core::Scalar::new(255.0, 255.0, 255.0, 0.0),
        2,
        imgproc::LINE_AA,
        false,
    )?;

    Ok(BlackRingFrameResult {
        found,
        dx,
        dy,
        score,
        gray,
        black_mask,
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
fn format_black_ring_value(found: bool, dx: i32, dy: i32, score: u8) -> String {
    if found {
        format!("RING,1,{dx},{dy},{score}")
    } else {
        "RING,0,0,0,0".to_string()
    }
}

#[cfg(feature = "opencv")]
async fn detect_black_ring(
    camera: std::sync::Arc<CameraDevice>,
    parameters: &BlackRingParameters,
) -> Result<BlackRingFrameResult, FunctionError> {
    let mut best = None;
    for _ in 0..parameters.max_frames {
        let frame = camera.frame().await?;
        let frame_parameters = parameters.clone();
        let result = tokio::task::spawn_blocking(move || {
            analyze_black_ring_frame(&frame, &frame_parameters)
        })
        .await
        .map_err(|error| FunctionError::Call {
            message: format!("black_ring_detect task failed: {error}"),
        })?
        .map_err(|error| FunctionError::Call {
            message: error.to_string(),
        })?;
        if best
            .as_ref()
            .is_none_or(|current: &BlackRingFrameResult| result.score > current.score)
        {
            best = Some(result);
        }
    }
    best.ok_or_else(|| FunctionError::Call {
        message: "black_ring_detect received no camera frames".to_string(),
    })
}

#[rubo_engine::function(id = "black_ring_detect")]
#[derive(Default)]
pub struct BlackRingDetect;

#[async_trait]
impl Function for BlackRingDetect {
    async fn call(&self, call: FunctionCall<'_>) -> Result<FuncResult, FunctionError> {
        let parameters = BlackRingParameters::from_config(call.function_config())?;
        let camera = call.devices().get::<CameraDevice>(&parameters.device_id)?;
        run_black_ring_detect(camera, &parameters, call.image_enabled()).await
    }
}

#[cfg(not(feature = "opencv"))]
async fn run_black_ring_detect(
    _camera: std::sync::Arc<CameraDevice>,
    _parameters: &BlackRingParameters,
    _image_enabled: bool,
) -> Result<FuncResult, FunctionError> {
    Err(FunctionError::Call {
        message: crate::tool::opencv_disabled_message("black_ring_detect"),
    })
}

#[cfg(feature = "opencv")]
async fn run_black_ring_detect(
    camera: std::sync::Arc<CameraDevice>,
    parameters: &BlackRingParameters,
    image_enabled: bool,
) -> Result<FuncResult, FunctionError> {
    let result = detect_black_ring(camera, parameters).await?;
    let value = format_black_ring_value(result.found, result.dx, result.dy, result.score);
    let mut output = serde_json::json!({
        "text": format!("black_ring_detect finished: {value}"),
        "value": value,
        "found": result.found,
        "dx": result.dx,
        "dy": result.dy,
        "score": result.score
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
    async fn test_black_ring() {
        let (config, camera) = crate::vision::test::load_camera("road")
            .await
            .expect("load black ring camera");
        let camera = camera.get::<CameraDevice>().expect("get black ring camera");
        let parameters = BlackRingParameters::from_config(&config.funcs()["black_ring_detect"])
            .expect("load black ring config");
        let result = detect_black_ring(camera, &parameters)
            .await
            .expect("detect black ring");
        println!(
            "{}",
            format_black_ring_value(result.found, result.dx, result.dy, result.score)
        );
    }

    #[tokio::test]
    async fn test_black_ring_show() {
        let (config, camera) = crate::vision::test::load_camera("road")
            .await
            .expect("load black ring camera");
        let camera = camera.get::<CameraDevice>().expect("get black ring camera");
        let parameters = BlackRingParameters::from_config(&config.funcs()["black_ring_detect"])
            .expect("load black ring config");
        loop {
            let frame = camera.frame().await.expect("read black ring frame");
            let frame_for_analysis = frame.try_clone().expect("clone black ring frame");
            let frame_parameters = parameters.clone();
            let result = tokio::task::spawn_blocking(move || {
                analyze_black_ring_frame(&frame_for_analysis, &frame_parameters)
            })
            .await
            .expect("join black ring analysis")
            .expect("analyze black ring frame");
            highgui::imshow("black_ring.original", &frame).expect("show black ring original");
            highgui::imshow("black_ring.gray", &result.gray).expect("show black ring gray");
            highgui::imshow("black_ring.mask", &result.black_mask).expect("show black ring mask");
            highgui::imshow("black_ring.result", &result.frame).expect("show black ring result");
            let key = highgui::wait_key(1).expect("wait for black ring key") & 0xff;
            if key == 113 || key == 27 {
                break;
            }
        }
    }
}
