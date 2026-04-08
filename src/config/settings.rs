use serde::{Deserialize, Serialize};
use crate::config::{AppConfig, WebConfig, BindingsConfig};
use crate::config::device::{DeviceParamConfig};
use crate::config::func::{FuncParamConfig};

pub fn load_config() -> Result<RuntimeConfig, config::ConfigError> {
    let builder = config::Config::builder()
        .add_source(config::File::with_name("config/app").required(true))
        .add_source(config::File::with_name("config/web").required(true))
        .add_source(config::File::with_name("config/bindings").required(false))
        .add_source(config::File::with_name("config/func_param").required(false))
        .add_source(config::File::with_name("config/device").required(false))
        .add_source(config::Environment::with_prefix("RUBO"));

    let cfg = builder.build()?;
    cfg.try_deserialize()
}



#[derive(Debug,Clone, Deserialize, Serialize)]
pub struct RuntimeConfig{
    pub app: AppConfig,
    pub web: WebConfig,
    pub bindings: BindingsConfig,
    pub func_param_config: FuncParamConfig,
    pub device_param_config: DeviceParamConfig,
}

#[cfg(test)]
#[test]
fn test_load_config() {
    let cfg = load_config().unwrap();
    println!("{:#?}", cfg);
}
