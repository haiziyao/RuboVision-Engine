use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(default)]
pub struct DeviceParamConfig {
    pub device_config_list: Vec<DeviceParam>,
}


#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct DeviceParam{
    pub device_id: String,
    pub kind: String,
    pub args: Vec<String>,
}


impl DeviceParamConfig {

}

impl DeviceParam {

}