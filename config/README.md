# 开发板配置

`config/application.yaml` 保存应用配置，开发板对应的 Source、Device、Function、Sink 和 Binding 配置分别保存在 profile 子目录中。

```text
config/
  application.yaml
  ubuntu/
    source.toml
    device.toml
    function.toml
    sink.toml
    binding.toml
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

Ubuntu：

```yaml
config_path: config
profile: ubuntu
```

Orange Pi：

```yaml
config_path: config
profile: orangepi
```

Raspberry Pi：

```yaml
config_path: config
profile: raspberrypi
```

修改 `profile` 后需要重启程序。Web 配置管理页可以查看当前运行平台并保存下次启动平台。

`source.toml` 和 `sink.toml` 保存 UART 路径，`device.toml` 保存 Camera 路径。Ubuntu profile 使用本机两个 USB 端口的稳定 V4L2 路径，并使用 `/tmp/rubo-uart` 作为开发串口路径。GPIO 状态灯由 RuboVision 的 `GpioDevice` FunctionAspect 实现；它当前从 `sink.toml` 读取配置。`chip = N` 对应 `/dev/gpiochipN`，`run_pin` 和 `signals` 是该芯片内部的 line 编号。Ubuntu profile 不声明 GPIO。
