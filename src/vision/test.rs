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

pub fn load_config() -> Result<RuboConfig, String> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let app_config =
        ConfigStore::load_app_config(root.join("config")).map_err(|error| error.to_string())?;
    ConfigStore::load_or_init_config(root, &app_config, &default_rubo_config())
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
