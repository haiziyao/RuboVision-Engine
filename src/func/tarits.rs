use crate::config::ReturnTargets;
use crate::device::Device;
use crate::message::TaskOutput;
use std::collections::HashMap;

pub type Function = fn(&[String], &Device, &ReturnTargets) -> TaskOutput;

pub struct FunctionDef {
    pub func_id: String,
    pub args: Vec<String>,
    pub returns: ReturnTargets,
    pub func: Function,
}

impl FunctionDef {
    pub fn new(func_id: &str, args: Vec<String>, returns: ReturnTargets, func: Function) -> Self {
        Self {
            func_id: func_id.to_string(),
            args,
            returns,
            func,
        }
    }

    pub fn build_worker(&self) -> FunctionWorker {
        FunctionWorker::new(
            &self.func_id,
            self.func,
            self.args.clone(),
            self.returns.clone(),
        )
    }
}

pub struct FunctionWorker {
    pub func_id: String,
    pub func: Function,
    pub args: Vec<String>,
    pub returns: ReturnTargets,
}

impl FunctionWorker {
    pub fn new(func_id: &str, func: Function, args: Vec<String>, returns: ReturnTargets) -> Self {
        let func_id = func_id.to_string();
        Self {
            func_id: func_id.to_string(),
            func,
            args,
            returns,
        }
    }
}

pub struct FuncWorkerMap {
    pub func_worker_map: HashMap<String, FunctionDef>,
}

impl FuncWorkerMap {
    pub fn new() -> Self {
        FuncWorkerMap {
            func_worker_map: HashMap::new(),
        }
    }

    pub fn add(&mut self, func_id: &str, def: FunctionDef) {
        self.func_worker_map.insert(func_id.to_string(), def);
    }

    pub fn get_func(&self, func_id: &str) -> FunctionWorker {
        self.func_worker_map
            .get(func_id)
            .unwrap_or_else(|| panic!("unknown function_id `{func_id}`"))
            .build_worker()
    }
}
