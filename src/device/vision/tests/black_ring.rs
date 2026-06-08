use anyhow::Result;
use opencv::{
    core::{self, Point, Scalar},
    imgproc,
    prelude::*,
};

use super::super::{
    BlackRingDetectConfig, TargetCorrection, analyze_black_ring_frame, format_black_ring_value,
};

fn base_config() -> BlackRingDetectConfig {
    BlackRingDetectConfig {
        path: "/dev/null".to_string(),
        debug_model: false,
        loop_count: 1,
        target_correction: TargetCorrection { x: 20, y: -10 },
        black_threshold: 90,
        min_radius: 20.0,
        max_radius: 140.0,
        min_circularity: 0.65,
        min_score: 50,
    }
}

fn synthetic_frame_with_line_and_ring() -> Result<Mat> {
    let mut frame = Mat::new_rows_cols_with_default(
        480,
        640,
        core::CV_8UC3,
        Scalar::new(230.0, 230.0, 230.0, 0.0),
    )?;

    imgproc::rectangle(
        &mut frame,
        core::Rect::new(120, 0, 55, 480),
        Scalar::new(0.0, 0.0, 0.0, 0.0),
        -1,
        imgproc::LINE_8,
        0,
    )?;
    imgproc::circle(
        &mut frame,
        Point::new(420, 240),
        70,
        Scalar::new(0.0, 0.0, 0.0, 0.0),
        9,
        imgproc::LINE_8,
        0,
    )?;

    Ok(frame)
}

#[test]
fn black_ring_detection_ignores_vertical_black_line() -> Result<()> {
    let frame = synthetic_frame_with_line_and_ring()?;
    let analysis = analyze_black_ring_frame(&frame, &base_config())?;

    assert!(analysis.result.valid);
    let center = analysis.result.center.expect("ring center");
    assert!((center.x - 420.0).abs() <= 3.0, "center.x={}", center.x);
    assert!((center.y - 240.0).abs() <= 3.0, "center.y={}", center.y);
    assert!(
        analysis.result.score >= 50,
        "score={}",
        analysis.result.score
    );
    assert!(core::count_non_zero(&analysis.black_mask)? > 0);
    assert!(!analysis.annotated.empty());
    Ok(())
}

#[test]
fn black_ring_value_applies_target_correction_and_formats_uart_output() -> Result<()> {
    let frame = synthetic_frame_with_line_and_ring()?;
    let analysis = analyze_black_ring_frame(&frame, &base_config())?;

    assert_eq!(analysis.result.dx, 80);
    assert_eq!(analysis.result.dy, 10);
    assert_eq!(
        format_black_ring_value(&analysis.result),
        format!("RING,1,80,10,{}", analysis.result.score)
    );
    Ok(())
}

#[test]
fn black_ring_value_reports_invalid_when_no_candidate() -> Result<()> {
    let frame = Mat::new_rows_cols_with_default(
        480,
        640,
        core::CV_8UC3,
        Scalar::new(230.0, 230.0, 230.0, 0.0),
    )?;
    let analysis = analyze_black_ring_frame(&frame, &base_config())?;

    assert!(!analysis.result.valid);
    assert_eq!(format_black_ring_value(&analysis.result), "RING,0,0,0,0");
    Ok(())
}
