use std::{collections::HashMap, marker::PhantomData};

use async_trait::async_trait;

use crate::{Device, DeviceError, DeviceRef, MutexDevice, config::DeviceConfig};

#[derive(Default)]
pub struct DeviceRegister {
    creators: HashMap<String, Box<dyn DeviceCreator>>,
}

impl DeviceRegister {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register_device<T>(&mut self, kind: impl Into<String>)
    where
        T: Device,
    {
        self.creators
            .insert(kind.into(), Box::new(SharedDeviceCreator::<T>::new()));
    }

    pub fn register_mutex_device<T>(&mut self, kind: impl Into<String>)
    where
        T: MutexDevice,
    {
        self.creators
            .insert(kind.into(), Box::new(MutexDeviceCreator::<T>::new()));
    }

    pub fn contains_kind(&self, kind: &str) -> bool {
        self.creators.contains_key(kind)
    }

    pub async fn create(&self, config: &DeviceConfig) -> Result<DeviceRef, DeviceError> {
        let creator =
            self.creators
                .get(config.kind())
                .ok_or_else(|| DeviceError::KindNotRegistered {
                    kind: config.kind().to_string(),
                })?;
        creator.create(config).await
    }
}

#[async_trait]
trait DeviceCreator: Send + Sync {
    async fn create(&self, config: &DeviceConfig) -> Result<DeviceRef, DeviceError>;
}

struct SharedDeviceCreator<T> {
    _device: PhantomData<fn() -> T>,
}

impl<T> SharedDeviceCreator<T> {
    fn new() -> Self {
        Self {
            _device: PhantomData,
        }
    }
}

#[async_trait]
impl<T> DeviceCreator for SharedDeviceCreator<T>
where
    T: Device,
{
    async fn create(&self, config: &DeviceConfig) -> Result<DeviceRef, DeviceError> {
        let device = T::create(config).await?;
        Ok(DeviceRef::shared(config.id(), device))
    }
}

struct MutexDeviceCreator<T> {
    _device: PhantomData<fn() -> T>,
}

impl<T> MutexDeviceCreator<T> {
    fn new() -> Self {
        Self {
            _device: PhantomData,
        }
    }
}

#[async_trait]
impl<T> DeviceCreator for MutexDeviceCreator<T>
where
    T: MutexDevice,
{
    async fn create(&self, config: &DeviceConfig) -> Result<DeviceRef, DeviceError> {
        let device = T::create(config).await?;
        Ok(DeviceRef::mutex(config.id(), device))
    }
}
