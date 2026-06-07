use crate::device::{CameraDevice, Device};
use tracing::info;

pub fn register_camera(args: &[String]) -> Device {
    info!("Registering camera with args {:?}", args);
    Device::Camera(CameraDevice::from_args(args).expect("invalid camera config"))
}
