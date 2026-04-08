use serde::{Deserialize, Serialize};


// 这个的属性的命名也是难绷。。。以后再改吧
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(default)]
pub struct BindingsConfig {
    pub uart_source: Vec<UartBinding>,
    pub timer_source: Vec<TimerBinding>,
    pub loop_source: Vec<LoopBinding>,
    pub web_source: Vec<WebBinding>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UartBinding {
    pub task_id: String,
    pub source_key: String,
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
pub struct WebBinding {
    pub task_id: String,
    pub source_key: String,
    pub device_id: String,
    pub function_id: String,
}