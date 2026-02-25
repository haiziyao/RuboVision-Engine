#![allow(dead_code)]
/// Rust中的OpenCV的api封装的不够流畅
/// 在代码文件中写起来很费力气
/// 这里进行一次封装
/// 注意这里知识简单化封装

use anyhow::{Ok, Result};
use opencv::{
    core,
    imgproc,
    prelude::*,
};



#[derive(Copy, Clone, Debug)]
pub enum MyKernelShape {
    Rect,  //方形，容易把圆变方
    Ellipse,  //椭圆/圆，最适合检测圆、圆柱顶面
    Cross,  //十字形，很少用
}

#[allow(dead_code)]
//*一般kernel是奇数，自己注意一下   记得用枚举类kernel核的形状 */    
fn kernel_factory(ksize: i32,shape:MyKernelShape) -> Result<Mat> { //TODO偷懒了，健壮性不足
    let shape = match shape {
        MyKernelShape::Rect => imgproc::MORPH_RECT,
        MyKernelShape::Ellipse => imgproc::MORPH_ELLIPSE,
        MyKernelShape::Cross => imgproc::MORPH_CROSS,
    };
    Ok(imgproc::get_structuring_element(
        shape,    
        core::Size::new(ksize, ksize),
        core::Point::new(-1, -1),
    )?)
}

pub fn to_bgr_out(original:&Mat) -> Result<Mat>{
    let mut out = Mat::default();
    imgproc::cvt_color(&original, &mut out, imgproc::COLOR_GRAY2BGR, 0)?;
    Ok(out)
}


//*解释imgproc操作的参数  最后一个参数表示通道数 写0表示让opencv自己决定*/
pub fn bgr_to_gray(bgr:&Mat) -> Result<Mat>{
    let mut gray = Mat::default();
    imgproc::cvt_color(bgr, &mut gray, imgproc::COLOR_BGR2GRAY, 0)?;
    Ok(gray)
}

//*  返回值是一个类似二值化后的Mat*/
pub fn bgr_inrange(bgr: &Mat,lower:&core::Scalar,upper:&core::Scalar) -> Result<Mat> {
    let mut mask = Mat::default();
    core::in_range(bgr, lower, upper, &mut mask)?;
    Ok(mask)
}


pub fn hsv_scalar_factory(hsv: [i32; 6]) -> Result<(core::Scalar, core::Scalar)> {
    let lower = core::Scalar::new(
        hsv[0] as f64, // H min
        hsv[2] as f64, // S min
        hsv[4] as f64, // V min
        0.0,
    );
    let upper = core::Scalar::new(
        hsv[1] as f64, // H max
        hsv[3] as f64, // S max
        hsv[5] as f64, // V max
        0.0,
    );
    Ok((lower, upper))
}

//*  返回值是一个mask*/
pub fn hsv_inrange(bgr: &Mat,lower:&core::Scalar,upper:&core::Scalar) -> Result<Mat> {
    let mut hsv = Mat::default();
    imgproc::cvt_color(bgr, &mut hsv, imgproc::COLOR_BGR2HSV, 0)?;

    let mut mask = Mat::default();
    core::in_range(&hsv, lower, upper, &mut mask)?;
    Ok(mask)
}   

 
pub fn threshold(bgr: &Mat,thresh:f64,maxval:f64) -> Result<Mat> {
    let gray = bgr_to_gray(bgr)?;

    let mut bin = Mat::default();
    imgproc::threshold(&gray, &mut bin, thresh, maxval, imgproc::THRESH_BINARY)?;
    Ok(bin)
}


// TODO 我迟早用个枚举把下面的俩玩意封装在一起,为什么不封装？因为多写一个参数确实费劲，不如方法名
//* 开运算 去小白噪点 */
pub fn morph_open(bin: &Mat,kernel:&Mat) -> Result<Mat> {
    let mut out_bin = Mat::default();
    imgproc::morphology_ex(  //这些参数几乎不用变，大概变变kernel就行了，如果真想改，自己new个方法
        &bin,
        &mut out_bin,
        imgproc::MORPH_OPEN,
        kernel,
        core::Point::new(-1, -1),
        1,
        core::BORDER_CONSTANT,
        core::Scalar::all(0.0),
    )?;
    Ok(out_bin)
}

//* 闭运算 补小黑洞 */
pub fn morph_close(bin: &Mat,kernel:&Mat) -> Result<Mat> {
 let mut out_bin = Mat::default();
    imgproc::morphology_ex(  //这些参数几乎不用变，大概变变kernel就行了，如果真想改，自己new个方法
        &bin,
        &mut out_bin,
        imgproc::MORPH_CLOSE,
        kernel,
        core::Point::new(-1, -1),
        1,
        core::BORDER_CONSTANT,
        core::Scalar::all(0.0),
    )?;
    Ok(out_bin)
}

//* 均值模糊 */
pub fn bgr_blur_box(bgr: &Mat,ksize: i32) -> Result<Mat> {
    let mut out = Mat::default();
    imgproc::blur(
        bgr,
        &mut out,
        core::Size::new(ksize, ksize),
        core::Point::new(-1, -1),
        core::BORDER_DEFAULT,
    )?;
    Ok(out)
}

//* 高斯模糊 */
pub fn bgr_blur_gaussian(bgr: &Mat,ksize: i32) -> Result<Mat> {
    let mut out = Mat::default();
    imgproc::gaussian_blur(
        bgr,
        &mut out,
        core::Size::new(ksize, ksize),
        0.0,
        0.0,
        core::BORDER_DEFAULT,
    )?;
    Ok(out)
}

pub fn roi_circle_mask(
    frame_bgr: &Mat,
    radius_ratio: f64, // 0~1（圆半径占短边比例）
) -> Result<(Mat, Mat)> {
    let size = frame_bgr.size()?;
    let w = size.width;
    let h = size.height;

    let cx = w / 2;
    let cy = h / 2;
    let r = ((w.min(h) as f64) * radius_ratio) as i32;
    let mut mask = Mat::zeros(h, w, core::CV_8UC1)?.to_mat()?;
    imgproc::circle(
        &mut mask,
        core::Point::new(cx, cy),
        r,
        core::Scalar::all(255.0),
        -1, // 填充
        imgproc::LINE_8,
        0,
    )?;
    let mut roi = Mat::default();
    core::bitwise_and(frame_bgr, frame_bgr, &mut roi, &mask)?;

    Ok((roi, mask))
}

// TODO 膨胀腐蚀 : diy开运算 


// TODO 腐蚀膨胀 : diy闭运算

 