use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;
use serde_json::Value;
use tokio::sync::Mutex;

use crate::{
    Device, DeviceRef, FunctionError, Message, MutexDevice, Output, TaskRequest, config::FuncConfig,
};

#[async_trait]
pub trait Function: Send + Sync + 'static {
    async fn call(&self, function_call: FunctionCall<'_>) -> Result<FuncResult, FunctionError>;
}

#[async_trait]
pub trait FunctionAspect: Send + Sync + 'static {
    async fn before(&self, task: &TaskRequest);

    async fn after(&self, output: &Output);
}

#[derive(Debug, Clone, PartialEq)]
pub struct FuncResult {
    value: Value,
    description: String,
}

impl FuncResult {
    pub fn new(value: Value) -> Self {
        Self {
            value,
            description: String::new(),
        }
    }

    pub fn value(&self) -> &Value {
        &self.value
    }

    pub fn description_ref(&self) -> &str {
        &self.description
    }

    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }
}

pub struct FunctionCall<'a> {
    function_config: &'a FuncConfig,
    message: &'a Message,
    devices: FunctionDevices<'a>,
}

impl<'a> FunctionCall<'a> {
    pub fn new(
        function_config: &'a FuncConfig,
        message: &'a Message,
        devices: FunctionDevices<'a>,
    ) -> Self {
        Self {
            function_config,
            message,
            devices,
        }
    }

    pub fn function_config(&self) -> &FuncConfig {
        self.function_config
    }

    pub fn message(&self) -> &Message {
        self.message
    }

    pub fn devices(&self) -> &FunctionDevices<'a> {
        &self.devices
    }
}

#[derive(Clone, Default)]
pub struct FunctionDevices<'a> {
    devices: HashMap<String, &'a DeviceRef>,
}

impl<'a> FunctionDevices<'a> {
    pub fn new() -> Self {
        Self {
            devices: HashMap::new(),
        }
    }

    pub fn insert(&mut self, id: impl Into<String>, device: &'a DeviceRef) {
        self.devices.insert(id.into(), device);
    }

    pub fn get<T>(&self, id: &str) -> Result<Arc<T>, FunctionError>
    where
        T: Device,
    {
        let device = self
            .devices
            .get(id)
            .ok_or_else(|| FunctionError::DeviceNotFound { id: id.to_string() })?;
        device.get::<T>().map_err(FunctionError::from)
    }

    pub fn get_mutex<T>(&self, id: &str) -> Result<Arc<Mutex<T>>, FunctionError>
    where
        T: MutexDevice,
    {
        let device = self
            .devices
            .get(id)
            .ok_or_else(|| FunctionError::DeviceNotFound { id: id.to_string() })?;
        device.get_mutex::<T>().map_err(FunctionError::from)
    }
}
