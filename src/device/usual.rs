use tracing::info;
use crate::device::Device;

// TODO
pub fn register_camera(args:&Vec<String>)->Device {
    info!("Registering camera with args {:?}", args);
    Device::Camera(" ".to_string())
}