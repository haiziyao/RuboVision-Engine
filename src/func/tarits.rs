use std::collections::HashMap;
use crate::device::Device;
use crate::web::WebMessage;

pub type FunctionBuilder =
fn() -> Box<dyn FnMut(&mut Vec<String>, &mut Device) -> WebMessage + Send>;

pub struct FunctionDef {
    pub func_id: String,
    pub args: Vec<String>,
    pub builder: FunctionBuilder,
}

impl FunctionDef {
    pub fn new(func_id: &str, args: Vec<String>, builder: FunctionBuilder) -> Self {
        Self {
            func_id: func_id.to_string(),
            args,
            builder,
        }
    }

    pub fn build_worker(&self) -> FunctionWorker {
        FunctionWorker::new(
            &self.func_id,
            (self.builder)(),
            self.args.clone(),
        )
    }
}


pub struct FunctionWorker{
    pub func_id: String,
    pub func :Box<dyn FnMut(&mut Vec<String>,&mut Device) -> WebMessage + Send>,
    pub args: Vec<String>,
}

impl FunctionWorker{
    pub fn new(func_id: &str, func: Box<dyn FnMut(&mut Vec<String>,&mut Device) -> WebMessage + Send>,args:Vec<String>) -> Self{
        let func_id =func_id.to_string();
        Self {
            func_id: func_id.to_string(),
            func,
            args,
        }
    }
}

pub struct FuncWorkerMap {
    pub func_worker_map:HashMap<String, FunctionDef>,
}

impl FuncWorkerMap {
    pub fn new() -> Self{
        FuncWorkerMap {
            func_worker_map:HashMap::new(),
        }
    }

    pub fn add(&mut self, func_id: &str, def: FunctionDef) {
        self.func_worker_map.insert(func_id.to_string(), def);
    }

    pub fn get_func(&self, func_id: &str) -> Option<FunctionWorker> {
        self.func_worker_map.get(func_id).map(|def| def.build_worker())
    }

}


