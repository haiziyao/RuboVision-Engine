use super::vision::CameraDevice;
use anyhow::{Result, anyhow};
use std::collections::HashMap;
use std::fmt;

#[derive(Debug, Clone)]
pub enum Device {
    Camera(CameraDevice),
    None,
}
impl fmt::Display for Device {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Device::Camera(camera) => write!(f, "Camera({})", camera.path),
            Device::None => write!(f, "None"),
        }
    }
}

pub struct DeviceMap {
    device_list: HashMap<String, Device>,
}

impl DeviceMap {
    pub fn new() -> DeviceMap {
        DeviceMap {
            device_list: HashMap::new(),
        }
    }

    pub fn add(&mut self, device_id: &str, device: Device) {
        self.device_list.insert(device_id.to_string(), device);
    }

    pub fn get_device(&self, device_id: &str) -> Result<Device> {
        self.device_list
            .get(device_id)
            .cloned()
            .ok_or_else(|| anyhow!("unknown device_id `{device_id}`"))
    }
}
