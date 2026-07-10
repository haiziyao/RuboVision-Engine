use async_trait::async_trait;
use rubo_engine::{
    Device, DeviceError,
    config::{ConfigAccess, DeviceConfig},
};

#[rubo_engine::device(kind = "camera")]
#[derive(Debug, Clone)]
pub struct CameraDevice {
    path: String,
}

impl CameraDevice {
    pub fn path(&self) -> &str {
        &self.path
    }
}

#[async_trait]
impl Device for CameraDevice {
    async fn create(config: &DeviceConfig) -> Result<Self, DeviceError> {
        Ok(Self {
            path: config.get_or("path", "/dev/video0".to_string())?,
        })
    }
}
