# rubo_engine

`rubo_engine` is an experimental pure Rust core runtime for building source-device-function-sink pipelines.

Version `0.3.0` is an early public release. APIs may change significantly before `1.0`.

## Minimal Shape

```text
Source -> Message -> Binding -> Function -> Sink
```

- Implement `SourceHandler` to wait for external input and return one `Message`.
- Implement `Function` to process a matched event.
- Implement `Sink` to receive outputs.
- Optional `Device` values are shared through `DeviceHandle<T>`.
- Web Debug Console is enabled by default.
- `FunctionAspect` can run application-defined behavior before and after each function call.
- The optional `hardware` feature provides the built-in UART source and sink implementation.
- Registration macros register runtime types; application code still declares concrete config instances and bindings.

See [Config](doc/config.md) for config loading and profile directories, and
[UART Message Protocol](doc/message.md) for serial framing.
