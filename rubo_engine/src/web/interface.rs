use serde::{Deserialize, Serialize};

use crate::WebConfig;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct WebInterface {
    name: String,
    version: u32,
    routes: Vec<WebRouteInfo>,
    pages: Vec<String>,
}

impl WebInterface {
    pub fn from_config(config: &WebConfig) -> Self {
        let routes = config.routes();
        Self {
            name: "RuboEngine".to_string(),
            version: 1,
            pages: vec![
                "runtime".to_string(),
                "outputs".to_string(),
                "config".to_string(),
            ],
            routes: vec![
                WebRouteInfo::new(
                    "interface",
                    "GET",
                    routes.interface(),
                    "runtime",
                    "API table",
                ),
                WebRouteInfo::new("health", "GET", routes.health(), "runtime", "web health"),
                WebRouteInfo::new(
                    "runtime_summary",
                    "GET",
                    routes.runtime_summary(),
                    "runtime",
                    "runtime summary",
                ),
                WebRouteInfo::new(
                    "runtime_chain",
                    "GET",
                    routes.runtime_chain(),
                    "runtime",
                    "runtime chain",
                ),
                WebRouteInfo::new(
                    "runtime_outputs_latest",
                    "GET",
                    routes.runtime_outputs_latest(),
                    "runtime",
                    "latest runtime outputs",
                ),
                WebRouteInfo::new(
                    "runtime_events",
                    "GET",
                    routes.runtime_events(),
                    "runtime",
                    "runtime SSE events",
                ),
                WebRouteInfo::new(
                    "runtime_start",
                    "POST",
                    routes.runtime_start(),
                    "runtime",
                    "start runtime",
                ),
                WebRouteInfo::new(
                    "runtime_stop",
                    "POST",
                    routes.runtime_stop(),
                    "runtime",
                    "stop runtime",
                ),
                WebRouteInfo::new(
                    "runtime_restart",
                    "POST",
                    routes.runtime_restart(),
                    "runtime",
                    "restart runtime",
                ),
                WebRouteInfo::new(
                    "runtime_control_status",
                    "GET",
                    routes.runtime_control_status(),
                    "runtime",
                    "runtime control status",
                ),
                WebRouteInfo::new(
                    "outputs",
                    "GET",
                    routes.outputs(),
                    "outputs",
                    "output history",
                ),
                WebRouteInfo::new(
                    "outputs_latest",
                    "GET",
                    routes.outputs_latest(),
                    "outputs",
                    "latest outputs",
                ),
                WebRouteInfo::new(
                    "output_detail",
                    "GET",
                    routes.output_detail(),
                    "outputs",
                    "output detail",
                ),
                WebRouteInfo::new("config", "GET", routes.config(), "config", "full config"),
                WebRouteInfo::new(
                    "config_sources",
                    "GET",
                    routes.config_sources(),
                    "config",
                    "source config",
                ),
                WebRouteInfo::new(
                    "config_devices",
                    "GET",
                    routes.config_devices(),
                    "config",
                    "device config",
                ),
                WebRouteInfo::new(
                    "config_functions",
                    "GET",
                    routes.config_functions(),
                    "config",
                    "function config",
                ),
                WebRouteInfo::new(
                    "config_sinks",
                    "GET",
                    routes.config_sinks(),
                    "config",
                    "sink config",
                ),
                WebRouteInfo::new(
                    "config_bindings",
                    "GET",
                    routes.config_bindings(),
                    "config",
                    "binding config",
                ),
                WebRouteInfo::new(
                    "config_profile",
                    "GET",
                    routes.config_profile(),
                    "config",
                    "active and selected config profile",
                ),
                WebRouteInfo::new(
                    "config_profile_update",
                    "PUT",
                    routes.config_profile(),
                    "config",
                    "select config profile for next startup",
                ),
                WebRouteInfo::new(
                    "config_validate",
                    "POST",
                    routes.config_validate(),
                    "config",
                    "validate config",
                ),
                WebRouteInfo::new(
                    "config_save",
                    "POST",
                    routes.config_save(),
                    "config",
                    "save config",
                ),
                WebRouteInfo::new(
                    "debug_bindings",
                    "GET",
                    routes.debug_bindings(),
                    "runtime",
                    "debug bindings",
                ),
                WebRouteInfo::new(
                    "debug_trigger",
                    "POST",
                    routes.debug_trigger(),
                    "runtime",
                    "debug trigger",
                ),
            ],
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn version(&self) -> u32 {
        self.version
    }

    pub fn routes(&self) -> &[WebRouteInfo] {
        &self.routes
    }

    pub fn pages(&self) -> &[String] {
        &self.pages
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct WebRouteInfo {
    name: String,
    method: String,
    path: String,
    page: String,
    description: String,
}

impl WebRouteInfo {
    pub fn new(
        name: impl Into<String>,
        method: impl Into<String>,
        path: impl Into<String>,
        page: impl Into<String>,
        description: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            method: method.into(),
            path: path.into(),
            page: page.into(),
            description: description.into(),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn method(&self) -> &str {
        &self.method
    }

    pub fn path(&self) -> &str {
        &self.path
    }

    pub fn page(&self) -> &str {
        &self.page
    }

    pub fn description(&self) -> &str {
        &self.description
    }
}
