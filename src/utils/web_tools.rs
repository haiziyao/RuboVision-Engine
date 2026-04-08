use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use base64::{engine::general_purpose::STANDARD, Engine as _};

pub fn image_to_data_url(path: impl AsRef<Path>) -> Result<String> {
    let path = path.as_ref();

    let bytes = fs::read(path)
        .with_context(|| format!("failed to read image file: {}", path.display()))?;

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


#[cfg(test)]
#[test]
fn test_image_to_data_url() {
    let str = image_to_data_url("static/image/a.jpg").unwrap();
    println!("{}", str);
}