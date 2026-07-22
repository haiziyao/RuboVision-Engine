use async_trait::async_trait;
use rubo_engine::{
    FuncResult, Function, FunctionCall, FunctionError,
    config::{ConfigAccess, FuncConfig},
};

use crate::device::CameraDevice;

#[derive(Debug, Clone)]
#[cfg_attr(not(feature = "opencv"), allow(dead_code))]
struct LetterParameters {
    device_id: String,
    max_frames: usize,
    confirm_frames: usize,
    black_threshold: i32,
    min_letter_area_ratio: f64,
}

impl LetterParameters {
    fn from_config(config: &FuncConfig) -> Result<Self, FunctionError> {
        let parameters = Self {
            device_id: config.get("device_id")?,
            max_frames: config.get("max_frames")?,
            confirm_frames: config.get("confirm_frames")?,
            black_threshold: config.get("black_threshold")?,
            min_letter_area_ratio: config.get("min_letter_area_ratio")?,
        };
        parameters.validate()?;
        Ok(parameters)
    }

    fn validate(&self) -> Result<(), FunctionError> {
        if self.device_id.trim().is_empty() {
            return Err(FunctionError::Config {
                message: "letter_detect.device_id cannot be empty".to_string(),
            });
        }
        if self.max_frames == 0 {
            return Err(FunctionError::Config {
                message: "letter_detect.max_frames must be greater than 0".to_string(),
            });
        }
        if self.confirm_frames == 0 || self.confirm_frames > self.max_frames {
            return Err(FunctionError::Config {
                message: "letter_detect.confirm_frames must be between 1 and max_frames"
                    .to_string(),
            });
        }
        if !(0..=255).contains(&self.black_threshold) {
            return Err(FunctionError::Config {
                message: "letter_detect.black_threshold must be between 0 and 255".to_string(),
            });
        }
        if !self.min_letter_area_ratio.is_finite()
            || self.min_letter_area_ratio <= 0.0
            || self.min_letter_area_ratio > 1.0
        {
            return Err(FunctionError::Config {
                message: "letter_detect.min_letter_area_ratio must be greater than 0 and at most 1"
                    .to_string(),
            });
        }
        Ok(())
    }
}

#[cfg(feature = "opencv")]
struct LetterFrameResult {
    letter: Option<String>,
    holes: usize,
    gray: opencv::core::Mat,
    black_mask: opencv::core::Mat,
    inner_mask: opencv::core::Mat,
    frame: opencv::core::Mat,
}

#[cfg(feature = "opencv")]
struct LetterResult {
    value: String,
    holes: usize,
    frame: opencv::core::Mat,
}

