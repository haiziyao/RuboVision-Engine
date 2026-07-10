# rubo_engine

`rubo_engine` is an experimental pure Rust core runtime for building source-device-function-sink pipelines.

Version `0.2.0` is an early public release. APIs may change significantly before `1.0`.

## Minimal Shape

```text
Source -> Message -> Binding -> Function -> Sink
```

- Implement `SourceHandler` to wait for external input and return one `Message`.
- Implement `Function` to process a matched event.
- Implement `Sink` to receive outputs.
- Optional `Device` values are shared through `DeviceHandle<T>`.
- Web Debug Console is enabled by default.

See the repository docs for the full first-use guide.
