use super::*;
use crate::config::{ColorDetectParams, CrossDetectParams, QrDetectParams, WebConfig, load_config};
use crate::utils::cv_util::{bgr_to_gray, hsv_inrange, hsv_scalar_factory, roi_circle_mask};
use crate::web::WebMessage;
use anyhow::{Context, Result, anyhow};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use opencv::{
    core::{self, Mat, Scalar},
    highgui, imgcodecs, imgproc,
    prelude::*,
    videoio,
};
use std::env;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

fn color_detect_config_from_config() -> Result<ColorDetectConfig> {
    let cfg = load_config().context("failed to load runtime config")?;
    let mut camera = camera_from_config(&cfg, "color_camera")?;
    if let Ok(override_path) = env::var("RUBO_TEST_COLOR_CAMERA") {
        camera.path = override_path;
    }
    if let Ok(override_path) = env::var("RUBO_TEST_CAMERA") {
        camera.path = override_path;
    }

    let func = cfg
        .functions
        .entries
        .iter()
        .find(|function| function.function_id == "color_detect")
        .ok_or_else(|| anyhow!("config does not contain function_id=color_detect"))?;

    let params: ColorDetectParams = func
        .params
        .clone()
        .try_into()
        .context("invalid color_detect params")?;
    Ok(ColorDetectConfig::from_params(&params, &camera))
}

fn qr_detect_config_from_config() -> Result<QrDetectConfig> {
    let cfg = load_config().context("failed to load runtime config")?;
    let mut camera = camera_from_config(&cfg, "qr_camera")?;
    if let Ok(override_path) = env::var("RUBO_TEST_QR_CAMERA") {
        camera.path = override_path;
    }
    if let Ok(override_path) = env::var("RUBO_TEST_CAMERA") {
        camera.path = override_path;
    }

    let func = cfg
        .functions
        .entries
        .iter()
        .find(|function| function.function_id == "qr_detect")
        .ok_or_else(|| anyhow!("config does not contain function_id=qr_detect"))?;

    let params: QrDetectParams = func
        .params
        .clone()
        .try_into()
        .context("invalid qr_detect params")?;
    Ok(QrDetectConfig::from_params(&params, &camera))
}

fn cross_detect_config_from_config() -> Result<CrossDetectConfig> {
    let cfg = load_config().context("failed to load runtime config")?;
    let mut camera = camera_from_config(&cfg, "cross_camera")?;
    if let Ok(override_path) = env::var("RUBO_TEST_CROSS_CAMERA") {
        camera.path = override_path;
    }
    if let Ok(override_path) = env::var("RUBO_TEST_CAMERA") {
        camera.path = override_path;
    }

    let func = cfg
        .functions
        .entries
        .iter()
        .find(|function| function.function_id == "cross_detect")
        .ok_or_else(|| anyhow!("config does not contain function_id=cross_detect"))?;

    let params: CrossDetectParams = func
        .params
        .clone()
        .try_into()
        .context("invalid cross_detect params")?;
    Ok(CrossDetectConfig::from_params(&params, &camera))
}

fn configured_device_path(device_id: &str) -> Result<String> {
    let cfg = load_config().context("failed to load runtime config")?;
    Ok(camera_from_config(&cfg, device_id)?.path)
}

fn web_config_from_config() -> Result<WebConfig> {
    let cfg = load_config().context("failed to load runtime config")?;
    Ok(cfg.message.web)
}

fn camera_from_config(cfg: &crate::config::RuntimeConfig, device_id: &str) -> Result<CameraDevice> {
    let device = cfg
        .devices
        .list
        .iter()
        .find(|device| device.device_id == device_id)
        .ok_or_else(|| anyhow!("config does not contain device_id={device_id}"))?;
    Ok(CameraDevice::new(&device.path))
}

fn configured_color_names() -> Result<Vec<String>> {
    let cfg = load_config().context("failed to load runtime config")?;
    let func = cfg
        .functions
        .entries
        .iter()
        .find(|function| function.function_id == "color_detect")
        .ok_or_else(|| anyhow!("config does not contain function_id=color_detect"))?;

    let params: ColorDetectParams = func
        .params
        .clone()
        .try_into()
        .context("invalid color_detect params")?;
    Ok(params
        .color_ranges
        .iter()
        .map(|range| range.name.clone())
        .collect())
}

