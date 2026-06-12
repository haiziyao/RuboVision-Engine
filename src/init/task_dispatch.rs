use anyhow::{Result, anyhow};
use tracing::{debug, info};

use crate::device::{Device, DeviceMap};
use crate::func::{FuncWorkerMap, FunctionWorker};
use crate::source::Event;

pub struct TaskDispatcher {
    func_worker_map: FuncWorkerMap,
    device_map: DeviceMap,
}

impl TaskDispatcher {
    pub fn new(func_worker_map: FuncWorkerMap, device_map: DeviceMap) -> Self {
        Self {
            func_worker_map,
            device_map,
        }
    }

    pub fn find_device(&self, event: &Event) -> Result<Device> {
        match event {
            Event::UsualEvent { device_id, .. } => {
                info!("Usual event: {:?} get the device", device_id);
                if device_id == "none" {
                    Ok(Device::None)
                } else {
                    self.device_map.get_device(device_id)
                }
            }
            Event::DebugEvent(debug_msg) => {
                debug!("Debug event: {:?} get no device", debug_msg);
                Ok(Device::None)
            }
            Event::OtherEvent(message) => Err(anyhow!(
                "unsupported event `{message}` while resolving device"
            )),
        }
    }

    pub fn find_func(&self, event: &Event) -> Result<FunctionWorker> {
        match event {
            Event::UsualEvent { function_id, .. } => {
                info!("Usual event: {:?} get the func", function_id);
                self.func_worker_map.get_func(function_id)
            }
            Event::DebugEvent(debug_msg) => {
                info!("Debug event: {:?} get debug_fun", debug_msg);
                self.func_worker_map.get_func("debug_fun")
            }
            Event::OtherEvent(message) => Err(anyhow!(
                "unsupported event `{message}` while resolving function"
            )),
        }
    }

    pub fn runtime_param(&self, event: &Event) -> u8 {
        match event {
            Event::UsualEvent { runtime_param, .. } => *runtime_param,
            Event::DebugEvent(_) | Event::OtherEvent(_) => 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::device::DeviceMap;
    use crate::func::FuncWorkerMap;
    use crate::source::Event;

    use super::TaskDispatcher;

    #[test]
    fn dispatcher_returns_errors_for_unknown_device_and_function() {
        let dispatcher = TaskDispatcher::new(FuncWorkerMap::new(), DeviceMap::new());
        let event = Event::UsualEvent {
            task_id: "missing_task".to_string(),
            function_id: "missing_function".to_string(),
            device_id: "missing_device".to_string(),
            runtime_param: 7,
        };

        assert!(dispatcher.find_device(&event).is_err());
        assert!(dispatcher.find_func(&event).is_err());
        assert_eq!(dispatcher.runtime_param(&event), 7);
    }
}
