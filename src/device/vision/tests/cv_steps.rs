use anyhow::Result;
use opencv::{core, highgui, prelude::*};

use crate::utils::cv_util::{hsv_inrange, roi_circle_mask};

use super::super::{analyze_black_ring_frame, color::analyze_color_frame};
use super::support::{
    black_ring_detect_config_from_config, color_detect_config_from_config, draw_label, open_camera,
    read_non_empty_frame,
};

#[test]
#[ignore = "requires color camera and GUI; shows frame, ROI, circle mask, HSV masks and results"]
fn show_color_detect_cv_steps_from_config() -> Result<()> {
    let config = color_detect_config_from_config()?;
    let mut cam = open_camera(&config.path)?;

    highgui::named_window("color/frame", highgui::WINDOW_NORMAL)?;
    highgui::named_window("color/roi", highgui::WINDOW_NORMAL)?;
    highgui::named_window("color/circle_mask", highgui::WINDOW_NORMAL)?;
    for color in &config.color_ranges {
        highgui::named_window(
            &format!("color/{}/hsv_mask", color.name),
            highgui::WINDOW_NORMAL,
        )?;
        highgui::named_window(
            &format!("color/{}/mask_in_circle", color.name),
            highgui::WINDOW_NORMAL,
        )?;
        highgui::named_window(
            &format!("color/{}/result", color.name),
            highgui::WINDOW_NORMAL,
        )?;
    }

    loop {
        let mut frame = read_non_empty_frame(&mut cam)?;
        let analysis = analyze_color_frame(&frame, &config)?;
        draw_label(
            &mut frame,
            &format!("best={} ratio={:.3}", analysis.color_name, analysis.ratio),
            10,
            30,
        )?;

        highgui::imshow("color/frame", &frame)?;
        highgui::imshow("color/roi", &analysis.roi)?;
        highgui::imshow("color/circle_mask", &analysis.circle_mask)?;
        for step in &analysis.steps {
            highgui::imshow(&format!("color/{}/hsv_mask", step.name), &step.hsv_mask)?;
            highgui::imshow(
                &format!("color/{}/mask_in_circle", step.name),
                &step.mask_in_circle,
            )?;
            highgui::imshow(&format!("color/{}/result", step.name), &step.result)?;
        }

        let key = highgui::wait_key(1)?;
        if key == 113 || key == 27 {
            break;
        }
    }

    Ok(())
}

