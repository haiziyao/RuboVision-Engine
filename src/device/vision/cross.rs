#![allow(dead_code)]

use std::f64::consts::{PI, TAU};

use anyhow::Result;
use opencv::{
    core::{self, Mat, Point, Point2f, Scalar, Size},
    imgproc,
    prelude::*,
    types,
};

use crate::utils::cv_util::bgr_to_gray;

use super::config::CrossDetectConfig;

#[derive(Debug, Clone)]
pub struct CrossResult {
    pub param: u8,
    pub valid: bool,
    pub ring_center: Option<Point2f>,
    pub cylinder_center: Option<Point2f>,
    pub dx: i32,
    pub dy: i32,
    pub score: u8,
}

#[allow(dead_code)]
pub struct CrossFrameAnalysis {
    pub result: CrossResult,
    pub gray: Mat,
    pub black_mask: Mat,
    pub annotated: Mat,
}

#[derive(Debug, Clone)]
struct RingCandidate {
    center: Point2f,
    radius: f32,
    coverage: f64,
    residual: f64,
    weight: f64,
}

#[derive(Debug, Clone)]
struct RingGroup {
    center: Point2f,
    score: u8,
}

pub fn run_cross_detect(config: &CrossDetectConfig) -> Result<String> {
    let _path = &config.path;
    Ok("0".to_string())
}

pub fn analyze_cross_frame(
    frame_bgr: &Mat,
    runtime_param: u8,
    config: &CrossDetectConfig,
) -> Result<CrossFrameAnalysis> {
    let gray = bgr_to_gray(frame_bgr)?;
    let black_mask = black_mask(&gray, config.black_threshold)?;
    let candidates = ring_candidates(&black_mask, config)?;
    let group = best_ring_group(&candidates, config);
    let result = cross_result(group, runtime_param, frame_bgr.size()?, config);
    let annotated = draw_cross_overlay(frame_bgr, &result, config)?;

    Ok(CrossFrameAnalysis {
        result,
        gray,
        black_mask,
        annotated,
    })
}

pub fn format_cross_value(result: &CrossResult) -> String {
    if result.valid {
        format!(
            "CROSS,{},1,{},{},{}",
            result.param, result.dx, result.dy, result.score
        )
    } else {
        format!("CROSS,{},0,0,0,0", result.param)
    }
}

impl CrossResult {
    fn invalid(param: u8) -> Self {
        Self {
            param,
            valid: false,
            ring_center: None,
            cylinder_center: None,
            dx: 0,
            dy: 0,
            score: 0,
        }
    }
}

fn black_mask(gray: &Mat, threshold: i32) -> Result<Mat> {
    let mut blurred = Mat::default();
    imgproc::gaussian_blur(
        gray,
        &mut blurred,
        Size::new(5, 5),
        0.0,
        0.0,
        core::BORDER_DEFAULT,
    )?;

    let mut thresholded = Mat::default();
    imgproc::threshold(
        &blurred,
        &mut thresholded,
        threshold as f64,
        255.0,
        imgproc::THRESH_BINARY_INV,
    )?;

    let kernel = imgproc::get_structuring_element(
        imgproc::MORPH_ELLIPSE,
        Size::new(3, 3),
        Point::new(-1, -1),
    )?;
    let mut closed = Mat::default();
    imgproc::morphology_ex(
        &thresholded,
        &mut closed,
        imgproc::MORPH_CLOSE,
        &kernel,
        Point::new(-1, -1),
        1,
        core::BORDER_CONSTANT,
        Scalar::all(0.0),
    )?;
    let mut cleaned = Mat::default();
    imgproc::morphology_ex(
        &closed,
        &mut cleaned,
        imgproc::MORPH_OPEN,
        &kernel,
        Point::new(-1, -1),
        1,
        core::BORDER_CONSTANT,
        Scalar::all(0.0),
    )?;
    Ok(cleaned)
}

fn ring_candidates(
    black_mask: &Mat,
    config: &CrossDetectConfig,
) -> Result<Vec<RingCandidate>> {
    let mut contours = types::VectorOfVectorOfPoint::new();
    imgproc::find_contours(
        black_mask,
        &mut contours,
        imgproc::RETR_LIST,
        imgproc::CHAIN_APPROX_NONE,
        Point::new(0, 0),
    )?;

    let mut candidates = Vec::new();
    for index in 0..contours.len() {
        let contour = contours.get(index)?;
        if contour.len() < config.min_arc_points {
            continue;
        }
        if let Some(candidate) = fit_ring_candidate(&contour, config)? {
            candidates.push(candidate);
        }
    }
    Ok(candidates)
}