fn open_camera(path: &str) -> Result<videoio::VideoCapture> {
    let cam = videoio::VideoCapture::from_file(path, videoio::CAP_V4L2)
        .with_context(|| format!("failed to open camera {path}"))?;
    if !videoio::VideoCapture::is_opened(&cam)? {
        return Err(anyhow!("camera is not opened: {path}"));
    }
    Ok(cam)
}

fn read_non_empty_frame(cam: &mut videoio::VideoCapture) -> Result<Mat> {
    for _ in 0..30 {
        let mut frame = Mat::default();
        cam.read(&mut frame)?;
        if !frame.empty() {
            return Ok(frame);
        }
    }

    Err(anyhow!("camera returned empty frames"))
}

#[test]
fn test_color_detect_config_from_config_file() -> Result<()> {
    let device = color_detect_config_from_config()?;
    let names: Vec<String> = device
        .color_ranges
        .iter()
        .map(|range| range.name.clone())
        .collect();

    assert_eq!(device.path, configured_device_path("color_camera")?);
    assert_eq!(names, configured_color_names()?);
    Ok(())
}

#[test]
fn test_qr_detect_config_from_config_file() -> Result<()> {
    let device = qr_detect_config_from_config()?;

    assert_eq!(device.path, configured_device_path("qr_camera")?);
    assert!(!device.debug_model);
    Ok(())
}

#[test]
fn test_cross_detect_config_from_config_file() -> Result<()> {
    let device = cross_detect_config_from_config()?;

    assert_eq!(device.path, configured_device_path("cross_camera")?);
    Ok(())
}

#[test]
#[ignore = "requires USB camera; optionally set RUBO_TEST_CAMERA=/dev/videoX"]
fn test_usb_camera_open_and_read_frame_from_config() -> Result<()> {
    let config = color_detect_config_from_config()?;
    let mut cam = open_camera(&config.path)?;
    let frame = read_non_empty_frame(&mut cam)?;
    let size = frame.size()?;

    println!(
        "camera={} frame={}x{}",
        config.path, size.width, size.height
    );
    assert!(size.width > 0 && size.height > 0);
    Ok(())
}

#[test]
#[ignore = "requires color camera and GUI; press q or ESC to exit"]
fn test_color_detect_single_function_with_imshow() -> Result<()> {
    let mut config = color_detect_config_from_config()?;
    config.debug_model = true;

    let result = run_color_detect(&config)?;
    println!("color_detect result: {result}");
    Ok(())
}

#[test]
#[ignore = "requires QR camera and GUI; press ESC to exit if no QR is found"]
fn test_qr_detect_single_function_with_imshow() -> Result<()> {
    let mut config = qr_detect_config_from_config()?;
    config.debug_model = true;

    let result = run_qr_detect(&config)?;
    println!("qr_detect result: {result}");
    Ok(())
}

#[test]
fn test_cross_detect_single_function() -> Result<()> {
    let config = cross_detect_config_from_config()?;
    let result = run_cross_detect(&config)?;
    println!("cross_detect result: {result}");
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "starts the original web server for vision tests"]
async fn test_start_web_for_vision_result() -> Result<()> {
    let config = web_config_from_config()?;
    let url = format!("http://{}:{}", config.host, config.port);
    let (tx, rx) = mpsc::channel::<WebMessage>(32);

    println!("starting original vision web server: {url}");
    println!("keep this test running, then run a vision web result test in another terminal");
    let _keep_sender = tx;
    crate::web::run(config, rx, None).await
}

#[test]
#[ignore = "requires color camera and GUI; loops forever and auto-starts web if needed"]
fn test_color_detect_result_to_web_with_base64() -> Result<()> {
    let config = color_detect_config_from_config()?;

    loop {
        let (result, frame) = color_detect_frame_for_web(&config)?;
        send_vision_result_to_web(format!("color_detect result: {result}"), &frame)?;
        if !sleep_after_success()? {
            break;
        }
    }

    Ok(())
}

#[test]
#[ignore = "requires QR camera and GUI; loops forever and auto-starts web if needed"]
fn test_qr_detect_result_to_web_with_base64() -> Result<()> {
    let config = qr_detect_config_from_config()?;

    loop {
        let (result, frame) = qr_detect_frame_for_web(&config)?;
        send_vision_result_to_web(format!("qr_detect result: {result}"), &frame)?;
        if !sleep_after_success()? {
            break;
        }
    }

    Ok(())
}

