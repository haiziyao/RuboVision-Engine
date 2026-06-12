use std::collections::HashMap;
use std::sync::Arc;

use anyhow::{Context, Result, anyhow};
use serde::de::DeserializeOwned;

use crate::config::{FunctionEntryConfig, ReturnTargets};
use crate::device::{CameraDevice, Device};
use crate::message::TaskOutput;

pub type FunctionResult = TaskOutput;
pub type FunctionRunner =
    Arc<dyn Fn(u8, &Device) -> Result<FunctionResult> + Send + Sync + 'static>;

pub trait FromDevice: Sized {
    fn from_device(device: &Device) -> Result<&Self>;
}

pub trait ValidateParams {
    fn validate(&self) -> Result<()>;
}

impl FromDevice for CameraDevice {
    fn from_device(device: &Device) -> Result<&Self> {
        match device {
            Device::Camera(camera) => Ok(camera),
            Device::None => Err(anyhow!("function requires a camera device")),
        }
    }
}

#[derive(Debug)]
pub struct NoDevice;

static NO_DEVICE: NoDevice = NoDevice;

impl FromDevice for NoDevice {
    fn from_device(device: &Device) -> Result<&Self> {
        match device {
            Device::None => Ok(&NO_DEVICE),
            Device::Camera(_) => Err(anyhow!("function does not accept a device")),
        }
    }
}

#[derive(Clone)]
pub struct FunctionDef {
    pub func_id: String,
    pub returns: ReturnTargets,
    runner: FunctionRunner,
}

impl FunctionDef {
    pub fn new(func_id: &str, returns: ReturnTargets, runner: FunctionRunner) -> Self {
        Self {
            func_id: func_id.to_string(),
            returns,
            runner,
        }
    }

    pub fn build_worker(&self) -> FunctionWorker {
        FunctionWorker::new(&self.func_id, self.returns.clone(), self.runner.clone())
    }
}

#[derive(Clone)]
pub struct FunctionWorker {
    pub func_id: String,
    pub returns: ReturnTargets,
    runner: FunctionRunner,
}

impl FunctionWorker {
    pub fn new(func_id: &str, returns: ReturnTargets, runner: FunctionRunner) -> Self {
        Self {
            func_id: func_id.to_string(),
            returns,
            runner,
        }
    }

    pub fn run(&self, runtime_param: u8, device: &Device) -> Result<FunctionResult> {
        (self.runner)(runtime_param, device)
    }
}

pub struct FuncWorkerMap {
    func_worker_map: HashMap<String, FunctionDef>,
}

impl FuncWorkerMap {
    pub fn new() -> Self {
        Self {
            func_worker_map: HashMap::new(),
        }
    }

    pub fn add(&mut self, func_id: &str, def: FunctionDef) {
        self.func_worker_map.insert(func_id.to_string(), def);
    }

    pub fn get_func(&self, func_id: &str) -> Result<FunctionWorker> {
        self.func_worker_map
            .get(func_id)
            .map(FunctionDef::build_worker)
            .ok_or_else(|| anyhow!("unknown function_id `{func_id}`"))
    }
}

pub struct FunctionDescriptor {
    pub id: &'static str,
    pub build: fn(&FunctionEntryConfig) -> Result<FunctionDef>,
}

pub fn build_typed_function<Params, FunctionDevice>(
    entry: &FunctionEntryConfig,
    function: fn(&Params, u8, &FunctionDevice) -> Result<FunctionResult>,
) -> Result<FunctionDef>
where
    Params: DeserializeOwned + ValidateParams + Send + Sync + 'static,
    FunctionDevice: FromDevice + Send + Sync + 'static,
{
    let params: Params =
        entry.params.clone().try_into().with_context(|| {
            format!("invalid typed params for function `{}`", entry.function_id)
        })?;
    params
        .validate()
        .with_context(|| format!("invalid values for function `{}`", entry.function_id))?;
    let params = Arc::new(params);
    let runner: FunctionRunner = Arc::new(move |runtime_param, device| {
        let device = FunctionDevice::from_device(device)?;
        function(params.as_ref(), runtime_param, device)
    });

    Ok(FunctionDef::new(
        &entry.function_id,
        entry.returns.clone(),
        runner,
    ))
}

macro_rules! declare_functions {
    (
        $(
            $id:ident(params: $params:ty, device: $device:ty) => $function:path
        ),+ $(,)?
    ) => {
        pub static FUNCTION_DESCRIPTORS: &[$crate::func::FunctionDescriptor] = &[
            $(
                $crate::func::FunctionDescriptor {
                    id: stringify!($id),
                    build: |entry| {
                        $crate::func::build_typed_function::<$params, $device>(
                            entry,
                            $function,
                        )
                    },
                },
            )+
        ];
    };
}

pub(crate) use declare_functions;
