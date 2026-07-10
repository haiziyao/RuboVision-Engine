use std::{collections::HashMap, time::Duration};

use crate::{
    ChannelSource, IntervalSource, ManualSource, RuntimeResources, SourceError, SourceHandler,
    config::{ConfigAccess, SourceConfig},
};

#[derive(Default)]
pub struct SourceRegister {
    factories: HashMap<String, Box<dyn SourceFactory>>,
}

impl SourceRegister {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, kind: impl Into<String>, factory: impl SourceFactory) {
        self.factories.insert(kind.into(), Box::new(factory));
    }

    pub fn contains_kind(&self, kind: &str) -> bool {
        self.factories.contains_key(kind)
    }

    pub fn build(&self, config: &SourceConfig) -> Result<Box<dyn SourceHandler>, SourceError> {
        if config.kind_ref().is_empty() {
            return Err(SourceError::KindMissing {
                id: config.id().to_string(),
            });
        }
        let factory = self.factories.get(config.kind_ref()).ok_or_else(|| {
            SourceError::KindNotRegistered {
                kind: config.kind_ref().to_string(),
            }
        })?;
        factory.build(config)
    }

    pub fn build_with_resources(
        &self,
        config: &SourceConfig,
        resources: &mut RuntimeResources,
    ) -> Result<Box<dyn SourceHandler>, SourceError> {
        if config.kind_ref().is_empty() {
            return Err(SourceError::KindMissing {
                id: config.id().to_string(),
            });
        }
        let factory = self.factories.get(config.kind_ref()).ok_or_else(|| {
            SourceError::KindNotRegistered {
                kind: config.kind_ref().to_string(),
            }
        })?;
        factory.build_with_resources(config, resources)
    }
}

pub trait SourceFactory: Send + Sync + 'static {
    fn build(&self, config: &SourceConfig) -> Result<Box<dyn SourceHandler>, SourceError>;

    fn build_with_resources(
        &self,
        config: &SourceConfig,
        _resources: &mut RuntimeResources,
    ) -> Result<Box<dyn SourceHandler>, SourceError> {
        self.build(config)
    }
}

pub struct ManualSourceFactory;

impl SourceFactory for ManualSourceFactory {
    fn build(&self, _config: &SourceConfig) -> Result<Box<dyn SourceHandler>, SourceError> {
        Ok(Box::new(ManualSource::new()))
    }
}

pub struct IntervalSourceFactory;

impl SourceFactory for IntervalSourceFactory {
    fn build(&self, config: &SourceConfig) -> Result<Box<dyn SourceHandler>, SourceError> {
        let key = config.get::<String>("key")?;
        let interval_ms = config.get::<u64>("interval_ms")?;
        if interval_ms == 0 {
            return Err(SourceError::Config {
                message: "interval_ms must be greater than zero".to_string(),
            });
        }
        Ok(Box::new(IntervalSource::new(
            key,
            Duration::from_millis(interval_ms),
        )))
    }
}

pub struct ChannelSourceFactory;

impl SourceFactory for ChannelSourceFactory {
    fn build(&self, config: &SourceConfig) -> Result<Box<dyn SourceHandler>, SourceError> {
        Err(SourceError::ResourceMissing {
            id: config.id().to_string(),
        })
    }

    fn build_with_resources(
        &self,
        config: &SourceConfig,
        resources: &mut RuntimeResources,
    ) -> Result<Box<dyn SourceHandler>, SourceError> {
        let receiver = resources.take_source_channel(config.id()).ok_or_else(|| {
            SourceError::ResourceMissing {
                id: config.id().to_string(),
            }
        })?;
        Ok(Box::new(ChannelSource::new(receiver)))
    }
}
