use crate::config::{DeviceParam, DevicesConfig, UartConfig};
use crate::device::UartDeviceConfig;
use crate::device::usual::*;
use crate::device::{Device, DeviceMap};

pub fn register_device(config: DevicesConfig, uart_config: &UartConfig) -> DeviceMap {
    let runtime_uart =
        UartDeviceConfig::from_param(uart_config).expect("invalid message.uart config");

    let mut map = DeviceMap::new();
    config.list.iter().for_each(|device_config| {
        let DeviceParam {
            device_id,
            kind,
            args,
        } = device_config;
        map.add(device_id, device_factory(kind, args, runtime_uart.clone()));
    });
    map
}

fn device_factory(kind: &str, args: &[String], uart: UartDeviceConfig) -> Device {
    match kind {
        "Camera" => register_camera(args, uart),
        _ => panic!("unknown device kind `{kind}`"),
    }
}
