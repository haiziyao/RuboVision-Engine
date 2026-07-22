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
struct CrossParameters {
    device_id: String,
    max_frames: usize,
    target_correction: TargetCorrection,
    black_threshold: i32,
    close_kernel_size: i32,
    dilate_kernel_size: i32,
    dilate_iterations: i32,
    min_radius: f64,
    max_radius: f64,
    center_tolerance: f64,
    min_arc_points: usize,
    min_ring_score: u8,
}

impl CrossParameters {
    fn from_config(config: &FuncConfig) -> Result<Self, FunctionError> {
        let parameters = Self {
            device_id: config.get("device_id")?,
            max_frames: config.get("max_frames")?,
            target_correction: config.get("target_correction")?,
            black_threshold: config.get("black_threshold")?,
            close_kernel_size: config.get("close_kernel_size")?,
            dilate_kernel_size: config.get("dilate_kernel_size")?,
            dilate_iterations: config.get("dilate_iterations")?,
            min_radius: config.get("min_radius")?,
            max_radius: config.get("max_radius")?,
            center_tolerance: config.get("center_tolerance")?,
            min_arc_points: config.get("min_arc_points")?,
            min_ring_score: config.get("min_ring_score")?,
        };
        parameters.validate()?;
        Ok(parameters)
    }

    fn validate(&self) -> Result<(), FunctionError> {
        if self.device_id.trim().is_empty() {
            return Err(FunctionError::Config {
                message: "cross.device_id cannot be empty".to_string(),
            });
        }
        if self.max_frames == 0 {
            return Err(FunctionError::Config {
                message: "cross.max_frames must be greater than 0".to_string(),
            });
        }
        if !(0..=255).contains(&self.black_threshold) {
            return Err(FunctionError::Config {
                message: "cross.black_threshold must be between 0 and 255".to_string(),
            });
        }
        for (name, size) in [
            ("close_kernel_size", self.close_kernel_size),
            ("dilate_kernel_size", self.dilate_kernel_size),
        ] {
            if size <= 0 || size % 2 == 0 {
                return Err(FunctionError::Config {
                    message: format!("cross.{name} must be a positive odd number"),
                });
            }
        }
        if self.dilate_iterations < 0 {
            return Err(FunctionError::Config {
                message: "cross.dilate_iterations cannot be negative".to_string(),
            });
        }
        if !self.min_radius.is_finite()
            || !self.max_radius.is_finite()
            || self.min_radius <= 0.0
            || self.max_radius < self.min_radius
        {
            return Err(FunctionError::Config {
                message: "cross radii must be finite, positive, and ordered".to_string(),
            });
        }
        if !self.center_tolerance.is_finite() || self.center_tolerance <= 0.0 {
            return Err(FunctionError::Config {
                message: "cross.center_tolerance must be finite and greater than 0".to_string(),
            });
        }
        if self.min_arc_points < 3 {
            return Err(FunctionError::Config {
                message: "cross.min_arc_points must be at least 3".to_string(),
            });
        }
        if self.min_ring_score > 100 {
            return Err(FunctionError::Config {
                message: "cross.min_ring_score must be at most 100".to_string(),
            });
        }
        Ok(())
    }
}

#[cfg(feature = "opencv")]
struct CrossFrameResult {
    found: bool,
    dx: i32,
    dy: i32,
    score: u8,
    gray: opencv::core::Mat,
    black_mask: opencv::core::Mat,
    frame: opencv::core::Mat,
}

#[cfg(feature = "opencv")]
#[derive(Debug, Clone)]
struct RingCandidate {
    center: opencv::core::Point2f,
    radius: f32,
    coverage: f64,
    residual: f64,
    weight: f64,
}

#[cfg(feature = "opencv")]
struct RingGroup {
    center: opencv::core::Point2f,
    score: u8,
}

