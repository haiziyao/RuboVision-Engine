use serde::{Deserialize,Serialize};



#[derive(Debug,Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct WebConfig{
    pub on: bool,
    pub host: String,
    pub port: u16,
}

impl Default for WebConfig {
    fn default() -> Self {
        WebConfig{
            on: true,
            host: "127.0.0.1".to_string(),
            port: 3000,
        }
    }
}