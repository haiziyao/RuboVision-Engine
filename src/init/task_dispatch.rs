use tracing::{debug, info, warn};
use crate::source::{Event};
use crate::device::{Device, DeviceMap};
use crate::func::{fn_debug, FuncWorkerMap, FunctionWorker};


#[warn(unused)]
pub struct TaskDispatcher{
    func_worker_map: FuncWorkerMap,
    device_map: DeviceMap,
}



impl TaskDispatcher {
    pub fn new(func_worker_map: FuncWorkerMap,device_map: DeviceMap) -> Self {
        TaskDispatcher {
            func_worker_map,
            device_map,
        }
    }
    
    
    pub fn find_device(&self,event: &Event) -> Option<Device> {
        match event {
            Event::UsualEvent(_,_,device_id) => {
                info!("Usual event: {:?} get the device", device_id);
                self.device_map.get_device(device_id)
            },
            Event::DebugEvent(debug_msg) => {
                debug!("Debug event: {:?} get the device", debug_msg);
                None
            },
            _ => {
                warn!("Unhandled event: {:?} in getting the device", event);
                None
            }
        }
    }

    
    pub fn find_func(&mut self,event: &Event)-> Option<FunctionWorker> {
        match event {
            Event::UsualEvent(_,func_id,_) => {
                info!("Usual event: {:?} get the func", func_id);
                self.func_worker_map.get_func(func_id)
            },
            Event::DebugEvent(debug_msg) => {
                info!("Debug event: {:?} get the func", debug_msg);
                let mut args = Vec::new();
                args.push(debug_msg.to_string());
                Some(FunctionWorker::new("debug_fun",fn_debug(),args))
            },
            _ => {
                warn!("Unhandled event: {:?} in getting the func", event);
                None
            }
        }
    }




}


