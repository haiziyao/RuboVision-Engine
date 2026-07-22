use std::path::Path;

use rubo_engine::{
    DeviceRef, DeviceRegister,
    config::{ConfigStore, RuboConfig},
};

use crate::{default_rubo_config, device::CameraDevice};

#[cfg(feature = "opencv")]
pub fn show_frame(window: &str, frame: &opencv::core::Mat) -> opencv::Result<()> {
    opencv::highgui::imshow(window, frame)?;
    opencv::highgui::wait_key(0)?;
    Ok(())
}

#[cfg(feature = "opencv")]
pub fn annotate_result(
    frame: &opencv::core::Mat,
    label: &str,
    value: &str,
) -> opencv::Result<opencv::core::Mat> {
    use opencv::{
        core::{Point, Scalar},
        imgproc,
        prelude::MatTraitConst,
    };

    let value = value
        .chars()
        .map(|character| {
            if character.is_control() {
                ' '
            } else {
                character
            }
        })
        .collect::<String>();
    let value = value.trim();
    let text = format!(
        "{label}: {}",
        if value.is_empty() { "NOT FOUND" } else { value }
    );

    let mut annotated = frame.try_clone()?;
    let font = imgproc::FONT_HERSHEY_SIMPLEX;
    let base_scale = 0.8;
    let base_size = imgproc::get_text_size(&text, font, base_scale, 2, &mut 0)?;
    let available_width = (annotated.cols() - 40).max(1);
    let scale = if base_size.width > available_width {
        (base_scale * f64::from(available_width) / f64::from(base_size.width)).max(0.35)
    } else {
        base_scale
    };
    let origin = Point::new(20, 40);

    imgproc::put_text(
        &mut annotated,
        &text,
        origin,
        font,
        scale,
        Scalar::new(0.0, 0.0, 0.0, 0.0),
        5,
        imgproc::LINE_AA,
        false,
    )?;
    imgproc::put_text(
        &mut annotated,
        &text,
        origin,
        font,
        scale,
        Scalar::new(0.0, 255.0, 0.0, 0.0),
        2,
        imgproc::LINE_AA,
        false,
    )?;
    Ok(annotated)
}

pub fn load_config() -> Result<RuboConfig, String> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let app_config =
        ConfigStore::load_app_config(root.join("config")).map_err(|error| error.to_string())?;
    let declared_config = default_rubo_config(&app_config).map_err(|error| error.to_string())?;
    ConfigStore::load_or_init_config(root, &app_config, &declared_config)
        .map_err(|error| error.to_string())
}

pub async fn load_camera(id: &str) -> Result<(RuboConfig, DeviceRef), String> {
    let config = load_config()?;
    let device_config = config
        .devices()
        .get(id)
        .ok_or_else(|| format!("camera config `{id}` missing"))?;
    let mut register = DeviceRegister::new();
    register.register_device::<CameraDevice>("camera");
    let device = register
        .create(device_config)
        .await
        .map_err(|error| error.to_string())?;
    Ok((config, device))
}
