use serde::{Deserialize,Serialize};



#[derive(Debug,Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct AppConfig{
    name: String,
    profile: String,
    log_level: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        AppConfig{
            name: "rubovision".to_string(),
            profile: "dev".to_string(),
            log_level: "info".to_string(),
        }
    }
}