use crate::config::device_config::{ColorCameraConfig,QrCameraConfig};


use anyhow::{Ok, Result};  
use opencv::{
    videoio,
    highgui
}; 



pub fn register_color_camera(config:ColorCameraConfig) -> Result<videoio::VideoCapture> {  

    let camera_filename = config.color_camera;
    // Open a GUI window
    highgui::named_window("window", highgui::WINDOW_FULLSCREEN)?;
    // Open the web-camera (assuming you have one)
    let cam = videoio::VideoCapture::from_file(&camera_filename, videoio::CAP_V4L2)?;
    Ok(cam)
}

pub fn register_qr_camera(config:QrCameraConfig)-> Result<videoio::VideoCapture> {  

    let camera_filename = config.qr_camera;
    // Open a GUI window
    highgui::named_window("window", highgui::WINDOW_FULLSCREEN)?;
    // Open the web-camera (assuming you have one)
    let cam = videoio::VideoCapture::from_file(&camera_filename, videoio::CAP_V4L2)?;
    Ok(cam)
}

