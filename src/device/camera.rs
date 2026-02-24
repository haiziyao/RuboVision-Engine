use crate::config::device_config::{ColorCameraConfig,QrCameraConfig};


use anyhow::{Ok, Result};  
use opencv::{
    videoio,
    highgui
}; 



pub fn register_color_camera(config:ColorCameraConfig) -> Result<videoio::VideoCapture> {  

    let camera_filename = config.color_camera;
    let cam = videoio::VideoCapture::from_file(&camera_filename, videoio::CAP_V4L2)?;
    Ok(cam)
}

pub fn register_qr_camera(config:QrCameraConfig)-> Result<videoio::VideoCapture> {  

    let camera_filename = config.qr_camera;

    let cam = videoio::VideoCapture::from_file(&camera_filename, videoio::CAP_V4L2)?;
    Ok(cam)
}