#[cfg(feature = "opencv")]
fn analyze_cross_frame(
    frame: &opencv::core::Mat,
    parameters: &CrossParameters,
) -> opencv::Result<CrossFrameResult> {
    use opencv::{core, imgproc, prelude::*};

    let gray = crate::vision::util::bgr_to_gray(frame)?;
    let black_mask = make_black_mask(&gray, parameters)?;
    let candidates = ring_candidates(&black_mask, parameters)?;
    let group = best_ring_group(&candidates, parameters);
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

    let (found, dx, dy, score) = match group {
        Some(group) => {
            let center =
                core::Point::new(group.center.x.round() as i32, group.center.y.round() as i32);
            draw_marker(
                &mut annotated,
                center,
                core::Scalar::new(0.0, 255.0, 0.0, 0.0),
            )?;
            (true, center.x - target.x, center.y - target.y, group.score)
        }
        None => (false, 0, 0, 0),
    };
    let value = format_cross_value(found, dx, dy, score);
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
    Ok(CrossFrameResult {
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
fn make_black_mask(
    gray: &opencv::core::Mat,
    parameters: &CrossParameters,
) -> opencv::Result<opencv::core::Mat> {
    use opencv::{core, imgproc};

    let mut blurred = core::Mat::default();
    imgproc::gaussian_blur(
        gray,
        &mut blurred,
        core::Size::new(5, 5),
        0.0,
        0.0,
        core::BORDER_DEFAULT,
    )?;
    let mut thresholded = core::Mat::default();
    imgproc::threshold(
        &blurred,
        &mut thresholded,
        parameters.black_threshold as f64,
        255.0,
        imgproc::THRESH_BINARY_INV,
    )?;
    let close_kernel = imgproc::get_structuring_element(
        imgproc::MORPH_ELLIPSE,
        core::Size::new(parameters.close_kernel_size, parameters.close_kernel_size),
        core::Point::new(-1, -1),
    )?;
    let mut closed = core::Mat::default();
    imgproc::morphology_ex(
        &thresholded,
        &mut closed,
        imgproc::MORPH_CLOSE,
        &close_kernel,
        core::Point::new(-1, -1),
        1,
        core::BORDER_CONSTANT,
        imgproc::morphology_default_border_value()?,
    )?;
    if parameters.dilate_iterations == 0 {
        return Ok(closed);
    }
    let dilate_kernel = imgproc::get_structuring_element(
        imgproc::MORPH_ELLIPSE,
        core::Size::new(parameters.dilate_kernel_size, parameters.dilate_kernel_size),
        core::Point::new(-1, -1),
    )?;
    let mut dilated = core::Mat::default();
    imgproc::dilate(
        &closed,
        &mut dilated,
        &dilate_kernel,
        core::Point::new(-1, -1),
        parameters.dilate_iterations,
        core::BORDER_CONSTANT,
        imgproc::morphology_default_border_value()?,
    )?;
    Ok(dilated)
}

#[cfg(feature = "opencv")]
fn ring_candidates(
    black_mask: &opencv::core::Mat,
    parameters: &CrossParameters,
) -> opencv::Result<Vec<RingCandidate>> {
    use opencv::{core, imgproc, types};

    let mut contours = types::VectorOfVectorOfPoint::new();
    imgproc::find_contours(
        black_mask,
        &mut contours,
        imgproc::RETR_LIST,
        imgproc::CHAIN_APPROX_NONE,
        core::Point::new(0, 0),
    )?;
    let mut candidates = Vec::new();
    for index in 0..contours.len() {
        let contour = contours.get(index)?;
        if contour.len() < parameters.min_arc_points {
            continue;
        }
        if let Some(candidate) = fit_ring_candidate(&contour, parameters)? {
            candidates.push(candidate);
        }
    }
    Ok(candidates)
}

#[cfg(feature = "opencv")]
fn fit_ring_candidate(
    contour: &opencv::types::VectorOfPoint,
    parameters: &CrossParameters,
) -> opencv::Result<Option<RingCandidate>> {
    use std::f64::consts::TAU;

    let Some((center, radius)) = least_squares_circle(contour)? else {
        return Ok(None);
    };
    if radius < parameters.min_radius || radius > parameters.max_radius {
        return Ok(None);
    }
    let mut angles = Vec::with_capacity(contour.len());
    let mut residual_sum = 0.0;
    for index in 0..contour.len() {
        let point = contour.get(index)?;
        let dx = point.x as f64 - center.x as f64;
        let dy = point.y as f64 - center.y as f64;
        residual_sum += (dx.hypot(dy) - radius).abs();
        angles.push(dy.atan2(dx).rem_euclid(TAU));
    }
    let residual = residual_sum / contour.len() as f64;
    let max_residual = 6.0_f64.max(radius * 0.08);
    if !residual.is_finite() || residual > max_residual {
        return Ok(None);
    }
    angles.sort_by(f64::total_cmp);
    let largest_gap = angles
        .windows(2)
        .map(|pair| pair[1] - pair[0])
        .chain(std::iter::once(angles[0] + TAU - angles[angles.len() - 1]))
        .fold(0.0_f64, f64::max);
    let coverage = TAU - largest_gap;
    if coverage < std::f64::consts::PI / 8.0 {
        return Ok(None);
    }
    let coverage_ratio = coverage / TAU;
    let weight = coverage_ratio * (contour.len() as f64).sqrt() / (1.0 + residual);
    Ok(Some(RingCandidate {
        center,
        radius: radius as f32,
        coverage,
        residual,
        weight,
    }))
}

#[cfg(feature = "opencv")]
fn least_squares_circle(
    contour: &opencv::types::VectorOfPoint,
) -> opencv::Result<Option<(opencv::core::Point2f, f64)>> {
    let mut matrix = [[0.0_f64; 4]; 3];
    for index in 0..contour.len() {
        let point = contour.get(index)?;
        let x = point.x as f64;
        let y = point.y as f64;
        let z = x * x + y * y;
        matrix[0][0] += x * x;
        matrix[0][1] += x * y;
        matrix[0][2] += x;
        matrix[0][3] -= x * z;
        matrix[1][0] += x * y;
        matrix[1][1] += y * y;
        matrix[1][2] += y;
        matrix[1][3] -= y * z;
        matrix[2][0] += x;
        matrix[2][1] += y;
        matrix[2][2] += 1.0;
        matrix[2][3] -= z;
    }
    let Some([a, b, c]) = solve_3x3(matrix) else {
        return Ok(None);
    };
    let center_x = -a / 2.0;
    let center_y = -b / 2.0;
    let radius_squared = center_x * center_x + center_y * center_y - c;
    if !center_x.is_finite()
        || !center_y.is_finite()
        || !radius_squared.is_finite()
        || radius_squared <= 0.0
    {
        return Ok(None);
    }
    Ok(Some((
        opencv::core::Point2f::new(center_x as f32, center_y as f32),
        radius_squared.sqrt(),
    )))
}

#[cfg(feature = "opencv")]
fn solve_3x3(mut matrix: [[f64; 4]; 3]) -> Option<[f64; 3]> {
    for pivot in 0..3 {
        let best_row = (pivot..3).max_by(|&left, &right| {
            matrix[left][pivot]
                .abs()
                .total_cmp(&matrix[right][pivot].abs())
        })?;
        if matrix[best_row][pivot].abs() < 1.0e-9 {
            return None;
        }
        matrix.swap(pivot, best_row);
        let divisor = matrix[pivot][pivot];
        for column in pivot..4 {
            matrix[pivot][column] /= divisor;
        }
        for row in 0..3 {
            if row == pivot {
                continue;
            }
            let factor = matrix[row][pivot];
            for column in pivot..4 {
                matrix[row][column] -= factor * matrix[pivot][column];
            }
        }
    }
    Some([matrix[0][3], matrix[1][3], matrix[2][3]])
}

#[cfg(feature = "opencv")]
fn best_ring_group(
    candidates: &[RingCandidate],
    parameters: &CrossParameters,
) -> Option<RingGroup> {
    candidates
        .iter()
        .filter_map(|seed| score_ring_group(seed, candidates, parameters))
        .max_by_key(|group| group.score)
}

#[cfg(feature = "opencv")]
fn score_ring_group(
    seed: &RingCandidate,
    candidates: &[RingCandidate],
    parameters: &CrossParameters,
) -> Option<RingGroup> {
    use std::f64::consts::TAU;

    let members: Vec<&RingCandidate> = candidates
        .iter()
        .filter(|candidate| {
            point_distance(seed.center, candidate.center) <= parameters.center_tolerance
        })
        .collect();
    if members.len() < 3 {
        return None;
    }
    let total_weight: f64 = members.iter().map(|candidate| candidate.weight).sum();
    if total_weight <= 0.0 {
        return None;
    }
    let center = opencv::core::Point2f::new(
        (members
            .iter()
            .map(|candidate| candidate.center.x as f64 * candidate.weight)
            .sum::<f64>()
            / total_weight) as f32,
        (members
            .iter()
            .map(|candidate| candidate.center.y as f64 * candidate.weight)
            .sum::<f64>()
            / total_weight) as f32,
    );
    let mut radii: Vec<f32> = members.iter().map(|candidate| candidate.radius).collect();
    radii.sort_by(f32::total_cmp);
    let mut distinct_radii = Vec::new();
    for radius in radii {
        if distinct_radii
            .last()
            .is_none_or(|previous: &f32| (radius - *previous) > (4.0_f32).max(*previous * 0.03))
        {
            distinct_radii.push(radius);
        }
    }
    if distinct_radii.len() < 3 {
        return None;
    }
    let total_coverage: f64 = members.iter().map(|candidate| candidate.coverage).sum();
    let average_residual = members
        .iter()
        .map(|candidate| candidate.residual * candidate.weight)
        .sum::<f64>()
        / total_weight;
    let center_spread = members
        .iter()
        .map(|candidate| point_distance(center, candidate.center) * candidate.weight)
        .sum::<f64>()
        / total_weight;
    let radius_score = (distinct_radii.len().min(6) as f64 / 6.0) * 50.0;
    let coverage_score = (total_coverage / (TAU * 4.0)).clamp(0.0, 1.0) * 30.0;
    let residual_score =
        (1.0 - average_residual / parameters.center_tolerance.max(1.0)).clamp(0.0, 1.0) * 10.0;
    let center_score = (1.0 - center_spread / parameters.center_tolerance).clamp(0.0, 1.0) * 10.0;
    let score = (radius_score + coverage_score + residual_score + center_score)
        .round()
        .clamp(0.0, 100.0) as u8;
    (score >= parameters.min_ring_score).then_some(RingGroup { center, score })
}

#[cfg(feature = "opencv")]
fn point_distance(left: opencv::core::Point2f, right: opencv::core::Point2f) -> f64 {
    (left.x as f64 - right.x as f64).hypot(left.y as f64 - right.y as f64)
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
fn format_cross_value(found: bool, dx: i32, dy: i32, score: u8) -> String {
    if found {
        format!("CROSS,0,1,{dx},{dy},{score}")
    } else {
        "CROSS,0,0,0,0,0".to_string()
    }
}

#[cfg(feature = "opencv")]
async fn detect_cross(
    camera: std::sync::Arc<CameraDevice>,
    parameters: &CrossParameters,
) -> Result<CrossFrameResult, FunctionError> {
    let mut best = None;
    for _ in 0..parameters.max_frames {
        let frame = camera.frame().await?;
        let frame_parameters = parameters.clone();
        let result =
            tokio::task::spawn_blocking(move || analyze_cross_frame(&frame, &frame_parameters))
                .await
                .map_err(|error| FunctionError::Call {
                    message: format!("cross task failed: {error}"),
                })?
                .map_err(|error| FunctionError::Call {
                    message: error.to_string(),
                })?;
        if best
            .as_ref()
            .is_none_or(|current: &CrossFrameResult| result.score > current.score)
        {
            best = Some(result);
        }
    }
    best.ok_or_else(|| FunctionError::Call {
        message: "cross received no camera frames".to_string(),
    })
}

#[rubo_engine::function(id = "cross")]
#[derive(Default)]
pub struct CrossDetect;

#[async_trait]
impl Function for CrossDetect {
    async fn call(&self, call: FunctionCall<'_>) -> Result<FuncResult, FunctionError> {
        let parameters = CrossParameters::from_config(call.function_config())?;
        let camera = call.devices().get::<CameraDevice>(&parameters.device_id)?;
        run_cross_detect(camera, &parameters, call.image_enabled()).await
    }
}

#[cfg(not(feature = "opencv"))]
async fn run_cross_detect(
    _camera: std::sync::Arc<CameraDevice>,
    _parameters: &CrossParameters,
    _image_enabled: bool,
) -> Result<FuncResult, FunctionError> {
    Err(FunctionError::Call {
        message: crate::tool::opencv_disabled_message("cross"),
    })
}

#[cfg(feature = "opencv")]
async fn run_cross_detect(
    camera: std::sync::Arc<CameraDevice>,
    parameters: &CrossParameters,
    image_enabled: bool,
) -> Result<FuncResult, FunctionError> {
    let result = detect_cross(camera, parameters).await?;
    let value = format_cross_value(result.found, result.dx, result.dy, result.score);
    let mut output = serde_json::json!({
        "text": format!("cross finished: {value}"),
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
    async fn test_cross() {
        let (config, camera) = crate::vision::test::load_camera("road")
            .await
            .expect("load cross camera");
        let camera = camera.get::<CameraDevice>().expect("get cross camera");
        let parameters =
            CrossParameters::from_config(&config.funcs()["cross"]).expect("load cross config");
        let result = detect_cross(camera, &parameters)
            .await
            .expect("detect cross");
        println!(
            "{}",
            format_cross_value(result.found, result.dx, result.dy, result.score)
        );
    }

    #[tokio::test]
    async fn test_cross_show() {
        let (config, camera) = crate::vision::test::load_camera("road")
            .await
            .expect("load cross camera");
        let camera = camera.get::<CameraDevice>().expect("get cross camera");
        let parameters =
            CrossParameters::from_config(&config.funcs()["cross"]).expect("load cross config");
        loop {
            let frame = camera.frame().await.expect("read cross frame");
            let frame_for_analysis = frame.try_clone().expect("clone cross frame");
            let frame_parameters = parameters.clone();
            let result = tokio::task::spawn_blocking(move || {
                analyze_cross_frame(&frame_for_analysis, &frame_parameters)
            })
            .await
            .expect("join cross analysis")
            .expect("analyze cross frame");
            let value = format_cross_value(result.found, result.dx, result.dy, result.score);
            let display = crate::vision::test::annotate_result(&result.frame, "CROSS", &value)
                .expect("annotate cross result");
            highgui::imshow("cross.original", &frame).expect("show cross original");
            highgui::imshow("cross.gray", &result.gray).expect("show cross gray");
            highgui::imshow("cross.mask", &result.black_mask).expect("show cross mask");
            highgui::imshow("cross.result", &display).expect("show cross result");
            let key = highgui::wait_key(1).expect("wait for cross key") & 0xff;
            if key == 113 || key == 27 {
                break;
            }
        }
    }
}
