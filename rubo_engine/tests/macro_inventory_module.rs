use async_trait::async_trait;
use rubo_engine::{
    Device, DeviceError, DeviceRegister, FuncResult, Function, FunctionCall, FunctionError,
    FunctionRegister, Sink, SinkError, SinkRegister, SourceError, SourceFactory, SourceHandler,
    SourceRegister,
    config::{DeviceConfig, SinkConfig, SourceConfig},
    device, function, sink, source,
};
use serde_json::json;

#[source(kind = "macro_source")]
#[derive(Default)]
struct MacroSourceFactory;

impl SourceFactory for MacroSourceFactory {
    fn build(&self, _config: &SourceConfig) -> Result<Box<dyn SourceHandler>, SourceError> {
        Err(SourceError::SourceHandle {
            message: "not used".to_string(),
        })
    }
}

#[device(kind = "macro_device")]
struct MacroDevice;

#[async_trait]
impl Device for MacroDevice {
    async fn create(_config: &DeviceConfig) -> Result<Self, DeviceError> {
        Ok(Self)
    }
}

#[function(id = "macro_function")]
#[derive(Default)]
struct MacroFunction;

#[async_trait]
impl Function for MacroFunction {
    async fn call(&self, _function_call: FunctionCall<'_>) -> Result<FuncResult, FunctionError> {
        Ok(FuncResult::new(json!({ "ok": true })))
    }
}

#[sink(id = "macro_sink")]
#[derive(Default)]
struct MacroSink;

#[async_trait]
impl Sink for MacroSink {
    async fn handle(
        &self,
        _output: &rubo_engine::Output,
        _sink_config: &SinkConfig,
    ) -> Result<(), SinkError> {
        Ok(())
    }
}

#[test]
fn attribute_macros_register_items_into_inventory() {
    let mut sources = SourceRegister::new();
    let mut devices = DeviceRegister::new();
    let mut functions = FunctionRegister::new();
    let mut sinks = SinkRegister::new();

    rubo_engine::register_inventory(&mut sources, &mut devices, &mut functions, &mut sinks);

    assert!(sources.contains_kind("macro_source"));
    assert!(devices.contains_kind("macro_device"));
    assert!(functions.contains("macro_function"));
    assert!(sinks.contains("macro_sink"));
}
