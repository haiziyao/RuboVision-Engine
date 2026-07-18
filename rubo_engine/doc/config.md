# Config

Rubo Engine keeps application settings and pipeline settings separate.

## Application config

`application.json`, `application.toml`, or `application.yaml` is loaded with
`ConfigStore::load_app_config(...)`.

```yaml
name: rubo_vision
config_path: config
profile: orangepi
config_format: toml

web:
  enabled: true
  output_image: true
  host: 0.0.0.0
  port: 3888

log:
  enabled: true
  level: info
```

- `config_path` is the base directory for pipeline config.
- `profile` selects one directory below `config_path`.
- An empty `profile` keeps the original single-directory layout.
- `config_format` controls generated pipeline files and supports `json`, `toml`, and `yaml`.
- `web.output_image` tells visual Functions whether a Web-bound result needs image data. It
  defaults to `true`. When Web is disabled, this field is `false`, or a Binding has no `web` Sink,
  `FunctionCall::image_enabled()` returns `false` so the Function can skip image encoding.

## Profile layout

```text
config/
  application.yaml
  orangepi/
    source.toml
    device.toml
    function.toml
    sink.toml
    binding.toml
  raspberrypi/
    source.toml
    device.toml
    function.toml
    sink.toml
    binding.toml
```

For `config_path: config` and `profile: orangepi`, `AppConfig::config_dir()` returns
`config/orangepi`.

## Startup

Application code builds a declared `RuboConfig`, then loads or creates the selected profile:

```rust
let app_config = ConfigStore::load_app_config(root.join("config"))?;
let declared_config = build_config(&app_config)?;
let config = ConfigStore::load_or_init_config(root, &app_config, &declared_config)?;
```

`load_or_init_config` behaves as follows:

1. If the selected profile has no pipeline files, it writes the declared config there.
2. If file config equals declared config, it uses the file config.
3. If parsing fails, it writes the declared config to `<profile>/code_config/` and returns the declared config so Web can still start.
4. If both configs are valid but different, it writes the same snapshot and returns `ConfigError::ConfigMismatch`.

Every successful generation or Web save also updates the root `chain.json`.

## Registration macros

`#[source]`, `#[device]`, `#[function]`, and `#[sink]` register runtime implementations.
They do not create Source, Device, Function, Sink, or Binding config instances. Concrete IDs,
parameters, platform paths, and bindings remain in the application's declared `RuboConfig`.

Unknown macro arguments and attributes placed on non-type items are compile errors.

## UART source config

```toml
[uart]
kind = "uart"
serial = "/dev/ttyAMA1"
baud = 9600
data_bit = 8
stop_bit = 1
parity_bit = false
prefix = [170]
suffix = [85]
content_bytes = 1
```

The UART source matches `prefix + content + suffix`, then converts only `content` into
`Message.key`. See [UART Message Protocol](message.md).

GPIO status behavior is application-specific. Rubo Engine provides `FunctionAspect`, but does not
provide or automatically register a GPIO implementation.
