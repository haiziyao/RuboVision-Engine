use crate::device::{CameraDevice, Device};
use tracing::info;

pub fn register_camera(path: &str) -> Device {
    info!("Registering camera at {path}");
    Device::Camera(CameraDevice::new(path))
}
