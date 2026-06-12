use anyhow::Result;
use opencv::{
    core::{self, Point, Rect, Scalar},
    imgproc,
    prelude::*,
};

use super::super::{
    CrossColor, CrossDetectConfig, CrossResult, TargetCorrection, analyze_cross_frame,
    format_cross_value,
};

fn base_cross_config() -> CrossDetectConfig {
    CrossDetectConfig {
        path: "/dev/null".to_string(),
        debug_model: false,
        loop_count: 1,
        target_correction: TargetCorrection { x: 0, y: 0 },
        black_threshold: 90,
        min_radius: 20.0,
        max_radius: 260.0,
        center_tolerance: 14.0,
        min_arc_points: 24,
        min_ring_score: 50,
        colors: vec![CrossColor {
            id: 1,
            name: "red".to_string(),
            hsv: [0, 20, 100, 255, 80, 255],
            min_area: 500.0,
            min_circularity: 0.6,
        }],
    }
}

fn synthetic_ring_frame(center: Point, radii: &[i32]) -> Result<Mat> {
    let mut frame = Mat::new_rows_cols_with_default(
        480,
        640,
        core::CV_8UC3,
        Scalar::new(220.0, 220.0, 220.0, 0.0),
    )?;
    for &radius in radii {
        imgproc::circle(
            &mut frame,
            center,
            radius,
            Scalar::new(0.0, 0.0, 0.0, 0.0),
            7,
            imgproc::LINE_AA,
            0,
        )?;
    }
    Ok(frame)
}

#[test]
fn cross_zero_finds_concentric_ring_center_and_target_offset() -> Result<()> {
    let frame = synthetic_ring_frame(Point::new(350, 220), &[45, 80, 120, 165])?;
    let analysis = analyze_cross_frame(&frame, 0, &base_cross_config())?;

    assert!(analysis.result.valid);
    let center = analysis.result.ring_center.expect("ring center");
    assert!((center.x - 350.0).abs() <= 5.0, "center.x={}", center.x);
    assert!((center.y - 220.0).abs() <= 5.0, "center.y={}", center.y);
    assert_eq!(analysis.result.dx, 30);
    assert_eq!(analysis.result.dy, -20);
    assert!(!analysis.annotated.empty());
    Ok(())
}

#[test]
fn cross_zero_uses_visible_arcs_when_center_is_occluded() -> Result<()> {
    let mut frame = synthetic_ring_frame(Point::new(300, 250), &[50, 90, 135, 180])?;
    imgproc::rectangle(
        &mut frame,
        Rect::new(260, 210, 180, 170),
        Scalar::new(220.0, 220.0, 220.0, 0.0),
        -1,
        imgproc::LINE_8,
        0,
    )?;

    let result = analyze_cross_frame(&frame, 0, &base_cross_config())?.result;

    assert!(result.valid);
    let center = result.ring_center.expect("ring center");
    assert!((center.x - 300.0).abs() <= 7.0, "center.x={}", center.x);
    assert!((center.y - 250.0).abs() <= 7.0, "center.y={}", center.y);
    Ok(())
}

#[test]
fn cross_zero_reports_invalid_without_concentric_rings() -> Result<()> {
    let frame = Mat::new_rows_cols_with_default(
        480,
        640,
        core::CV_8UC3,
        Scalar::new(220.0, 220.0, 220.0, 0.0),
    )?;

    let result = analyze_cross_frame(&frame, 0, &base_cross_config())?.result;

    assert!(!result.valid);
    assert!(result.ring_center.is_none());
    Ok(())
}

#[test]
fn cross_value_formats_valid_and_invalid_results() {
    let valid = CrossResult {
        param: 0,
        valid: true,
        ring_center: None,
        cylinder_center: None,
        dx: -42,
        dy: 18,
        score: 91,
    };
    let invalid = CrossResult {
        valid: false,
        ..valid.clone()
    };

    assert_eq!(format_cross_value(&valid), "CROSS,0,1,-42,18,91");
    assert_eq!(format_cross_value(&invalid), "CROSS,0,0,0,0,0");
}