#[cfg(feature = "opencv")]
fn analyze_letter_frame(
    frame: &opencv::core::Mat,
    parameters: &LetterParameters,
) -> opencv::Result<LetterFrameResult> {
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
    let mut hierarchy = types::VectorOfVec4i::new();
    imgproc::find_contours_with_hierarchy(
        &black_mask,
        &mut contours,
        &mut hierarchy,
        imgproc::RETR_TREE,
        imgproc::CHAIN_APPROX_SIMPLE,
        core::Point::new(0, 0),
    )?;

    let frame_area = (frame.rows() * frame.cols()) as f64;
    let mut ring = None;
    for index in 0..contours.len() {
        let relation = hierarchy.get(index)?;
        if relation[3] >= 0 || relation[2] < 0 {
            continue;
        }
        let contour = contours.get(index)?;
        let area = imgproc::contour_area(&contour, false)?;
        let perimeter = imgproc::arc_length(&contour, true)?;
        if area < frame_area * 0.03 || perimeter <= 0.0 {
            continue;
        }
        let rect = imgproc::bounding_rect(&contour)?;
        if rect.width <= 0 || rect.height <= 0 {
            continue;
        }
        let aspect = rect.width as f64 / rect.height as f64;
        let circularity = 4.0 * PI * area / (perimeter * perimeter);
        if !(0.65..=1.45).contains(&aspect) || circularity < 0.55 {
            continue;
        }
        let inner_index = relation[2] as usize;
        let inner_area = imgproc::contour_area(&contours.get(inner_index)?, false)?;
        if inner_area < area * 0.3 {
            continue;
        }
        if ring
            .as_ref()
            .is_none_or(|current: &(usize, usize, f64)| area > current.2)
        {
            ring = Some((index, inner_index, area));
        }
    }

    let mut annotated = frame.try_clone()?;
    let mut inner_mask = core::Mat::zeros(frame.rows(), frame.cols(), core::CV_8UC1)?.to_mat()?;
    let mut letter = None;
    let mut holes = 0;

    if let Some((ring_index, inner_index, _)) = ring {
        let ring_contour = contours.get(ring_index)?;
        let mut ring_center = core::Point2f::default();
        let mut ring_radius = 0.0_f32;
        imgproc::min_enclosing_circle(&ring_contour, &mut ring_center, &mut ring_radius)?;
        imgproc::circle(
            &mut annotated,
            core::Point::new(ring_center.x.round() as i32, ring_center.y.round() as i32),
            ring_radius.round() as i32,
            core::Scalar::new(0.0, 255.0, 255.0, 0.0),
            2,
            imgproc::LINE_AA,
            0,
        )?;

        let inner_contour = contours.get(inner_index)?;
        let inner_area = imgproc::contour_area(&inner_contour, false)?;
        let mut inner_center = core::Point2f::default();
        let mut inner_radius = 0.0_f32;
        imgproc::min_enclosing_circle(&inner_contour, &mut inner_center, &mut inner_radius)?;
        imgproc::circle(
            &mut inner_mask,
            core::Point::new(inner_center.x.round() as i32, inner_center.y.round() as i32),
            inner_radius.round() as i32,
            core::Scalar::all(255.0),
            -1,
            imgproc::LINE_8,
            0,
        )?;
        let source_mask = black_mask.try_clone()?;
        let roi_mask = inner_mask.try_clone()?;
        core::bitwise_and(&source_mask, &source_mask, &mut inner_mask, &roi_mask)?;

        let mut letter_index = None;
        let mut letter_area = 0.0_f64;
        let mut child = hierarchy.get(inner_index)?[2];
        while child >= 0 {
            let child_index = child as usize;
            let area = imgproc::contour_area(&contours.get(child_index)?, false)?;
            if area > letter_area {
                letter_area = area;
                letter_index = Some(child_index);
            }
            child = hierarchy.get(child_index)?[0];
        }

        if let Some(letter_index) = letter_index
            && letter_area / inner_area >= parameters.min_letter_area_ratio
        {
            let mut hole = hierarchy.get(letter_index)?[2];
            while hole >= 0 {
                let hole_index = hole as usize;
                let hole_area = imgproc::contour_area(&contours.get(hole_index)?, false)?;
                if hole_area / letter_area >= 0.01 {
                    holes += 1;
                }
                hole = hierarchy.get(hole_index)?[0];
            }
            letter = match holes {
                0 => Some("C".to_string()),
                1 => Some("A".to_string()),
                2 => Some("B".to_string()),
                _ => None,
            };
            let rect = imgproc::bounding_rect(&contours.get(letter_index)?)?;
            imgproc::rectangle(
                &mut annotated,
                rect,
                core::Scalar::new(0.0, 255.0, 0.0, 0.0),
                2,
                imgproc::LINE_AA,
                0,
            )?;
        }
    }

    let label = letter.as_deref().unwrap_or("unknown");
    imgproc::put_text(
        &mut annotated,
        &format!("letter: {label} holes: {holes}"),
        core::Point::new(10, 30),
        imgproc::FONT_HERSHEY_SIMPLEX,
        0.8,
        core::Scalar::new(255.0, 255.0, 255.0, 0.0),
        2,
        imgproc::LINE_AA,
        false,
    )?;

    Ok(LetterFrameResult {
        letter,
        holes,
        gray,
        black_mask,
        inner_mask,
        frame: annotated,
    })
}

