use anyhow::Result;
use opencv::{
    core::{self, Mat, Scalar},
    highgui, imgproc,
    prelude::*,
};

use crate::utils::cv_util::{hsv_inrange, hsv_scalar_factory, roi_circle_mask};

use super::camera::register_color_camera;
use super::config::{ColorDetectConfig, ColorRange};

#[allow(dead_code)]
pub(super) struct ColorFrameAnalysis {
    pub(super) color_name: String,
    pub(super) ratio: f64,
    pub(super) roi: Mat,
    pub(super) circle_mask: Mat,
    pub(super) steps: Vec<ColorRangeAnalysis>,
}

#[allow(dead_code)]
pub(super) struct ColorRangeAnalysis {
    pub(super) name: String,
    pub(super) ratio: f64,
    pub(super) hsv_mask: Mat,
    pub(super) mask_in_circle: Mat,
    pub(super) result: Mat,
}

pub fn run_color_detect(config: &ColorDetectConfig) -> Result<String> {
    let mut cam = register_color_camera(config)?;
    let mut best_color = String::new();
    let mut count = 0;
    let stable_count = config.loop_count;

    loop {
        let mut frame = core::Mat::default();
        cam.read(&mut frame)?;
        if frame.empty() {
            continue;
        }

        let (color_name, ratio) = detect_color_in_circle_mask(&frame, config)?;

        if config.debug_model {
            draw_debug_info(&mut frame, &color_name, ratio, config.radius_ratio)?;
            let key = highgui::wait_key(1)?;
            if key == 113 || key == 27 {
                break;
            }
            continue;
        }

        if count == 0 {
            best_color = color_name;
            count = 1;
        } else if best_color == color_name {
            count += 1;
        } else {
            best_color.clear();
            count = 0;
        }

        if count >= stable_count {
            return Ok(best_color);
        }
    }

    Ok(String::new())
}

pub(super) fn analyze_color_frame(
    frame_bgr: &Mat,
    config: &ColorDetectConfig,
) -> Result<ColorFrameAnalysis> {
    let (roi, circle_mask) = roi_circle_mask(frame_bgr, config.radius_ratio)?;
    let mut best_name = "unknown".to_string();
    let mut best_ratio = 0.0_f64;
    let mut steps = Vec::with_capacity(config.color_ranges.len());

    for color in &config.color_ranges {
        let step = analyze_color_range(frame_bgr, &circle_mask, color)?;

        if step.ratio > best_ratio {
            best_ratio = step.ratio;
            best_name = step.name.clone();
        }
        steps.push(step);
    }

    let color_name = if best_ratio >= config.detect_area_access_rate {
        best_name
    } else {
        "unknown".to_string()
    };

    Ok(ColorFrameAnalysis {
        color_name,
        ratio: best_ratio,
        roi,
        circle_mask,
        steps,
    })
}

fn analyze_color_range(
    frame_bgr: &Mat,
    circle_mask: &Mat,
    color: &ColorRange,
) -> Result<ColorRangeAnalysis> {
    let (lower, upper) = hsv_scalar_factory(color.hsv)?;
    let hsv_mask = hsv_inrange(frame_bgr, &lower, &upper)?;

    let mut mask_in_circle = Mat::default();
    core::bitwise_and(&hsv_mask, &hsv_mask, &mut mask_in_circle, circle_mask)?;

    let mut result = Mat::default();
    core::bitwise_and(frame_bgr, frame_bgr, &mut result, &mask_in_circle)?;

    let hit = core::count_non_zero(&mask_in_circle)? as f64;
    let total = core::count_non_zero(circle_mask)? as f64;
    let ratio = if total > 0.0 { hit / total } else { 0.0 };

    Ok(ColorRangeAnalysis {
        name: color.name.clone(),
        ratio,
        hsv_mask,
        mask_in_circle,
        result,
    })
}

fn detect_color_in_circle_mask(
    frame_bgr: &Mat,
    config: &ColorDetectConfig,
) -> Result<(String, f64)> {
    let analysis = analyze_color_frame(frame_bgr, config)?;
    Ok((analysis.color_name, analysis.ratio))
}

fn draw_label(frame: &mut Mat, text: &str, x: i32, y: i32) -> Result<()> {
    imgproc::put_text(
        frame,
        text,
        core::Point::new(x, y),
        imgproc::FONT_HERSHEY_SIMPLEX,
        0.8,
        Scalar::new(255.0, 255.0, 255.0, 0.0),
        2,
        imgproc::LINE_AA,
        false,
    )?;
    Ok(())
}

fn draw_debug_info(frame: &mut Mat, color_name: &str, ratio: f64, radius_ratio: f64) -> Result<()> {
    let size = frame.size()?;
    let w = size.width;
    let h = size.height;
    let cx = w / 2;
    let cy = h / 2;
    let r = ((w.min(h) as f64) * radius_ratio) as i32;

    imgproc::circle(
        frame,
        core::Point::new(cx, cy),
        r,
        core::Scalar::new(0.0, 255.0, 0.0, 0.0),
        2,
        imgproc::LINE_8,
        0,
    )?;

    let label = format!("color: {}  ratio: {:.2}", color_name, ratio);
    draw_label(frame, &label, 10, 30)?;
    highgui::imshow("color_detect", frame)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::super::config::ColorRange;
    use super::*;
    use opencv::core;

    #[test]
    fn color_frame_analysis_exposes_roi_hsv_masks_and_result_steps() -> Result<()> {
        let frame = Mat::new_rows_cols_with_default(
            120,
            120,
            core::CV_8UC3,
            Scalar::new(0.0, 0.0, 255.0, 0.0),
        )?;
        let config = ColorDetectConfig {
            path: "/dev/null".to_string(),
            debug_model: false,
            loop_count: 1,
            radius_ratio: 0.35,
            detect_area_access_rate: 0.8,
            color_ranges: vec![ColorRange {
                name: "red".to_string(),
                hsv: [0, 10, 100, 255, 100, 255],
            }],
        };

        let analysis = analyze_color_frame(&frame, &config)?;
        let red_step = analysis
            .steps
            .iter()
            .find(|step| step.name == "red")
            .expect("red HSV step");

        assert_eq!(analysis.color_name, "red");
        assert!(analysis.ratio > 0.99);
        assert!(core::count_non_zero(&analysis.circle_mask)? > 0);
        assert!(!analysis.roi.empty());
        assert!(core::count_non_zero(&red_step.hsv_mask)? > 0);
        assert!(core::count_non_zero(&red_step.mask_in_circle)? > 0);
        assert!(!red_step.result.empty());
        Ok(())
    }
}
