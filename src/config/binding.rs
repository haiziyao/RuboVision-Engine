use serde::{Deserialize, Serialize};

// 这个的属性的命名也是难绷。。。以后再改吧
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(default)]
pub struct BindingsConfig {
    pub uart_source: Vec<UartBinding>,
    pub timer_source: Vec<TimerBinding>,
    pub loop_source: Vec<LoopBinding>,
    pub debug_source: Vec<DebugBinding>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UartBinding {
    pub task_id: String,
    pub source_key: u8,
    pub device_id: String,
    pub function_id: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TimerBinding {
    pub task_id: String,
    pub source_key: String,
    pub device_id: String,
    pub function_id: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LoopBinding {
    pub task_id: String,
    pub source_key: String,
    pub device_id: String,
    pub function_id: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DebugBinding {
    pub task_id: String,
    pub source_key: String,
    pub device_id: String,
    pub function_id: String,
}

pub struct TaskBindingRef<'a> {
    pub task_id: &'a str,
    pub device_id: &'a str,
    pub function_id: &'a str,
}

impl BindingsConfig {
    pub fn iter_task_bindings(&self) -> impl Iterator<Item = TaskBindingRef<'_>> {
        self.uart_source
            .iter()
            .map(|binding| TaskBindingRef {
                task_id: &binding.task_id,
                device_id: &binding.device_id,
                function_id: &binding.function_id,
            })
            .chain(self.timer_source.iter().map(|binding| TaskBindingRef {
                task_id: &binding.task_id,
                device_id: &binding.device_id,
                function_id: &binding.function_id,
            }))
            .chain(self.loop_source.iter().map(|binding| TaskBindingRef {
                task_id: &binding.task_id,
                device_id: &binding.device_id,
                function_id: &binding.function_id,
            }))
            .chain(self.debug_source.iter().map(|binding| TaskBindingRef {
                task_id: &binding.task_id,
                device_id: &binding.device_id,
                function_id: &binding.function_id,
            }))
    }
}
