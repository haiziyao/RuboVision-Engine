use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::{Duration, Instant};

use anyhow::{Context, Result, anyhow};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use opencv::{core, highgui, imgcodecs, prelude::*};
use tokio::sync::mpsc;

use crate::config::WebConfig;
use crate::utils::cv_util::bgr_to_gray;
use crate::web::WebMessage;

use super::super::color::analyze_color_frame;
use super::super::{ColorDetectConfig, QrDetectConfig};
use super::super::{format_cross_value, run_cross_detect_with_frame};
use super::support::{
    color_detect_config_from_config, cross_detect_config_from_config, draw_label, open_camera,
    qr_detect_config_from_config, read_non_empty_frame, web_config_from_config,
};

#[tokio::test(flavor = "multi_thread")]
#[ignore = "starts the original web server for vision tests"]
async fn start_web_for_vision_result() -> Result<()> {
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
fn color_detect_result_to_web_with_base64() -> Result<()> {
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
fn qr_detect_result_to_web_with_base64() -> Result<()> {
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
fn cross_detect_result_to_web_with_base64() -> Result<()> {
    let config = cross_detect_config_from_config()?;
    highgui::named_window("cross_detect_web", highgui::WINDOW_NORMAL)?;

    loop {
        let output = run_cross_detect_with_frame(0, &config)?;
        let result = format_cross_value(&output.result);
        highgui::imshow("cross_detect_web", &output.frame)?;
        highgui::wait_key(1)?;

        send_vision_result_to_web(format!("cross result: {result}"), &output.frame)?;
        if !sleep_after_success()? {
            break;
        }
    }

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
        let analysis = analyze_color_frame(&frame, config)?;
        draw_color_web_info(
            &mut frame,
            &analysis.color_name,
            analysis.ratio,
            config.radius_ratio,
        )?;

        if count == 0 {
            best_color = analysis.color_name;
            count = 1;
        } else if best_color == analysis.color_name {
            count += 1;
        } else {
            best_color.clear();
            count = 0;
        }

        draw_label(
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
            draw_label(&mut frame, &format!("final color: {best_color}"), 10, 90)?;
            highgui::imshow("color_detect_web", &frame)?;
            highgui::wait_key(1)?;
            return Ok((best_color, frame));
        }
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

    opencv::imgproc::circle(
        frame,
        core::Point::new(cx, cy),
        r,
        core::Scalar::new(0.0, 255.0, 0.0, 0.0),
        2,
        opencv::imgproc::LINE_8,
        0,
    )?;

    draw_label(
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
            draw_label(&mut frame, &format!("qr_detect result: {result}"), 10, 30)?;
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
            "failed to connect web server at {addr}; run `cargo test start_web_for_vision_result -- --ignored --nocapture` in another terminal first"
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
