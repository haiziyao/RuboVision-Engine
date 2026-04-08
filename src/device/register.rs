use tracing::info;
use crate::config::{DeviceParamConfig, DeviceParam};
use crate::device::{Device, DeviceMap};
use crate::device::usual::*;

// TODO :
pub fn register_device(config: DeviceParamConfig) -> DeviceMap {
    let DeviceParamConfig{
        device_config_list:device_config_list,
    } = config;
    let mut map = DeviceMap::new();
    device_config_list.iter().for_each(|device_config| {
        let DeviceParam{device_id,kind,args,} = device_config;
         map.add(device_id,device_factory(kind, args));
    });
    map
}


// TODO: Your must add your Device Kind on here
fn device_factory(kind:&str,args:&Vec<String>) -> Device {
    match kind {
        "Cam" => register_camera(args),
        _ => register_default(args) ,
    }
}


fn register_default(args:&Vec<String>)->Device {
    info!("Registering default with args {:?}", args);
    Device::None
}