#[test]
#[ignore = "requires cross camera and GUI; loops forever and auto-starts web if needed"]
fn test_cross_detect_result_to_web_with_base64() -> Result<()> {
    let config = cross_detect_config_from_config()?;
    highgui::named_window("cross_detect_web", highgui::WINDOW_NORMAL)?;

    loop {
        let result = run_cross_detect(&config)?;
        let mut cam = open_camera(&config.path)?;
        let mut frame = read_non_empty_frame(&mut cam)?;
        draw_web_label(
            &mut frame,
            &format!("cross_detect result: {result}"),
            10,
            30,
        )?;
        highgui::imshow("cross_detect_web", &frame)?;
        highgui::wait_key(1)?;

        send_vision_result_to_web(format!("cross_detect result: {result}"), &frame)?;
        if !sleep_after_success()? {
            break;
        }
    }

    Ok(())
}

#[test]
#[ignore = "requires USB camera and GUI; press q or ESC to exit"]
fn test_usb_hsv_trackbar_from_config() -> Result<()> {
    let config = color_detect_config_from_config()?;
    let mut cam = open_camera(&config.path)?;

    highgui::named_window("controls", highgui::WINDOW_AUTOSIZE)?;
    highgui::named_window("frame", highgui::WINDOW_NORMAL)?;
    highgui::named_window("roi", highgui::WINDOW_NORMAL)?;
    highgui::named_window("mask", highgui::WINDOW_NORMAL)?;
    highgui::named_window("result", highgui::WINDOW_NORMAL)?;

    let mut h_min = 0;
    let mut h_max = 179;
    let mut s_min = 0;
    let mut s_max = 255;
    let mut v_min = 0;
    let mut v_max = 255;

    highgui::create_trackbar("H min", "controls", Some(&mut h_min), 179, None)?;
    highgui::create_trackbar("H max", "controls", Some(&mut h_max), 179, None)?;
    highgui::create_trackbar("S min", "controls", Some(&mut s_min), 255, None)?;
    highgui::create_trackbar("S max", "controls", Some(&mut s_max), 255, None)?;
    highgui::create_trackbar("V min", "controls", Some(&mut v_min), 255, None)?;
    highgui::create_trackbar("V max", "controls", Some(&mut v_max), 255, None)?;

    loop {
        let frame = read_non_empty_frame(&mut cam)?;
        let (mut roi, circle_mask) = roi_circle_mask(&frame, config.radius_ratio)?;

        let hmin = highgui::get_trackbar_pos("H min", "controls")?;
        let hmax = highgui::get_trackbar_pos("H max", "controls")?;
        let smin = highgui::get_trackbar_pos("S min", "controls")?;
        let smax = highgui::get_trackbar_pos("S max", "controls")?;
        let vmin = highgui::get_trackbar_pos("V min", "controls")?;
        let vmax = highgui::get_trackbar_pos("V max", "controls")?;

        let (h1, h2) = ordered_pair(hmin, hmax);
        let (s1, s2) = ordered_pair(smin, smax);
        let (v1, v2) = ordered_pair(vmin, vmax);
        let current_hsv = [h1, h2, s1, s2, v1, v2];

        let lower = core::Scalar::new(h1 as f64, s1 as f64, v1 as f64, 0.0);
        let upper = core::Scalar::new(h2 as f64, s2 as f64, v2 as f64, 0.0);
        let mask = hsv_inrange(&roi, &lower, &upper)?;

        let mut mask_in_circle = Mat::default();
        core::bitwise_and(&mask, &mask, &mut mask_in_circle, &circle_mask)?;

        let mut result = Mat::default();
        core::bitwise_and(&roi, &roi, &mut result, &mask_in_circle)?;

        draw_hsv_label(&mut roi, current_hsv)?;
        highgui::imshow("frame", &frame)?;
        highgui::imshow("roi", &roi)?;
        highgui::imshow("mask", &mask_in_circle)?;
        highgui::imshow("result", &result)?;

        let key = highgui::wait_key(1)?;
        if key == 113 || key == 27 {
            println!(
                "last hsv config: color.name={},{},{},{},{},{}",
                current_hsv[0],
                current_hsv[1],
                current_hsv[2],
                current_hsv[3],
                current_hsv[4],
                current_hsv[5]
            );
            break;
        }
    }

    Ok(())
}

fn ordered_pair(a: i32, b: i32) -> (i32, i32) {
    if a <= b { (a, b) } else { (b, a) }
}