#[test]
#[ignore = "requires USB camera and GUI; press q or ESC to print the last HSV values"]
fn tune_hsv_thresholds_from_config() -> Result<()> {
    let config = color_detect_config_from_config()?;
    let mut cam = open_camera(&config.path)?;

    highgui::named_window("hsv/controls", highgui::WINDOW_AUTOSIZE)?;
    highgui::named_window("hsv/frame", highgui::WINDOW_NORMAL)?;
    highgui::named_window("hsv/roi", highgui::WINDOW_NORMAL)?;
    highgui::named_window("hsv/mask", highgui::WINDOW_NORMAL)?;
    highgui::named_window("hsv/result", highgui::WINDOW_NORMAL)?;

    let mut h_min = 0;
    let mut h_max = 179;
    let mut s_min = 0;
    let mut s_max = 255;
    let mut v_min = 0;
    let mut v_max = 255;

    highgui::create_trackbar("H min", "hsv/controls", Some(&mut h_min), 179, None)?;
    highgui::create_trackbar("H max", "hsv/controls", Some(&mut h_max), 179, None)?;
    highgui::create_trackbar("S min", "hsv/controls", Some(&mut s_min), 255, None)?;
    highgui::create_trackbar("S max", "hsv/controls", Some(&mut s_max), 255, None)?;
    highgui::create_trackbar("V min", "hsv/controls", Some(&mut v_min), 255, None)?;
    highgui::create_trackbar("V max", "hsv/controls", Some(&mut v_max), 255, None)?;

    loop {
        let frame = read_non_empty_frame(&mut cam)?;
        let (mut roi, circle_mask) = roi_circle_mask(&frame, config.radius_ratio)?;

        let hmin = highgui::get_trackbar_pos("H min", "hsv/controls")?;
        let hmax = highgui::get_trackbar_pos("H max", "hsv/controls")?;
        let smin = highgui::get_trackbar_pos("S min", "hsv/controls")?;
        let smax = highgui::get_trackbar_pos("S max", "hsv/controls")?;
        let vmin = highgui::get_trackbar_pos("V min", "hsv/controls")?;
        let vmax = highgui::get_trackbar_pos("V max", "hsv/controls")?;

        let (h1, h2) = ordered_pair(hmin, hmax);
        let (s1, s2) = ordered_pair(smin, smax);
        let (v1, v2) = ordered_pair(vmin, vmax);
        let current_hsv = [h1, h2, s1, s2, v1, v2];

        let lower = core::Scalar::new(h1 as f64, s1 as f64, v1 as f64, 0.0);
        let upper = core::Scalar::new(h2 as f64, s2 as f64, v2 as f64, 0.0);
        let mask = hsv_inrange(&roi, &lower, &upper)?;

        let mut mask_in_circle = Mat::default();
        core::bitwise_and(&mask, &mask, &mut mask_in_circle, &circle_mask)?;

        let mut result = Mat::default();
        core::bitwise_and(&roi, &roi, &mut result, &mask_in_circle)?;

        draw_hsv_label(&mut roi, current_hsv)?;
        highgui::imshow("hsv/frame", &frame)?;
        highgui::imshow("hsv/roi", &roi)?;
        highgui::imshow("hsv/mask", &mask_in_circle)?;
        highgui::imshow("hsv/result", &result)?;

        let key = highgui::wait_key(1)?;
        if key == 113 || key == 27 {
            println!(
                "{{ name = \"new_color\", hsv = [{}, {}, {}, {}, {}, {}] }}",
                current_hsv[0],
                current_hsv[1],
                current_hsv[2],
                current_hsv[3],
                current_hsv[4],
                current_hsv[5]
            );
            break;
        }
    }

    Ok(())
}

#[test]
#[ignore = "requires camera and GUI; shows black ring steps and prints target_correction on q/ESC"]
fn find_black_ring_target_correction_from_config() -> Result<()> {
    let config = black_ring_detect_config_from_config()?;
    let mut cam = open_camera(&config.path)?;
    let mut last_recommendation = None;

    highgui::named_window("black_ring/frame", highgui::WINDOW_NORMAL)?;
    highgui::named_window("black_ring/gray", highgui::WINDOW_NORMAL)?;
    highgui::named_window("black_ring/black_mask", highgui::WINDOW_NORMAL)?;
    highgui::named_window("black_ring/annotated", highgui::WINDOW_NORMAL)?;

    loop {
        let frame = read_non_empty_frame(&mut cam)?;
        let size = frame.size()?;
        let analysis = analyze_black_ring_frame(&frame, &config)?;

        if let Some(center) = analysis.result.center {
            last_recommendation = Some((
                center.x.round() as i32 - size.width / 2,
                center.y.round() as i32 - size.height / 2,
            ));
        }

        highgui::imshow("black_ring/frame", &frame)?;
        highgui::imshow("black_ring/gray", &analysis.gray)?;
        highgui::imshow("black_ring/black_mask", &analysis.black_mask)?;
        highgui::imshow("black_ring/annotated", &analysis.annotated)?;

        let key = highgui::wait_key(1)?;
        if key == 113 || key == 27 {
            if let Some((x, y)) = last_recommendation {
                println!("target_correction = {{ x = {x}, y = {y} }}");
            } else {
                println!("black ring not detected; target_correction unavailable");
            }
            break;
        }
    }

    Ok(())
}

fn ordered_pair(a: i32, b: i32) -> (i32, i32) {
    if a <= b { (a, b) } else { (b, a) }
}

fn draw_hsv_label(frame: &mut Mat, hsv: [i32; 6]) -> Result<()> {
    draw_label(
        frame,
        &format!(
            "HSV [{},{}] [{},{}] [{},{}]",
            hsv[0], hsv[1], hsv[2], hsv[3], hsv[4], hsv[5]
        ),
        10,
        30,
    )
}
