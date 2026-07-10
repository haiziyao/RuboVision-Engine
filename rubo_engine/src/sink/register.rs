use std::{collections::HashMap, sync::Arc};

use crate::{
    ChannelSink, RuntimeResources, Sink, SinkError,
    config::{RuboConfig, SinkConfig},
};

#[derive(Default)]
pub struct SinkRegister {
    sinks: HashMap<String, Arc<dyn Sink>>,
    factories: HashMap<String, Box<dyn SinkFactory>>,
}

impl SinkRegister {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register<T>(&mut self, id: impl Into<String>, sink: T)
    where
        T: Sink,
    {
        self.sinks.insert(id.into(), Arc::new(sink));
    }

    pub fn register_factory(&mut self, kind: impl Into<String>, factory: impl SinkFactory) {
        self.factories.insert(kind.into(), Box::new(factory));
    }

    pub fn contains(&self, id: &str) -> bool {
        self.sinks.contains_key(id)
    }

    pub fn contains_kind(&self, kind: &str) -> bool {
        self.factories.contains_key(kind)
    }

    pub fn get(&self, id: &str) -> Option<Arc<dyn Sink>> {
        self.sinks.get(id).cloned()
    }

    pub fn register_with_resources(
        &mut self,
        id: impl Into<String>,
        factory: impl SinkFactory,
        resources: &mut RuntimeResources,
    ) -> Result<(), SinkError> {
        let id = id.into();
        let config = SinkConfig::new(&id);
        let sink = factory.build(&config, resources)?;
        self.sinks.insert(id, sink);
        Ok(())
    }

    pub fn register_config_sinks_with_resources(
        &mut self,
        config: &RuboConfig,
        resources: &mut RuntimeResources,
    ) -> Result<(), SinkError> {
        for sink_config in config.sinks().values() {
            self.register_config_sink_with_resources(sink_config, resources)?;
        }
        Ok(())
    }

    pub fn register_config_sink_with_resources(
        &mut self,
        sink_config: &SinkConfig,
        resources: &mut RuntimeResources,
    ) -> Result<(), SinkError> {
        if sink_config.kind_ref().is_empty() {
            return Err(SinkError::KindMissing {
                id: sink_config.id().to_string(),
            });
        }
        let factory = self.factories.get(sink_config.kind_ref()).ok_or_else(|| {
            SinkError::KindNotRegistered {
                kind: sink_config.kind_ref().to_string(),
            }
        })?;
        let sink = factory.build(sink_config, resources)?;
        self.sinks.insert(sink_config.id().to_string(), sink);
        Ok(())
    }
}

pub trait SinkFactory: Send + Sync + 'static {
    fn build(
        &self,
        config: &SinkConfig,
        resources: &mut RuntimeResources,
    ) -> Result<Arc<dyn Sink>, SinkError>;
}

pub struct ChannelSinkFactory;

impl SinkFactory for ChannelSinkFactory {
    fn build(
        &self,
        config: &SinkConfig,
        resources: &mut RuntimeResources,
    ) -> Result<Arc<dyn Sink>, SinkError> {
        let sender =
            resources
                .get_sink_channel(config.id())
                .ok_or_else(|| SinkError::ResourceMissing {
                    id: config.id().to_string(),
                })?;
        Ok(Arc::new(ChannelSink::new(sender)))
    }
}
