use crate::device::{camera, config::ColorCameraConfig};

use anyhow::Result;  
use opencv::{
    prelude::*,
    videoio,
    highgui
}; 



pub fn main(config:ColorCameraConfig) -> Result<()> { // Note, this is anyhow::Result

    let camera_filename = config.color_camera;
    // Open a GUI window
    highgui::named_window("window", highgui::WINDOW_FULLSCREEN)?;
    // Open the web-camera (assuming you have one)
    let mut cam = videoio::VideoCapture::from_file(&camera_filename, videoio::CAP_ANY)?;
    let mut frame = Mat::default(); // This array will store the web-cam data
    // Read the camera
    // and display in the window
    loop {
        cam.read(&mut frame)?;
        highgui::imshow("window", &frame)?;
        let key = highgui::wait_key(1)?;
        if key == 113 { // quit with q
            break;
        }
    }
    Ok(())
}

 

