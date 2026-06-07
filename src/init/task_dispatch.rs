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
            Event::UsualEvent(_, _, device_id) => {
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
            Event::UsualEvent(_, func_id, _) => {
                info!("Usual event: {:?} get the func", func_id);
                self.func_worker_map.get_func(func_id)
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
        let event = Event::UsualEvent(
            "missing_task".to_string(),
            "missing_function".to_string(),
            "missing_device".to_string(),
        );

        assert!(dispatcher.find_device(&event).is_err());
        assert!(dispatcher.find_func(&event).is_err());
    }
}
