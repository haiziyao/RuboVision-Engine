use std::thread::sleep;
use std::time::Duration;
use tracing::field::debug;
use tracing::warn;
use crate::config::{FuncParamConfig, FuncParam};
use crate::device::Device;
use crate::func::{FuncWorkerMap, FunctionWorker};
use crate::web::WebMessage;
use crate::func::usual::*;

pub fn register_func(cfg: FuncParamConfig) -> FuncWorkerMap {
    let FuncParamConfig{func_param_list:func_param_list} = cfg;
    let mut map = FuncWorkerMap::new();
    func_param_list.iter().for_each(|x|{ 
        let FuncParam{ function_id, args } = &x;
        map.add(function_id,function_factory(function_id, args));
    });
    map
}


fn function_factory(function_id:&str,args:&Vec<String>) -> FunctionWorker{
    match function_id {
        "example_fn" => register_factory(function_id,args,example_fn()),
        _ => register_factory(function_id,args,fn_default())
    }
}

fn register_factory(func_id:&str, args:&Vec<String>, func :Box<dyn FnMut(&mut Vec<String>,&mut Device) -> WebMessage + Send>) -> FunctionWorker {
     
    FunctionWorker::new(func_id,func,args.clone())
}


fn fn_default( ) -> Box<dyn FnMut(&mut Vec<String>,&mut Device) -> WebMessage + Send> {

    warn!("FuncID Missed , default function executing");
    let message = " you can put everything to the MutFunc".to_string();
    Box::new( move |args,device| {
        WebMessage::ok(format!("This is the FnOnce, args are: {}, device is {device} , {message}", args.join(",")))
    })
}

