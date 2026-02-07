use crate::{config::device_config::ColorCameraConfig, device::camera};
use crate::device::color_detect::color_detect_utils::{roi_circle_mask,hsv_inrange,
hsv_scalar_factory};
use anyhow::Result;
use opencv::{
    core::{self, Mat, Scalar},
    highgui,
    imgproc,
    prelude::*,
};


pub fn work(config: ColorCameraConfig,state:bool) -> Result<()> {

    let mut cam = camera::register_color_camera(config.clone())?;

    loop {
        let mut frame = core::Mat::default();
        cam.read(&mut frame)?;
        if frame.empty() {continue;}
        
        let rate =  0.8;  // TODO: 封装进去config
        let (_roi, circle_mask) = roi_circle_mask(&frame, rate)?;
 
        let (color_name, ratio) = detect_color_in_circle_mask(&frame, &circle_mask, &config)?;


        if state {// TODO: 迟早state也封装进去
            // 画出圆形 ROI
            let size = frame.size()?;
            let w = size.width;
            let h = size.height;
            let cx = w / 2;
            let cy = h / 2;
            let r = ((w.min(h) as f64) * rate) as i32;

            imgproc::circle(
                &mut frame,
                core::Point::new(cx, cy),
                r,
                core::Scalar::new(0.0, 255.0, 0.0, 0.0),
                2,
                imgproc::LINE_8,
                0,
            )?;

            let label = format!("color: {}  ratio: {:.2}", color_name, ratio);
            draw_label(&mut frame, &label, 10, 30)?;

            highgui::imshow("color_detect", &frame)?;
            let key = highgui::wait_key(1)?;
            if key == 113 || key == 27 {
                break; // q / esc
            }
        }
    }

    Ok(())
}

 
//* 计算在ROI区域内，过滤得到的颜色的面积 */
fn color_ratio_in_circle_mask(
    frame_bgr: &Mat,circle_mask: &Mat,hsv_arr: [i32; 6]) -> Result<f64> {

    let (lower, upper) = hsv_scalar_factory(hsv_arr)?;

    let color_mask = hsv_inrange(frame_bgr, &lower, &upper)?;

    let mut inside = Mat::default();
    core::bitwise_and(&color_mask, &color_mask, &mut inside, circle_mask)?;

    let hit = core::count_non_zero(&inside)? as f64;
    let total = core::count_non_zero(circle_mask)? as f64;

    Ok(if total > 0.0 { hit / total } else { 0.0 })
}

fn detect_color_in_circle_mask(
    frame_bgr: &Mat,circle_mask: &Mat,config: &ColorCameraConfig,) 
    -> Result<(String, f64)> {


    let mut best_name = "unknown".to_string();
    let mut best_ratio = 0.0_f64;

    for c in &config.colors {
        let (name, hsv_arr) = match c.as_str() {
            "red" => ("red", config.hsv_red),
            "blue" => ("blue", config.hsv_blue),
            "green" => ("green", config.hsv_green),
            "black" => ("black", config.hsv_black),
            "white" => ("white", config.hsv_white),
            _ => continue,
        };

        let ratio = color_ratio_in_circle_mask(frame_bgr,circle_mask,hsv_arr)?;

        if ratio > best_ratio {
            best_ratio = ratio;
            best_name = name.to_string();
        }
    }

    if best_ratio >= 0.80 {
        Ok((best_name, best_ratio))
    } else {
        Ok(("unknown".to_string(), best_ratio))
    }
}



fn draw_label(frame: &mut Mat, text: &str, x: i32, y: i32) -> Result<()> {
    imgproc::put_text(
        frame,
        text,
        core::Point::new(x, y),
        imgproc::FONT_HERSHEY_SIMPLEX,
        0.8,
        Scalar::new(255.0, 255.0, 255.0, 0.0), // 白字
        2,
        imgproc::LINE_AA,
        false,
    )?;
    Ok(())
}