#[cfg(feature = "opencv")]
async fn detect_letter(
    camera: std::sync::Arc<CameraDevice>,
    parameters: &LetterParameters,
) -> Result<LetterResult, FunctionError> {
    let mut candidate = None;
    let mut consecutive_frames = 0;

    for _ in 0..parameters.max_frames {
        let frame = camera.frame().await?;
        let frame_parameters = parameters.clone();
        let frame_result =
            tokio::task::spawn_blocking(move || analyze_letter_frame(&frame, &frame_parameters))
                .await
                .map_err(|error| FunctionError::Call {
                    message: format!("letter_detect task failed: {error}"),
                })?
                .map_err(|error| FunctionError::Call {
                    message: error.to_string(),
                })?;

        let Some(letter) = frame_result.letter.as_deref() else {
            candidate = None;
            consecutive_frames = 0;
            continue;
        };
        if candidate.as_deref() == Some(letter) {
            consecutive_frames += 1;
        } else {
            candidate = Some(letter.to_string());
            consecutive_frames = 1;
        }
        if consecutive_frames >= parameters.confirm_frames {
            return Ok(LetterResult {
                value: letter.to_string(),
                holes: frame_result.holes,
                frame: frame_result.frame,
            });
        }
    }

    Err(FunctionError::Call {
        message: format!(
            "letter was not confirmed within {} frames",
            parameters.max_frames
        ),
    })
}

#[rubo_engine::function(id = "letter_detect")]
#[derive(Default)]
pub struct LetterDetect;

#[async_trait]
impl Function for LetterDetect {
    async fn call(&self, call: FunctionCall<'_>) -> Result<FuncResult, FunctionError> {
        let parameters = LetterParameters::from_config(call.function_config())?;
        let camera = call.devices().get::<CameraDevice>(&parameters.device_id)?;
        run_letter_detect(camera, &parameters, call.image_enabled()).await
    }
}

#[cfg(not(feature = "opencv"))]
async fn run_letter_detect(
    _camera: std::sync::Arc<CameraDevice>,
    _parameters: &LetterParameters,
    _image_enabled: bool,
) -> Result<FuncResult, FunctionError> {
    Err(FunctionError::Call {
        message: crate::tool::opencv_disabled_message("letter_detect"),
    })
}

#[cfg(feature = "opencv")]
async fn run_letter_detect(
    camera: std::sync::Arc<CameraDevice>,
    parameters: &LetterParameters,
    image_enabled: bool,
) -> Result<FuncResult, FunctionError> {
    let result = detect_letter(camera, parameters).await?;
    let mut output = serde_json::json!({
        "text": format!("letter_detect finished: {}", result.value),
        "value": result.value,
        "holes": result.holes
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
    async fn test_letter() {
        let (config, camera) = crate::vision::test::load_camera("road")
            .await
            .expect("load letter camera");
        let camera = camera.get::<CameraDevice>().expect("get letter camera");
        let parameters = LetterParameters::from_config(&config.funcs()["letter_detect"])
            .expect("load letter config");
        let result = detect_letter(camera, &parameters)
            .await
            .expect("detect letter");
        println!("letter={} holes={}", result.value, result.holes);
    }

    #[tokio::test]
    async fn test_letter_show() {
        let (config, camera) = crate::vision::test::load_camera("road")
            .await
            .expect("load letter camera");
        let camera = camera.get::<CameraDevice>().expect("get letter camera");
        let parameters = LetterParameters::from_config(&config.funcs()["letter_detect"])
            .expect("load letter config");
        loop {
            let frame = camera.frame().await.expect("read letter frame");
            let frame_for_analysis = frame.try_clone().expect("clone letter frame");
            let frame_parameters = parameters.clone();
            let result = tokio::task::spawn_blocking(move || {
                analyze_letter_frame(&frame_for_analysis, &frame_parameters)
            })
            .await
            .expect("join letter analysis")
            .expect("analyze letter frame");
            let value = format!(
                "{} holes={}",
                result.letter.as_deref().unwrap_or("NOT FOUND"),
                result.holes
            );
            let display = crate::vision::test::annotate_result(&result.frame, "LETTER", &value)
                .expect("annotate letter result");
            highgui::imshow("letter.original", &frame).expect("show letter original");
            highgui::imshow("letter.gray", &result.gray).expect("show letter gray");
            highgui::imshow("letter.mask", &result.black_mask).expect("show letter mask");
            highgui::imshow("letter.inner", &result.inner_mask).expect("show letter inner mask");
            highgui::imshow("letter.result", &display).expect("show letter result");
            let key = highgui::wait_key(1).expect("wait for letter key") & 0xff;
            if key == 113 || key == 27 {
                break;
            }
        }
    }
}
