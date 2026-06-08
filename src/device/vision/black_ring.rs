use std::f64::consts::PI;

use anyhow::Result;
use opencv::{
    core::{self, Mat, Point, Point2f, Scalar},
    highgui, imgproc,
    prelude::*,
    types,
};

use crate::utils::cv_util::bgr_to_gray;

use super::camera::register_black_ring_camera;
use super::config::BlackRingDetectConfig;

#[derive(Debug, Clone)]
pub struct BlackRingResult {
    pub valid: bool,
    pub center: Option<Point2f>,
    pub radius: f32,
    pub dx: i32,
    pub dy: i32,
    pub score: u8,
}

#[allow(dead_code)]
pub struct BlackRingFrameAnalysis {
    pub result: BlackRingResult,
    pub gray: Mat,
    pub black_mask: Mat,
    pub annotated: Mat,
}

pub struct BlackRingDetectOutput {
    pub result: BlackRingResult,
    pub frame: Mat,
}

struct BlackRingCandidate {
    center: Point2f,
    radius: f32,
    score: u8,
}

pub fn run_black_ring_detect_with_frame(
    config: &BlackRingDetectConfig,
) -> Result<BlackRingDetectOutput> {
    let mut cam = register_black_ring_camera(config)?;
    let iterations = config.loop_count.max(1);
    let mut best: Option<BlackRingDetectOutput> = None;
    let mut last_frame = Mat::default();

    for _ in 0..iterations {
        let mut frame = Mat::default();
        cam.read(&mut frame)?;
        if frame.empty() {
            continue;
        }
        last_frame = frame.clone();

        let analysis = analyze_black_ring_frame(&frame, config)?;
        if config.debug_model {
            highgui::imshow("black_ring/annotated", &analysis.annotated)?;
            let key = highgui::wait_key(1)?;
            if key == 113 || key == 27 {
                break;
            }
        }

        if best
            .as_ref()
            .is_none_or(|current| analysis.result.score > current.result.score)
        {
            best = Some(BlackRingDetectOutput {
                result: analysis.result,
                frame: analysis.annotated,
            });
        }
    }

    Ok(best.unwrap_or_else(|| BlackRingDetectOutput {
        result: BlackRingResult::invalid(),
        frame: last_frame,
    }))
}

pub fn analyze_black_ring_frame(
    frame_bgr: &Mat,
    config: &BlackRingDetectConfig,
) -> Result<BlackRingFrameAnalysis> {
    let gray = bgr_to_gray(frame_bgr)?;
    let black_mask = black_threshold_mask(&gray, config.black_threshold)?;
    let candidate = best_ring_candidate(&black_mask, config)?;
    let result = ring_result(candidate, frame_bgr.size()?, config);
    let annotated = draw_black_ring_overlay(frame_bgr, &result, config)?;

    Ok(BlackRingFrameAnalysis {
        result,
        gray,
        black_mask,
        annotated,
    })
}

pub fn format_black_ring_value(result: &BlackRingResult) -> String {
    if result.valid {
        format!("RING,1,{},{},{}", result.dx, result.dy, result.score)
    } else {
        "RING,0,0,0,0".to_string()
    }
}

impl BlackRingResult {
    fn invalid() -> Self {
        Self {
            valid: false,
            center: None,
            radius: 0.0,
            dx: 0,
            dy: 0,
            score: 0,
        }
    }
}

fn black_threshold_mask(gray: &Mat, black_threshold: i32) -> Result<Mat> {
    let mut mask = Mat::default();
    imgproc::threshold(
        gray,
        &mut mask,
        black_threshold as f64,
        255.0,
        imgproc::THRESH_BINARY_INV,
    )?;
    Ok(mask)
}

fn best_ring_candidate(
    black_mask: &Mat,
    config: &BlackRingDetectConfig,
) -> Result<Option<BlackRingCandidate>> {
    let mut contours = types::VectorOfVectorOfPoint::new();
    imgproc::find_contours(
        black_mask,
        &mut contours,
        imgproc::RETR_EXTERNAL,
        imgproc::CHAIN_APPROX_SIMPLE,
        Point::new(0, 0),
    )?;

    let mut best: Option<BlackRingCandidate> = None;
    for index in 0..contours.len() {
        let contour = contours.get(index)?;
        let Some(candidate) = score_contour(&contour, config)? else {
            continue;
        };
        if best
            .as_ref()
            .is_none_or(|current| candidate.score > current.score)
        {
            best = Some(candidate);
        }
    }

    Ok(best)
}

