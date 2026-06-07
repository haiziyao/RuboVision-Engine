use crate::config::ReturnTargets;
use crate::device::{Device, DeviceMap};
use crate::func::{FuncWorkerMap, FunctionWorker, fn_debug};
use crate::source::Event;
use tracing::{debug, info, warn};

pub struct TaskDispatcher {
    func_worker_map: FuncWorkerMap,
    device_map: DeviceMap,
}

impl TaskDispatcher {
    pub fn new(func_worker_map: FuncWorkerMap, device_map: DeviceMap) -> Self {
        TaskDispatcher {
            func_worker_map,
            device_map,
        }
    }

    pub fn find_device(&self, event: &Event) -> Device {
        match event {
            Event::UsualEvent(_, _, device_id) => {
                info!("Usual event: {:?} get the device", device_id);
                if device_id == "none" {
                    Device::None
                } else {
                    self.device_map.get_device(device_id)
                }
            }
            Event::DebugEvent(debug_msg) => {
                debug!("Debug event: {:?} get the device", debug_msg);
                Device::None
            }
            _ => {
                warn!("Unhandled event: {:?} in getting the device", event);
                Device::None
            }
        }
    }

    pub fn find_func(&mut self, event: &Event) -> FunctionWorker {
        match event {
            Event::UsualEvent(_, func_id, _) => {
                info!("Usual event: {:?} get the func", func_id);
                self.func_worker_map.get_func(func_id)
            }
            Event::DebugEvent(debug_msg) => {
                info!("Debug event: {:?} get the func", debug_msg);
                let args = vec![debug_msg.to_string()];
                FunctionWorker::new(
                    "debug_fun",
                    fn_debug,
                    args,
                    ReturnTargets {
                        web: true,
                        uart: false,
                        gpio: None,
                    },
                )
            }
            _ => {
                warn!("Unhandled event: {:?} in getting the func", event);
                self.func_worker_map.get_func("debug_fun")
            }
        }
    }
}
