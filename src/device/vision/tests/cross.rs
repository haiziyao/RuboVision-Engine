use anyhow::Result;
use opencv::{
    core::{self, Point, Rect, Scalar},
    imgproc,
    prelude::*,
};

use super::super::{
    CrossColor, CrossDetectConfig, CrossResult, TargetCorrection, analyze_cross_frame,
    format_cross_value, run_cross_detect_with_frame,
};

fn base_cross_config() -> CrossDetectConfig {
    CrossDetectConfig {
        path: "/dev/null".to_string(),
        debug_model: false,
        loop_count: 1,
        target_correction: TargetCorrection { x: 0, y: 0 },
        black_threshold: 90,
        close_kernel_size: 5,
        dilate_kernel_size: 3,
        dilate_iterations: 1,
        min_radius: 20.0,
        max_radius: 260.0,
        center_tolerance: 14.0,
        min_arc_points: 24,
        min_ring_score: 50,
        colors: vec![
            CrossColor {
                id: 1,
                name: "red".to_string(),
                hsv: [0, 20, 100, 255, 80, 255],
                min_area: 500.0,
                min_circularity: 0.6,
            },
            CrossColor {
                id: 2,
                name: "blue".to_string(),
                hsv: [90, 140, 80, 255, 50, 255],
                min_area: 500.0,
                min_circularity: 0.6,
            },
            CrossColor {
                id: 3,
                name: "green".to_string(),
                hsv: [35, 90, 70, 255, 50, 255],
                min_area: 500.0,
                min_circularity: 0.6,
            },
            CrossColor {
                id: 4,
                name: "black".to_string(),
                hsv: [0, 179, 0, 255, 0, 70],
                min_area: 500.0,
                min_circularity: 0.6,
            },
            CrossColor {
                id: 5,
                name: "white".to_string(),
                hsv: [0, 179, 0, 60, 230, 255],
                min_area: 500.0,
                min_circularity: 0.6,
            },
        ],
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

fn synthetic_broken_thin_ring_frame(center: Point, radii: &[i32]) -> Result<Mat> {
    let mut frame = Mat::new_rows_cols_with_default(
        480,
        640,
        core::CV_8UC3,
        Scalar::new(220.0, 220.0, 220.0, 0.0),
    )?;
    for &radius in radii {
        for &(start, end) in &[(5.0, 86.0), (94.0, 176.0), (184.0, 266.0), (274.0, 355.0)] {
            imgproc::ellipse(
                &mut frame,
                center,
                core::Size::new(radius, radius),
                0.0,
                start,
                end,
                Scalar::new(0.0, 0.0, 0.0, 0.0),
                2,
                imgproc::LINE_AA,
                0,
            )?;
        }
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
fn cross_morphology_reconnects_and_thickens_broken_arcs() -> Result<()> {
    let frame = synthetic_broken_thin_ring_frame(Point::new(330, 230), &[45, 80, 120, 165])?;
    let mut without_dilation = base_cross_config();
    without_dilation.dilate_iterations = 0;
    let without_dilation = analyze_cross_frame(&frame, 0, &without_dilation)?;

    let analysis = analyze_cross_frame(&frame, 0, &base_cross_config())?;
    let thin_foreground = core::count_non_zero(&without_dilation.black_mask)?;
    let thick_foreground = core::count_non_zero(&analysis.black_mask)?;

    assert!(
        thick_foreground > thin_foreground,
        "thin={thin_foreground}, thick={thick_foreground}"
    );
    assert!(analysis.result.valid);
    let center = analysis.result.ring_center.expect("ring center");
    assert!((center.x - 330.0).abs() <= 8.0, "center.x={}", center.x);
    assert!((center.y - 230.0).abs() <= 8.0, "center.y={}", center.y);
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

#[test]
fn cross_one_reports_red_cylinder_offset_from_ring_center() -> Result<()> {
    let mut frame = synthetic_ring_frame(Point::new(320, 240), &[50, 90, 135, 180])?;
    imgproc::circle(
        &mut frame,
        Point::new(390, 260),
        42,
        Scalar::new(0.0, 0.0, 255.0, 0.0),
        -1,
        imgproc::LINE_AA,
        0,
    )?;

    let result = analyze_cross_frame(&frame, 1, &base_cross_config())?.result;

    assert!(result.valid);
    assert!((result.dx - 70).abs() <= 2, "dx={}", result.dx);
    assert!((result.dy - 20).abs() <= 2, "dy={}", result.dy);
    assert!(result.cylinder_center.is_some());
    Ok(())
}

#[test]
fn cross_one_uses_outer_arcs_when_red_cylinder_covers_ring_center() -> Result<()> {
    let mut frame = synthetic_ring_frame(Point::new(320, 240), &[50, 90, 135, 180])?;
    imgproc::circle(
        &mut frame,
        Point::new(365, 265),
        72,
        Scalar::new(0.0, 0.0, 255.0, 0.0),
        -1,
        imgproc::LINE_AA,
        0,
    )?;

    let result = analyze_cross_frame(&frame, 1, &base_cross_config())?.result;

    assert!(result.valid);
    let ring = result.ring_center.expect("ring center");
    assert!((ring.x - 320.0).abs() <= 7.0, "ring.x={}", ring.x);
    assert!((ring.y - 240.0).abs() <= 7.0, "ring.y={}", ring.y);
    assert!((result.dx - 45).abs() <= 7, "dx={}", result.dx);
    assert!((result.dy - 25).abs() <= 7, "dy={}", result.dy);
    Ok(())
}

#[test]
fn cross_color_mode_reports_invalid_when_requested_color_is_absent() -> Result<()> {
    let mut frame = synthetic_ring_frame(Point::new(320, 240), &[50, 90, 135, 180])?;
    imgproc::circle(
        &mut frame,
        Point::new(390, 260),
        42,
        Scalar::new(0.0, 0.0, 255.0, 0.0),
        -1,
        imgproc::LINE_AA,
        0,
    )?;

    let result = analyze_cross_frame(&frame, 2, &base_cross_config())?.result;

    assert!(!result.valid);
    assert!(result.ring_center.is_some());
    assert!(result.cylinder_center.is_none());
    Ok(())
}

#[test]
fn cross_four_distinguishes_black_cylinder_from_thin_rings() -> Result<()> {
    let mut frame = synthetic_ring_frame(Point::new(320, 240), &[50, 90, 135, 180])?;
    imgproc::circle(
        &mut frame,
        Point::new(380, 265),
        40,
        Scalar::new(0.0, 0.0, 0.0, 0.0),
        -1,
        imgproc::LINE_AA,
        0,
    )?;

    let result = analyze_cross_frame(&frame, 4, &base_cross_config())?.result;

    assert!(result.valid);
    let center = result.cylinder_center.expect("black cylinder center");
    assert!((center.x - 380.0).abs() <= 5.0, "center.x={}", center.x);
    assert!((center.y - 265.0).abs() <= 5.0, "center.y={}", center.y);
    Ok(())
}

#[test]
fn cross_five_separates_white_cylinder_from_gray_background() -> Result<()> {
    let mut frame = synthetic_ring_frame(Point::new(320, 240), &[50, 90, 135, 180])?;
    imgproc::circle(
        &mut frame,
        Point::new(285, 275),
        38,
        Scalar::new(255.0, 255.0, 255.0, 0.0),
        -1,
        imgproc::LINE_AA,
        0,
    )?;

    let result = analyze_cross_frame(&frame, 5, &base_cross_config())?.result;

    assert!(result.valid);
    let center = result.cylinder_center.expect("white cylinder center");
    assert!((center.x - 285.0).abs() <= 3.0, "center.x={}", center.x);
    assert!((center.y - 275.0).abs() <= 3.0, "center.y={}", center.y);
    Ok(())
}

#[test]
fn cross_rejects_invalid_runtime_param_before_opening_camera() {
    let error = run_cross_detect_with_frame(6, &base_cross_config())
        .err()
        .expect("invalid runtime param");

    assert!(error.to_string().contains("runtime_param"));
}