fn score_contour(
    contour: &types::VectorOfPoint,
    config: &BlackRingDetectConfig,
) -> Result<Option<BlackRingCandidate>> {
    if contour.len() < 5 {
        return Ok(None);
    }

    let area = imgproc::contour_area(contour, false)?;
    let perimeter = imgproc::arc_length(contour, true)?;
    if area <= 0.0 || perimeter <= 0.0 {
        return Ok(None);
    }

    let rect = imgproc::bounding_rect(contour)?;
    if rect.width <= 0 || rect.height <= 0 {
        return Ok(None);
    }
    let aspect = rect.width as f64 / rect.height as f64;
    if !(0.65..=1.45).contains(&aspect) {
        return Ok(None);
    }

    let mut center = Point2f::default();
    let mut radius = 0.0_f32;
    imgproc::min_enclosing_circle(contour, &mut center, &mut radius)?;
    let radius_f64 = radius as f64;
    if radius_f64 < config.min_radius || radius_f64 > config.max_radius {
        return Ok(None);
    }

    let circularity = (4.0 * PI * area / (perimeter * perimeter)).clamp(0.0, 1.0);
    if circularity < config.min_circularity {
        return Ok(None);
    }

    let score = (circularity * 100.0).round().clamp(0.0, 100.0) as u8;
    if score < config.min_score {
        return Ok(None);
    }

    Ok(Some(BlackRingCandidate {
        center,
        radius,
        score,
    }))
}

fn ring_result(
    candidate: Option<BlackRingCandidate>,
    size: core::Size,
    config: &BlackRingDetectConfig,
) -> BlackRingResult {
    let Some(candidate) = candidate else {
        return BlackRingResult::invalid();
    };

    let target_x = size.width / 2 + config.target_correction.x;
    let target_y = size.height / 2 + config.target_correction.y;
    let dx = candidate.center.x.round() as i32 - target_x;
    let dy = candidate.center.y.round() as i32 - target_y;

    BlackRingResult {
        valid: true,
        center: Some(candidate.center),
        radius: candidate.radius,
        dx,
        dy,
        score: candidate.score,
    }
}

fn draw_black_ring_overlay(
    frame_bgr: &Mat,
    result: &BlackRingResult,
    config: &BlackRingDetectConfig,
) -> Result<Mat> {
    let mut annotated = frame_bgr.clone();
    let size = annotated.size()?;
    let target = Point::new(
        size.width / 2 + config.target_correction.x,
        size.height / 2 + config.target_correction.y,
    );

    draw_cross(&mut annotated, target, Scalar::new(255.0, 0.0, 0.0, 0.0))?;
    if let Some(center) = result.center {
        let center_point = Point::new(center.x.round() as i32, center.y.round() as i32);
        imgproc::circle(
            &mut annotated,
            center_point,
            result.radius.round() as i32,
            Scalar::new(0.0, 255.0, 0.0, 0.0),
            2,
            imgproc::LINE_AA,
            0,
        )?;
        draw_cross(
            &mut annotated,
            center_point,
            Scalar::new(0.0, 255.0, 0.0, 0.0),
        )?;
    }

    let label = format_black_ring_value(result);
    imgproc::put_text(
        &mut annotated,
        &label,
        Point::new(10, 30),
        imgproc::FONT_HERSHEY_SIMPLEX,
        0.8,
        Scalar::new(255.0, 255.0, 255.0, 0.0),
        2,
        imgproc::LINE_AA,
        false,
    )?;
    Ok(annotated)
}

fn draw_cross(frame: &mut Mat, center: Point, color: Scalar) -> Result<()> {
    let len = 12;
    imgproc::line(
        frame,
        Point::new(center.x - len, center.y),
        Point::new(center.x + len, center.y),
        color,
        2,
        imgproc::LINE_AA,
        0,
    )?;
    imgproc::line(
        frame,
        Point::new(center.x, center.y - len),
        Point::new(center.x, center.y + len),
        color,
        2,
        imgproc::LINE_AA,
        0,
    )?;
    Ok(())
}
