use std::collections::{HashMap};
use std::fmt;
#[derive(Debug,Clone)]
pub enum Device {
    Camera(String),
    None,
}
impl fmt::Display for Device {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Device::Camera(name) => write!(f, "Camera({})", name),
            Device::None => write!(f, "None"),
        }
    }
}

pub struct DeviceMap {
    device_list: HashMap<String,Device>,
}


impl DeviceMap {
    pub fn new() -> DeviceMap {
        DeviceMap {
            device_list: HashMap::new(),
        }
    }
    
    
    pub fn add(&mut self, device_id:&str,device: Device) {
        self.device_list.insert(device_id.to_string(),device);
    }

    pub fn get_device(&self, device_id:&str) -> Option<Device> {
        self.device_list.get(device_id).cloned()
    }
}