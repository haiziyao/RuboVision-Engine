#![cfg(test)]
#![allow(unused_imports)]


use anyhow::{Result, bail};
use opencv::{core, highgui, imgproc, prelude::*, videoio};
use crate::device::color_detect::color_detect_utils::{roi_circle_mask,hsv_inrange};


#[test]
fn test_hsv() -> Result<()> {
    // ===== 读配置 =====
    let config = crate::utils::device_config_util::get_config()?;
    let config = config.color_camera_config;

    // ===== 打开窗口 =====
    highgui::named_window("controls", highgui::WINDOW_AUTOSIZE)?;
    highgui::named_window("frame", highgui::WINDOW_NORMAL)?;
    highgui::named_window("roi", highgui::WINDOW_NORMAL)?;
    highgui::named_window("mask", highgui::WINDOW_NORMAL)?;
    highgui::named_window("result", highgui::WINDOW_NORMAL)?;

    // ===== 6 个滑动条 =====
    let mut h_min: i32 = 0;
    let mut s_min: i32 = 0;
    let mut v_min: i32 = 0;

    let mut h_max: i32 = 179;
    let mut s_max: i32 = 255;
    let mut v_max: i32 = 255;

    highgui::create_trackbar("H min", "controls", Some(&mut h_min), 179, None)?;
    highgui::create_trackbar("H max", "controls", Some(&mut h_max), 179, None)?;
    highgui::create_trackbar("S min", "controls", Some(&mut s_min), 255, None)?;
    highgui::create_trackbar("S max", "controls", Some(&mut s_max), 255, None)?;
    highgui::create_trackbar("V min", "controls", Some(&mut v_min), 255, None)?;
    highgui::create_trackbar("V max", "controls", Some(&mut v_max), 255, None)?;

    // ===== 打开相机 =====
    // 你有 register_color_camera 就用它（推荐）
    let mut cam = crate::device::camera::register_color_camera(config.clone())?;
    if !videoio::VideoCapture::is_opened(&cam)? {
        bail!("无法打开相机：{}", config.color_camera);
    }

    let mut frame = Mat::default();
    let mut result = Mat::default();

    // 你要只关注屏幕中间圆
    let radius_ratio: f64 = 0.40; // TODO 你自己调：0.35~0.45 常用

    loop {
        cam.read(&mut frame)?;
        if frame.empty() {
            continue;
        }

        // ===== ROI：只保留中间圆（圆外清零）+ circle_mask（圆内255/圆外0）=====
        let (roi, circle_mask) =   roi_circle_mask(&frame, radius_ratio)?;
        // ↑ 如果 roi_circle_mask 就在当前模块，直接 roi_circle_mask(&frame, radius_ratio)? 就行

        // ===== 取最新滑动条值 =====
        let hmin = highgui::get_trackbar_pos("H min", "controls")?;
        let hmax = highgui::get_trackbar_pos("H max", "controls")?;
        let smin = highgui::get_trackbar_pos("S min", "controls")?;
        let smax = highgui::get_trackbar_pos("S max", "controls")?;
        let vmin = highgui::get_trackbar_pos("V min", "controls")?;
        let vmax = highgui::get_trackbar_pos("V max", "controls")?;

        // 防呆：min/max 反了就交换
        let (h1, h2) = if hmin <= hmax { (hmin, hmax) } else { (hmax, hmin) };
        let (s1, s2) = if smin <= smax { (smin, smax) } else { (smax, smin) };
        let (v1, v2) = if vmin <= vmax { (vmin, vmax) } else { (vmax, vmin) };

        let lower = core::Scalar::new(h1 as f64, s1 as f64, v1 as f64, 0.0);
        let upper = core::Scalar::new(h2 as f64, s2 as f64, v2 as f64, 0.0);

        // ===== HSV inRange（你写的函数返回 1 通道 mask）=====
        let mut mask =  hsv_inrange(&roi, &lower, &upper)?;
        // ↑ 同上：如果在当前模块，直接 hsv_inrange(&roi, &lower, &upper)? 就行

        // ===== 关键：强制只保留圆内，避免圆外清零(黑)干扰“黑色检测”=====
        let mut mask_in_circle = Mat::default();
        core::bitwise_and(&mask, &mask, &mut mask_in_circle, &circle_mask)?;

        // ===== 可选：额外处理（按你项目需要开/闭/模糊）=====
        // 例：开运算去小白噪点
        // let k = kernel_factory(5, MyKernelShape::Ellipse)?;
        // let mask_in_circle = morph_open(&mask_in_circle, &k)?;

        // ===== 用 mask 把 ROI 原图抠出来 =====
        core::bitwise_and(&roi, &roi, &mut result, &mask_in_circle)?;

        // ===== 显示 =====
        highgui::imshow("frame", &frame)?;
        highgui::imshow("roi", &roi)?;
        highgui::imshow("mask", &mask_in_circle)?;
        highgui::imshow("result", &result)?;

        let key = highgui::wait_key(1)?;
        if key == 113 || key == 27 { // q / ESC
            break;
        }
    }

    Ok(())
}
