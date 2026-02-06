use crate::device::{camera, config::ColorCameraConfig};

use anyhow::Result;  
use opencv::{
    prelude::*,
    videoio,
    highgui
}; 



pub fn start(config:ColorCameraConfig) -> Result<()> {  

    let camera_filename = config.color_camera;
   
    highgui::named_window("window", highgui::WINDOW_FULLSCREEN)?;
    
    let mut cam = videoio::VideoCapture::from_file(&camera_filename, videoio::CAP_ANY)?;
    let mut frame = Mat::default(); 
  
 
    loop {
        cam.read(&mut frame)?;
        highgui::imshow("window", &frame)?;
        let key = highgui::wait_key(1)?;
        if key == 113 {  
            break;
        }
    }
    Ok(())
}


 
pub fn hsv_debug(config:ColorCameraConfig) -> Result<()> {  

    let camera_filename = config.color_camera;
   
    highgui::named_window("window", highgui::WINDOW_FULLSCREEN)?;
    
    let mut cam = videoio::VideoCapture::from_file(&camera_filename, videoio::CAP_ANY)?;
    let mut frame = Mat::default(); 
  
 
    loop {
        cam.read(&mut frame)?;
        highgui::imshow("window", &frame)?;
        let key = highgui::wait_key(1)?;
        if key == 113 {  
            break;
        }
    }
    Ok(())
}
 
#[test]
fn test_hsv(){
    
}
