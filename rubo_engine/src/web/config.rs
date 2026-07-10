use serde::{Deserialize, Serialize};

use crate::config::AppWebConfig;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(default)]
pub struct WebConfig {
    enabled: bool,
    host: String,
    port: u16,
    history_limit: usize,
    routes: WebRoutes,
}

impl Default for WebConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            host: "127.0.0.1".to_string(),
            port: 3888,
            history_limit: 500,
            routes: WebRoutes::default(),
        }
    }
}

impl WebConfig {
    pub fn from_app_web(config: &AppWebConfig) -> Self {
        Self {
            enabled: config.enabled(),
            host: config.host().to_string(),
            port: config.port(),
            ..Self::default()
        }
    }

    pub fn enabled(&self) -> bool {
        self.enabled
    }

    pub fn host(&self) -> &str {
        &self.host
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub fn history_limit(&self) -> usize {
        self.history_limit
    }

    pub fn routes(&self) -> &WebRoutes {
        &self.routes
    }

    pub fn address(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(default)]
pub struct WebRoutes {
    interface: String,
    health: String,
    runtime_summary: String,
    runtime_chain: String,
    runtime_outputs_latest: String,
    runtime_events: String,
    runtime_start: String,
    runtime_stop: String,
    runtime_restart: String,
    runtime_control_status: String,
    outputs: String,
    outputs_latest: String,
    output_detail: String,
    config: String,
    config_sources: String,
    config_devices: String,
    config_functions: String,
    config_sinks: String,
    config_bindings: String,
    config_validate: String,
    config_save: String,
    debug_bindings: String,
    debug_trigger: String,
}

impl Default for WebRoutes {
    fn default() -> Self {
        Self {
            interface: "/api/interface".to_string(),
            health: "/api/health".to_string(),
            runtime_summary: "/api/runtime/summary".to_string(),
            runtime_chain: "/api/runtime/chain".to_string(),
            runtime_outputs_latest: "/api/runtime/outputs/latest".to_string(),
            runtime_events: "/api/runtime/events".to_string(),
            runtime_start: "/api/runtime/start".to_string(),
            runtime_stop: "/api/runtime/stop".to_string(),
            runtime_restart: "/api/runtime/restart".to_string(),
            runtime_control_status: "/api/runtime/control/status".to_string(),
            outputs: "/api/outputs".to_string(),
            outputs_latest: "/api/outputs/latest".to_string(),
            output_detail: "/api/outputs/{id}".to_string(),
            config: "/api/config".to_string(),
            config_sources: "/api/config/sources".to_string(),
            config_devices: "/api/config/devices".to_string(),
            config_functions: "/api/config/functions".to_string(),
            config_sinks: "/api/config/sinks".to_string(),
            config_bindings: "/api/config/bindings".to_string(),
            config_validate: "/api/config/validate".to_string(),
            config_save: "/api/config/save".to_string(),
            debug_bindings: "/api/debug/bindings".to_string(),
            debug_trigger: "/api/debug/trigger".to_string(),
        }
    }
}

impl WebRoutes {
    pub fn interface(&self) -> &str {
        &self.interface
    }

    pub fn health(&self) -> &str {
        &self.health
    }

    pub fn runtime_summary(&self) -> &str {
        &self.runtime_summary
    }

    pub fn runtime_chain(&self) -> &str {
        &self.runtime_chain
    }

    pub fn runtime_outputs_latest(&self) -> &str {
        &self.runtime_outputs_latest
    }

    pub fn runtime_events(&self) -> &str {
        &self.runtime_events
    }

    pub fn runtime_start(&self) -> &str {
        &self.runtime_start
    }

    pub fn runtime_stop(&self) -> &str {
        &self.runtime_stop
    }

    pub fn runtime_restart(&self) -> &str {
        &self.runtime_restart
    }

    pub fn runtime_control_status(&self) -> &str {
        &self.runtime_control_status
    }

    pub fn outputs(&self) -> &str {
        &self.outputs
    }

    pub fn outputs_latest(&self) -> &str {
        &self.outputs_latest
    }

    pub fn output_detail(&self) -> &str {
        &self.output_detail
    }

    pub fn config(&self) -> &str {
        &self.config
    }

    pub fn config_sources(&self) -> &str {
        &self.config_sources
    }

    pub fn config_devices(&self) -> &str {
        &self.config_devices
    }

    pub fn config_functions(&self) -> &str {
        &self.config_functions
    }

    pub fn config_sinks(&self) -> &str {
        &self.config_sinks
    }

    pub fn config_bindings(&self) -> &str {
        &self.config_bindings
    }

    pub fn config_validate(&self) -> &str {
        &self.config_validate
    }

    pub fn config_save(&self) -> &str {
        &self.config_save
    }

    pub fn debug_bindings(&self) -> &str {
        &self.debug_bindings
    }

    pub fn debug_trigger(&self) -> &str {
        &self.debug_trigger
    }
}
