use crate::config::{DeviceParam, DevicesConfig};
use crate::device::usual::*;
use crate::device::{Device, DeviceMap};

pub fn register_device(config: DevicesConfig) -> DeviceMap {
    let mut map = DeviceMap::new();
    config.list.iter().for_each(|device_config| {
        let DeviceParam {
            device_id,
            kind,
            args,
        } = device_config;
        map.add(device_id, device_factory(kind, args));
    });
    map
}

fn device_factory(kind: &str, args: &[String]) -> Device {
    match kind {
        "Camera" => register_camera(args),
        _ => panic!("unknown device kind `{kind}`"),
    }
}
