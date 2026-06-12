use anyhow::{Context, Result};
use opencv::{core, highgui, prelude::*};

use crate::utils::cv_util::bgr_to_gray;

use super::camera::register_qr_camera;
use super::config::QrDetectConfig;

pub struct QrDetectOutput {
    pub value: i32,
    pub frame: core::Mat,
}

pub fn run_qr_detect_with_frame(config: &QrDetectConfig) -> Result<QrDetectOutput> {
    let mut cam = register_qr_camera(config)?;

    loop {
        let mut frame = core::Mat::default();
        cam.read(&mut frame)?;
        if frame.empty() {
            continue;
        }

        let processed_frame = bgr_to_gray(&frame)?;
        let content = if config.debug_model {
            decode_qr_debugging(&processed_frame)?
        } else {
            decode_qr(&processed_frame)?
        };

        if !content.is_empty() {
            let value = content
                .parse::<i32>()
                .with_context(|| format!("qr payload is not an integer: {content}"))?;
            return Ok(QrDetectOutput { value, frame });
        }

        if config.debug_model {
            highgui::imshow("qr_detect", &processed_frame)?;
            let key = highgui::wait_key(1)?;
            if key == 27 {
                return Ok(QrDetectOutput { value: -1, frame });
            }
        }
    }
}

fn decode_qr(processed_frame: &core::Mat) -> Result<String> {
    let size = processed_frame.size()?;
    let width = size.width as usize;
    let height = size.height as usize;
    let data = processed_frame.data_bytes()?;

    let mut decoder = quircs::Quirc::default();
    let codes = decoder.identify(width, height, &data[..width * height]);

    for code_res in codes {
        let Ok(code) = code_res else {
            continue;
        };
        let Ok(decoded) = code.decode() else {
            continue;
        };
        if let Ok(text) = std::str::from_utf8(&decoded.payload) {
            return Ok(text.to_string());
        }
    }

    Ok(String::new())
}

fn decode_qr_debugging(processed_frame: &core::Mat) -> Result<String> {
    let size = processed_frame.size()?;
    let width = size.width as usize;
    let height = size.height as usize;
    let data = processed_frame.data_bytes()?;

    let mut decoder = quircs::Quirc::default();
    let codes = decoder.identify(width, height, &data[..width * height]);

    for (i, code_res) in codes.enumerate() {
        let code = match code_res {
            Ok(code) => code,
            Err(e) => {
                eprintln!("[QR] extract failed #{i}: {:?}", e);
                continue;
            }
        };

        let decoded = match code.decode() {
            Ok(decoded) => decoded,
            Err(e) => {
                eprintln!("[QR] decode failed #{i}: {:?}", e);
                continue;
            }
        };

        match std::str::from_utf8(&decoded.payload) {
            Ok(text) => {
                println!("[QR] {}", text);
                return Ok(text.to_string());
            }
            Err(_) => println!("[QR] payload bytes: {:?}", decoded.payload),
        }
    }

    Ok(String::new())
}
