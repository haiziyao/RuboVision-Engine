use std::{any::Any, sync::Arc};

use async_trait::async_trait;
use tokio::sync::Mutex;

use crate::{DeviceError, config::DeviceConfig};

#[async_trait]
pub trait Device: Send + Sync + 'static {
    async fn create(config: &DeviceConfig) -> Result<Self, DeviceError>
    where
        Self: Sized;
}

#[async_trait]
pub trait MutexDevice: Send + 'static {
    async fn create(config: &DeviceConfig) -> Result<Self, DeviceError>
    where
        Self: Sized;
}

#[derive(Debug, Clone)]
pub struct DeviceRef {
    id: String,
    inner: Arc<dyn Any + Send + Sync>,
}

impl DeviceRef {
    pub fn shared<T>(id: impl Into<String>, device: T) -> Self
    where
        T: Device,
    {
        Self {
            id: id.into(),
            inner: Arc::new(device),
        }
    }

    pub fn mutex<T>(id: impl Into<String>, device: T) -> Self
    where
        T: MutexDevice,
    {
        Self {
            id: id.into(),
            inner: Arc::new(Mutex::new(device)),
        }
    }

    pub fn get<T>(&self) -> Result<Arc<T>, DeviceError>
    where
        T: Device,
    {
        Arc::downcast::<T>(self.inner.clone()).map_err(|_| DeviceError::TypeMismatch {
            id: self.id.clone(),
            expected: std::any::type_name::<T>().to_string(),
        })
    }

    pub fn get_mutex<T>(&self) -> Result<Arc<Mutex<T>>, DeviceError>
    where
        T: MutexDevice,
    {
        Arc::downcast::<Mutex<T>>(self.inner.clone()).map_err(|_| DeviceError::TypeMismatch {
            id: self.id.clone(),
            expected: std::any::type_name::<T>().to_string(),
        })
    }
}
