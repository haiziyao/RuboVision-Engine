use std::collections::HashMap;
use crate::device::Device;
use crate::web::WebMessage;



pub struct FunctionWorker{
    pub func_id: String,
    pub func :Box<dyn FnMut(&mut Vec<String>,&mut Device) -> WebMessage + Send>,
    pub args: Vec<String>,
}

impl FunctionWorker{
    pub fn new(func_id: &str, func: Box<dyn FnMut(&mut Vec<String>,&mut Device) -> WebMessage + Send>,args:Vec<String>) -> Self{
        let func_id =func_id.to_string();
        FunctionWorker{
            func_id,
            func,
            args
        }
    }
}

pub struct FuncWorkerMap {
    pub func_worker_map:HashMap<String,FunctionWorker>,
}

impl FuncWorkerMap {
    pub fn new() -> Self{
        FuncWorkerMap {
            func_worker_map:HashMap::new(),
        }
    }

    pub fn add(&mut self,func_id:&str,worker: FunctionWorker){
        self.func_worker_map.insert(func_id.to_string(),worker);
    }

    pub fn get_func(&mut self, func_id: &str) -> Option<FunctionWorker> {
        self.func_worker_map.remove(func_id)
    }

}


