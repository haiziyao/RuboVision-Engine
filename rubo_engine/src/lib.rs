pub mod config;
pub mod device;
pub mod dispatcher;
pub mod engine;
pub mod err;
pub mod executor;
pub mod function;
pub mod log;
pub mod output;
pub mod registry;
pub mod runtime;
pub mod sink;
pub mod source;
pub mod web;

pub use device::{Device, DevicePool, DeviceRef, DeviceRegister, MutexDevice, build_device_pool};
pub use dispatcher::{DispatchMessage, DispatchOutput, TaskRequest, dispatch};
pub use engine::{Engine, EngineRuntime, EngineRuntimeHandle};
pub use err::{
    ConfigError, DeviceError, DispatchError, DispatchErrorKind, FunctionError, RuntimeError,
    SinkError, SourceError,
};
pub use executor::execute;
pub use function::{
    FuncResult, Function, FunctionAspect, FunctionCall, FunctionDevices, FunctionRegister,
};
#[doc(hidden)]
pub use inventory;
pub use output::{Output, OutputError, OutputErrorKind, OutputRoute, OutputState, OutputTiming};
pub use registry::{
    DeviceInventoryRegistration, FunctionInventoryRegistration, SinkInventoryRegistration,
    SourceInventoryRegistration, register_device_type, register_function_type, register_inventory,
    register_sink_type, register_source_factory,
};
pub use rubo_engine_macros::{device, function, sink, source};
pub use runtime::{
    RuntimeOutput, RuntimeResources, SourceRunOutput, handle_message, run_config,
    run_config_sources, run_config_sources_with_resources, run_config_with_resources, run_source,
    run_source_messages,
};
pub use sink::{
    ChannelSink, ChannelSinkFactory, Sink, SinkFactory, SinkRegister, SinkRouteResult,
    SinkRouteState, UartSink, UartSinkFactory, route_output,
};
pub use source::{
    ChannelSource, ChannelSourceFactory, IntervalSource, IntervalSourceFactory, ManualSource,
    ManualSourceFactory, Message, Source, SourceFactory, SourceHandler, SourceRegister, UartSource,
    UartSourceFactory,
};
pub use web::{
    WEB_SINK_ID, WEB_SOURCE_ID, WebConfig, WebError, WebErrorKind, WebEvent, WebEventKind,
    WebHistory, WebHub, WebInterface, WebOutputFrame, WebOutputRoute, WebOutputState,
    WebOutputTiming, WebResponse, WebRoutes, WebRuntimeCommand, WebRuntimeCommandKind,
    WebRuntimeControl, WebSink, WebSinkRouteResult, WebSource, WebState, build_router, serve,
};