fn draw_hsv_label(frame: &mut Mat, hsv: [i32; 6]) -> Result<()> {
    let label = format!(
        "HSV [{},{}] [{},{}] [{},{}]",
        hsv[0], hsv[1], hsv[2], hsv[3], hsv[4], hsv[5]
    );
    imgproc::put_text(
        frame,
        &label,
        core::Point::new(10, 30),
        imgproc::FONT_HERSHEY_SIMPLEX,
        0.8,
        Scalar::new(255.0, 255.0, 255.0, 0.0),
        2,
        imgproc::LINE_AA,
        false,
    )?;
    Ok(())
}

fn color_detect_frame_for_web(config: &ColorDetectConfig) -> Result<(String, Mat)> {
    let mut cam = open_camera(&config.path)?;
    let mut best_color = String::new();
    let mut count = 0;
    let stable_count = config.loop_count;

    highgui::named_window("color_detect_web", highgui::WINDOW_NORMAL)?;

    loop {
        let mut frame = read_non_empty_frame(&mut cam)?;
        let (_roi, circle_mask) = roi_circle_mask(&frame, config.radius_ratio)?;
        let (color_name, ratio) = detect_color_for_web(&frame, &circle_mask, config)?;
        draw_color_web_info(&mut frame, &color_name, ratio, config.radius_ratio)?;

        if count == 0 {
            best_color = color_name;
            count = 1;
        } else if best_color == color_name {
            count += 1;
        } else {
            best_color.clear();
            count = 0;
        }

        draw_web_label(
            &mut frame,
            &format!("stable count: {count}/{stable_count}"),
            10,
            60,
        )?;
        highgui::imshow("color_detect_web", &frame)?;
        let key = highgui::wait_key(1)?;
        if key == 113 || key == 27 {
            return Err(anyhow!("color_detect_web canceled before final result"));
        }

        if count >= stable_count {
            draw_web_label(&mut frame, &format!("final color: {best_color}"), 10, 90)?;
            highgui::imshow("color_detect_web", &frame)?;
            highgui::wait_key(1)?;
            return Ok((best_color, frame));
        }
    }
}

fn detect_color_for_web(
    frame_bgr: &Mat,
    circle_mask: &Mat,
    config: &ColorDetectConfig,
) -> Result<(String, f64)> {
    let mut best_name = "unknown".to_string();
    let mut best_ratio = 0.0_f64;

    for color in &config.color_ranges {
        let (lower, upper) = hsv_scalar_factory(color.hsv)?;
        let color_mask = hsv_inrange(frame_bgr, &lower, &upper)?;

        let mut inside = Mat::default();
        core::bitwise_and(&color_mask, &color_mask, &mut inside, circle_mask)?;

        let hit = core::count_non_zero(&inside)? as f64;
        let total = core::count_non_zero(circle_mask)? as f64;
        let ratio = if total > 0.0 { hit / total } else { 0.0 };

        if ratio > best_ratio {
            best_ratio = ratio;
            best_name = color.name.clone();
        }
    }

    if best_ratio >= config.detect_area_access_rate {
        Ok((best_name, best_ratio))
    } else {
        Ok(("unknown".to_string(), best_ratio))
    }
}

fn draw_color_web_info(
    frame: &mut Mat,
    color_name: &str,
    ratio: f64,
    radius_ratio: f64,
) -> Result<()> {
    let size = frame.size()?;
    let w = size.width;
    let h = size.height;
    let cx = w / 2;
    let cy = h / 2;
    let r = ((w.min(h) as f64) * radius_ratio) as i32;

    imgproc::circle(
        frame,
        core::Point::new(cx, cy),
        r,
        Scalar::new(0.0, 255.0, 0.0, 0.0),
        2,
        imgproc::LINE_8,
        0,
    )?;

    draw_web_label(
        frame,
        &format!("color: {color_name} ratio: {ratio:.2}"),
        10,
        30,
    )
}

fn qr_detect_frame_for_web(config: &QrDetectConfig) -> Result<(i32, Mat)> {
    let mut cam = open_camera(&config.path)?;

    highgui::named_window("qr_detect_web", highgui::WINDOW_NORMAL)?;

    loop {
        let mut frame = read_non_empty_frame(&mut cam)?;
        let gray = bgr_to_gray(&frame)?;
        let content = decode_qr_for_web(&gray)?;
        highgui::imshow("qr_detect_web", &frame)?;
        let key = highgui::wait_key(1)?;
        if key == 27 {
            return Err(anyhow!("qr_detect_web canceled before final result"));
        }

        if !content.is_empty() {
            let result = content
                .parse::<i32>()
                .with_context(|| format!("qr payload is not an integer: {content}"))?;
            draw_web_label(&mut frame, &format!("qr_detect result: {result}"), 10, 30)?;
            highgui::imshow("qr_detect_web", &frame)?;
            highgui::wait_key(1)?;
            return Ok((result, frame));
        }
    }
}