fn fit_ring_candidate(
    contour: &types::VectorOfPoint,
    config: &CrossDetectConfig,
) -> Result<Option<RingCandidate>> {
    let Some((center, radius)) = least_squares_circle(contour)? else {
        return Ok(None);
    };
    if radius < config.min_radius || radius > config.max_radius {
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
        .chain(std::iter::once(
            angles[0] + TAU - angles[angles.len() - 1],
        ))
        .fold(0.0_f64, f64::max);
    let coverage = TAU - largest_gap;
    if coverage < PI / 8.0 {
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

fn least_squares_circle(
    contour: &types::VectorOfPoint,
) -> Result<Option<(Point2f, f64)>> {
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
        Point2f::new(center_x as f32, center_y as f32),
        radius_squared.sqrt(),
    )))
}

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

fn best_ring_group(
    candidates: &[RingCandidate],
    config: &CrossDetectConfig,
) -> Option<RingGroup> {
    candidates
        .iter()
        .filter_map(|seed| score_ring_group(seed, candidates, config))
        .max_by_key(|group| group.score)
}

fn score_ring_group(
    seed: &RingCandidate,
    candidates: &[RingCandidate],
    config: &CrossDetectConfig,
) -> Option<RingGroup> {
    let members: Vec<&RingCandidate> = candidates
        .iter()
        .filter(|candidate| point_distance(seed.center, candidate.center) <= config.center_tolerance)
        .collect();
    if members.len() < 3 {
        return None;
    }

    let total_weight: f64 = members.iter().map(|candidate| candidate.weight).sum();
    if total_weight <= 0.0 {
        return None;
    }
    let center = Point2f::new(
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
        if distinct_radii.last().is_none_or(|previous: &f32| {
            (radius - *previous) > (4.0_f32).max(*previous * 0.03)
        }) {
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
        (1.0 - average_residual / config.center_tolerance.max(1.0)).clamp(0.0, 1.0) * 10.0;
    let center_score =
        (1.0 - center_spread / config.center_tolerance).clamp(0.0, 1.0) * 10.0;
    let score = (radius_score + coverage_score + residual_score + center_score)
        .round()
        .clamp(0.0, 100.0) as u8;
    (score >= config.min_ring_score).then_some(RingGroup { center, score })
}

fn point_distance(left: Point2f, right: Point2f) -> f64 {
    (left.x as f64 - right.x as f64).hypot(left.y as f64 - right.y as f64)
}

fn cross_result(
    group: Option<RingGroup>,
    runtime_param: u8,
    size: Size,
    config: &CrossDetectConfig,
) -> CrossResult {
    let Some(group) = group else {
        return CrossResult::invalid(runtime_param);
    };
    if runtime_param != 0 {
        return CrossResult {
            ring_center: Some(group.center),
            ..CrossResult::invalid(runtime_param)
        };
    }

    let target_x = size.width / 2 + config.target_correction.x;
    let target_y = size.height / 2 + config.target_correction.y;
    CrossResult {
        param: runtime_param,
        valid: true,
        ring_center: Some(group.center),
        cylinder_center: None,
        dx: group.center.x.round() as i32 - target_x,
        dy: group.center.y.round() as i32 - target_y,
        score: group.score,
    }
}

fn draw_cross_overlay(
    frame_bgr: &Mat,
    result: &CrossResult,
    config: &CrossDetectConfig,
) -> Result<Mat> {
    let mut annotated = frame_bgr.clone();
    let size = annotated.size()?;
    let target = Point::new(
        size.width / 2 + config.target_correction.x,
        size.height / 2 + config.target_correction.y,
    );
    draw_marker(
        &mut annotated,
        target,
        Scalar::new(255.0, 0.0, 0.0, 0.0),
    )?;
    if let Some(center) = result.ring_center {
        draw_marker(
            &mut annotated,
            Point::new(center.x.round() as i32, center.y.round() as i32),
            Scalar::new(0.0, 255.0, 0.0, 0.0),
        )?;
    }

    imgproc::put_text(
        &mut annotated,
        &format_cross_value(result),
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

fn draw_marker(frame: &mut Mat, center: Point, color: Scalar) -> Result<()> {
    let half = 12;
    imgproc::line(
        frame,
        Point::new(center.x - half, center.y),
        Point::new(center.x + half, center.y),
        color,
        2,
        imgproc::LINE_AA,
        0,
    )?;
    imgproc::line(
        frame,
        Point::new(center.x, center.y - half),
        Point::new(center.x, center.y + half),
        color,
        2,
        imgproc::LINE_AA,
        0,
    )?;
    Ok(())
}
