use tracing::warn;
use crate::config::{FuncParamConfig, FuncParam};
use crate::device::Device;
use crate::func::{FuncWorkerMap};
use crate::web::WebMessage;
use crate::func::usual::*;
use crate::func::tarits::*;

pub fn register_func(cfg: FuncParamConfig) -> FuncWorkerMap {
    let FuncParamConfig{func_param_list} = cfg;
    let mut map = FuncWorkerMap::new();
    func_param_list.iter().for_each(|x|{ 
        let FuncParam{ function_id, args } = &x;
        map.add(function_id,function_factory(function_id, args));
    });
    map
}


fn function_factory(function_id: &str, args: &Vec<String>) -> FunctionDef {
    match function_id {
        "example_fn" => FunctionDef::new(function_id, args.clone(), example_fn),
        "debug_fun" => FunctionDef::new(function_id, args.clone(), fn_debug),
        _ => {
            warn!("FuncID missed, default function registered: {}", function_id);
            FunctionDef::new(function_id, args.clone(), fn_default)
        }
    }
}


fn fn_default( ) -> Box<dyn FnMut(&mut Vec<String>,&mut Device) -> WebMessage + Send> {

    warn!("FuncID Missed , default function executing");
    let message = " you can put everything to the MutFunc".to_string();
    Box::new( move |args,device| {
        WebMessage::ok(format!("This is the FnOnce, args are: {}, device is {device} , {message}", args.join(",")))
    })
}

