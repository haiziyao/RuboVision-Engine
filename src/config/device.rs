use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(default)]
pub struct DevicesConfig {
    pub list: Vec<DeviceParam>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct DeviceParam {
    pub device_id: String,
    pub kind: String,
    pub args: Vec<String>,
}

impl DeviceParam {}
