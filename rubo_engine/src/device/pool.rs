use std::collections::HashMap;

use crate::{DeviceError, DeviceRegister, config::RuboConfig};

use super::DeviceRef;

#[derive(Debug, Clone, Default)]
pub struct DevicePool {
    devices: HashMap<String, DeviceRef>,
}

impl DevicePool {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, id: impl Into<String>, device: DeviceRef) {
        self.devices.insert(id.into(), device);
    }

    pub fn get(&self, id: &str) -> Option<&DeviceRef> {
        self.devices.get(id)
    }
}

pub async fn build_device_pool(
    config: &RuboConfig,
    register: &DeviceRegister,
) -> Result<DevicePool, DeviceError> {
    let mut pool = DevicePool::new();
    for device_config in config.devices().values() {
        let device = register.create(device_config).await?;
        pool.insert(device_config.id(), device);
    }
    Ok(pool)
}
