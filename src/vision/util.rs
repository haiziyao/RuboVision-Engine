#[cfg(feature = "opencv")]
use opencv::{
    core::{self, Mat, Point, Scalar},
    imgproc,
    prelude::*,
};

#[cfg(feature = "opencv")]
pub(crate) fn bgr_to_hsv(frame: &Mat) -> opencv::Result<Mat> {
    let mut hsv = Mat::default();
    imgproc::cvt_color(frame, &mut hsv, imgproc::COLOR_BGR2HSV, 0)?;
    Ok(hsv)
}

#[cfg(feature = "opencv")]
pub(crate) fn bgr_to_gray(frame: &Mat) -> opencv::Result<Mat> {
    let mut gray = Mat::default();
    imgproc::cvt_color(frame, &mut gray, imgproc::COLOR_BGR2GRAY, 0)?;
    Ok(gray)
}

#[cfg(feature = "opencv")]
pub(crate) fn hsv_mask(hsv_frame: &Mat, range: [i32; 6]) -> opencv::Result<Mat> {
    let lower = Scalar::new(range[0] as f64, range[2] as f64, range[4] as f64, 0.0);
    let upper = Scalar::new(range[1] as f64, range[3] as f64, range[5] as f64, 0.0);
    let mut mask = Mat::default();
    core::in_range(hsv_frame, &lower, &upper, &mut mask)?;
    Ok(mask)
}

#[cfg(feature = "opencv")]
pub(crate) fn circle_roi(frame: &Mat, radius_ratio: f64) -> opencv::Result<(Mat, Mat)> {
    let size = frame.size()?;
    let mut mask = Mat::zeros(size.height, size.width, core::CV_8UC1)?.to_mat()?;
    let center = Point::new(size.width / 2, size.height / 2);
    let radius = (size.width.min(size.height) as f64 * radius_ratio) as i32;
    imgproc::circle(
        &mut mask,
        center,
        radius,
        Scalar::all(255.0),
        -1,
        imgproc::LINE_8,
        0,
    )?;
    let roi = apply_mask(frame, &mask)?;
    Ok((roi, mask))
}

#[cfg(feature = "opencv")]
pub(crate) fn mask_in_roi(mask: &Mat, roi_mask: &Mat) -> opencv::Result<Mat> {
    let mut output = Mat::default();
    core::bitwise_and(mask, mask, &mut output, roi_mask)?;
    Ok(output)
}

#[cfg(feature = "opencv")]
pub(crate) fn apply_mask(frame: &Mat, mask: &Mat) -> opencv::Result<Mat> {
    let mut output = Mat::default();
    core::bitwise_and(frame, frame, &mut output, mask)?;
    Ok(output)
}

#[cfg(feature = "opencv")]
pub(crate) fn mask_ratio(mask: &Mat, area_mask: &Mat) -> opencv::Result<f64> {
    let area = core::count_non_zero(area_mask)? as f64;
    if area == 0.0 {
        return Ok(0.0);
    }
    Ok(core::count_non_zero(mask)? as f64 / area)
}
