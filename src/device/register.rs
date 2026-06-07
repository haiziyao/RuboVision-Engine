use crate::config::{DeviceKind, DeviceParam, DevicesConfig};
use crate::device::usual::*;
use crate::device::{Device, DeviceMap};

pub fn register_device(config: DevicesConfig) -> DeviceMap {
    let mut map = DeviceMap::new();
    config.list.iter().for_each(|device_config| {
        let DeviceParam {
            device_id,
            kind,
            path,
        } = device_config;
        map.add(device_id, device_factory(*kind, path));
    });
    map
}

fn device_factory(kind: DeviceKind, path: &str) -> Device {
    match kind {
        DeviceKind::Camera => register_camera(path),
    }
}