fn decode_qr_for_web(processed_frame: &Mat) -> Result<String> {
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

fn draw_web_label(frame: &mut Mat, text: &str, x: i32, y: i32) -> Result<()> {
    imgproc::put_text(
        frame,
        text,
        core::Point::new(x, y),
        imgproc::FONT_HERSHEY_SIMPLEX,
        0.8,
        Scalar::new(255.0, 255.0, 255.0, 0.0),
        2,
        imgproc::LINE_AA,
        false,
    )?;
    Ok(())
}

fn send_vision_result_to_web(text: String, frame: &Mat) -> Result<()> {
    let config = web_config_from_config()?;
    let image = mat_to_jpeg_data_url(frame)?;
    let message = WebMessage::with_image(text, image);
    match post_web_message(&config, &message) {
        Ok(()) => {}
        Err(first_error) => {
            println!("web server is not ready: {first_error:#}");
            println!("starting original web server in this test process...");
            start_original_web_for_test(&config)?;
            post_web_message(&config, &message).with_context(|| {
                format!(
                    "failed to send vision result after starting web; first error: {first_error:#}"
                )
            })?;
        }
    };

    println!(
        "vision result sent to web: http://{}:{}",
        config.host, config.port
    );
    println!("open the page and click refresh/start polling to view the result");
    Ok(())
}

fn sleep_after_success() -> Result<bool> {
    println!("detect success, sleep 5 seconds before next detect");
    let key = highgui::wait_key(5000)?;
    Ok(key != 113 && key != 27)
}

fn start_original_web_for_test(config: &WebConfig) -> Result<()> {
    let config = config.clone();
    let addr = format!("{}:{}", config.host, config.port);
    std::thread::spawn(move || {
        let runtime = match tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
        {
            Ok(runtime) => runtime,
            Err(e) => {
                eprintln!("failed to build web runtime: {e:#}");
                return;
            }
        };

        runtime.block_on(async move {
            let (_tx, rx) = mpsc::channel::<WebMessage>(32);
            if let Err(e) = crate::web::run(config, rx, None).await {
                eprintln!("vision web server failed: {e:#}");
            }
        });
    });

    wait_web_ready(&addr, Duration::from_secs(3))
}

fn wait_web_ready(addr: &str, timeout: Duration) -> Result<()> {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        match TcpStream::connect(addr) {
            Ok(_) => return Ok(()),
            Err(_) => std::thread::sleep(Duration::from_millis(100)),
        }
    }

    Err(anyhow!("web server did not become ready at {addr}"))
}

fn post_web_message(config: &WebConfig, message: &WebMessage) -> Result<()> {
    let body = serde_json::to_string(message).context("failed to serialize web message")?;
    let addr = format!("{}:{}", config.host, config.port);
    let mut stream = TcpStream::connect(&addr).with_context(|| {
        format!(
            "failed to connect web server at {addr}; run `cargo test test_start_web_for_vision_result -- --ignored --nocapture` in another terminal first"
        )
    })?;
    let request = format!(
        "POST /message HTTP/1.1\r\nHost: {addr}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    stream
        .write_all(request.as_bytes())
        .context("failed to write web request")?;
    stream.flush().context("failed to flush web request")?;

    let mut response = String::new();
    stream
        .read_to_string(&mut response)
        .context("failed to read web response")?;
    let status = response.lines().next().unwrap_or("");
    if status.contains(" 200 ") || status.contains(" 202 ") {
        Ok(())
    } else {
        Err(anyhow!("web server returned unexpected response: {status}"))
    }
}

fn mat_to_jpeg_data_url(frame: &Mat) -> Result<String> {
    let mut buf = core::Vector::<u8>::new();
    let params = core::Vector::<i32>::new();
    imgcodecs::imencode(".jpg", frame, &mut buf, &params)?;

    let encoded = STANDARD.encode(buf.to_vec());
    Ok(format!("data:image/jpeg;base64,{encoded}"))
}
