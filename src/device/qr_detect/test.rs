use anyhow::{Result, bail};
use opencv::{core, highgui, imgproc, prelude::*, videoio};
use quircs::Quirc;

use crate::{
    device::camera,
    utils::device_config_util::get_config,
};

fn decode_qr_quircs(gray_u8: &opencv::core::Mat) -> Result<Vec<String>> {
    // typ() 不是 Result
    let typ = gray_u8.typ();
    if typ != core::CV_8UC1 {
        bail!("gray mat type is not CV_8UC1, got: {}", typ);
    }

    let size = gray_u8.size()?;
    let w = size.width as usize;
    let h = size.height as usize;
    if w == 0 || h == 0 {
        return Ok(vec![]);
    }

    // 稳妥：clone 保证连续
    let gray = gray_u8.try_clone()?;
    let data = gray.data_bytes()?; // &[u8], 长度应为 w*h

    if data.len() < w * h {
        bail!("gray data length {} < w*h {}", data.len(), w * h);
    }

    let mut q = Quirc::default();
    q.resize(w, h); // <- 在你这版 quircs 里返回 ()

    let mut results = Vec::new();
    // 关键：identify 需要 (w, h, data)
    for code in q.identify(w, h, data) {
        let code = code?;
        let decoded = code.decode()?;
        results.push(String::from_utf8_lossy(&decoded.payload).to_string());
    }

    Ok(results)
}

#[test]
#[ignore]
pub fn qr_scan() -> Result<()> {
    use std::time::{Duration, Instant};
    use anyhow::bail;

    let my_config = get_config()?;
    let mut cam = camera::register_qr_camera(my_config.qr_camera_config)?;

    if !opencv::videoio::VideoCapture::is_opened(&cam)? {
        bail!("无法打开二维码相机");
    }

    let show = true;
    if show {
        highgui::named_window("qr", highgui::WINDOW_AUTOSIZE)?;
    }

    let start = Instant::now();
    let timeout = Duration::from_secs(15);

    loop {
        if start.elapsed() > timeout {
            eprintln!("timeout: no qr decoded in 15s");
            break;
        }

        let mut frame = Mat::default();
        cam.read(&mut frame)?;
        if frame.empty() {
            continue;
        }

        let mut gray = Mat::default();
        imgproc::cvt_color(&frame, &mut gray, imgproc::COLOR_BGR2GRAY, 0)?;

        // ===== quircs 解码：遇错继续，不让 test 失败 =====
        match decode_qr_quircs_nonfatal(&gray) {
            Ok(texts) => {
                for t in texts {
                    println!("QR: {}", t);
                }
            }
            Err(e) => {
                eprintln!("decode_qr_quircs error: {e:?}");
            }
        }

        if show {
            highgui::imshow("qr", &frame)?;
            let key = highgui::wait_key(1)?;
            if key == 113 || key == 27 {
                break;
            }
        }
    }

    Ok(())
}

// 非致命版：内部不使用 ? 直接导致崩
fn decode_qr_quircs_nonfatal(gray_u8: &Mat) -> Result<Vec<String>> {
    use quircs::Quirc;

    let typ = gray_u8.typ();
    if typ != core::CV_8UC1 {
        return Ok(vec![]);
    }

    let size = gray_u8.size()?;
    let w = size.width as usize;
    let h = size.height as usize;
    if w == 0 || h == 0 {
        return Ok(vec![]);
    }

    let gray = gray_u8.try_clone()?;
    let data = gray.data_bytes()?;
    if data.len() < w * h {
        return Ok(vec![]);
    }

    let mut q = Quirc::default();
    q.resize(w, h);

    let mut out = Vec::new();
    for code in q.identify(w, h, data) {
        let code = match code {
            Ok(c) => c,
            Err(_) => continue,
        };
        let decoded = match code.decode() {
            Ok(d) => d,
            Err(_) => continue,
        };
        let s = String::from_utf8_lossy(&decoded.payload).to_string();
        if !s.is_empty() {
            out.push(s);
        }
    }
    Ok(out)
}
