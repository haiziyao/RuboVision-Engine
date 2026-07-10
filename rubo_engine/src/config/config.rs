use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::traits::{BindingConfig, DeviceConfig, FuncConfig, SinkConfig, SourceConfig};

#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq)]
#[serde(default)]
pub struct RuboConfig {
    sources: HashMap<String, SourceConfig>,
    devices: HashMap<String, DeviceConfig>,
    funcs: HashMap<String, FuncConfig>,
    sinks: HashMap<String, SinkConfig>,
    bindings: HashMap<String, BindingConfig>,
}

impl RuboConfig {
    pub fn sources(&self) -> &HashMap<String, SourceConfig> {
        &self.sources
    }

    pub fn sources_mut(&mut self) -> &mut HashMap<String, SourceConfig> {
        &mut self.sources
    }

    pub fn devices(&self) -> &HashMap<String, DeviceConfig> {
        &self.devices
    }

    pub fn devices_mut(&mut self) -> &mut HashMap<String, DeviceConfig> {
        &mut self.devices
    }

    pub fn funcs(&self) -> &HashMap<String, FuncConfig> {
        &self.funcs
    }

    pub fn funcs_mut(&mut self) -> &mut HashMap<String, FuncConfig> {
        &mut self.funcs
    }

    pub fn sinks(&self) -> &HashMap<String, SinkConfig> {
        &self.sinks
    }

    pub fn sinks_mut(&mut self) -> &mut HashMap<String, SinkConfig> {
        &mut self.sinks
    }

    pub fn bindings(&self) -> &HashMap<String, BindingConfig> {
        &self.bindings
    }

    pub fn bindings_mut(&mut self) -> &mut HashMap<String, BindingConfig> {
        &mut self.bindings
    }

    pub fn validate(&self) -> bool {
        self.bindings.values().all(|binding| {
            !binding.id().is_empty()
                && !binding.source_ref().id().is_empty()
                && !binding.source_ref().event().is_empty()
                && !binding.func_ref().is_empty()
                && self.sources.contains_key(binding.source_ref().id())
                && self.funcs.contains_key(binding.func_ref())
                && binding
                    .devices()
                    .iter()
                    .all(|device| self.devices.contains_key(device))
                && binding
                    .sinks()
                    .iter()
                    .all(|sink| self.sinks.contains_key(sink))
        })
    }
}
