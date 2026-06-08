use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use opencv::{core, imgcodecs, prelude::*};

#[allow(dead_code)]
pub fn image_to_data_url(path: impl AsRef<Path>) -> Result<String> {
    let path = path.as_ref();

    let bytes =
        fs::read(path).with_context(|| format!("failed to read image file: {}", path.display()))?;

    let ext = path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("png")
        .to_ascii_lowercase();

    let mime = match ext.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "bmp" => "image/bmp",
        _ => "application/octet-stream",
    };

    let encoded = STANDARD.encode(bytes);
    Ok(format!("data:{mime};base64,{encoded}"))
}

pub fn mat_to_jpeg_data_url(frame: &Mat) -> Result<String> {
    let mut buf = core::Vector::<u8>::new();
    let params = core::Vector::<i32>::new();
    imgcodecs::imencode(".jpg", frame, &mut buf, &params)
        .context("failed to encode Mat as JPEG")?;

    let encoded = STANDARD.encode(buf.to_vec());
    Ok(format!("data:image/jpeg;base64,{encoded}"))
}

#[cfg(test)]
#[test]
fn test_image_to_data_url() {
    let str = image_to_data_url("static/image/a.jpg").unwrap();
    println!("{}", str);
}

#[cfg(test)]
#[test]
fn mat_to_jpeg_data_url_encodes_frame_for_web() -> Result<()> {
    let frame = Mat::new_rows_cols_with_default(
        8,
        8,
        core::CV_8UC3,
        core::Scalar::new(0.0, 0.0, 255.0, 0.0),
    )?;

    let image = mat_to_jpeg_data_url(&frame)?;

    assert!(image.starts_with("data:image/jpeg;base64,"));
    assert!(image.len() > "data:image/jpeg;base64,".len());
    Ok(())
}
