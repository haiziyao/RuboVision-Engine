use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(default)]
pub struct DevicesConfig {
    pub list: Vec<DeviceParam>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DeviceParam {
    pub device_id: String,
    pub kind: DeviceKind,
    pub path: String,
}

#[derive(Debug, Copy, Clone, Deserialize, Serialize)]
pub enum DeviceKind {
    Camera,
}
