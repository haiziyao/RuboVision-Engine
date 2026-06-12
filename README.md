# RuboVision Engine

RuboVision Engine 是一个配置驱动的 Rust 嵌入式视觉任务运行框架。输入源只产生
`Event`，函数只返回业务结果，Web、UART 和 GPIO 输出由统一 Message 层负责。

```text
UART frame / Web source_key
          |
        Source
          |
         Event
          |
 TaskListener -> TaskDispatcher
          |
   pre_func -> func -> after_func
          |
       TaskOutput
          |
 MessageRouter -> WebSink / UartSink / GpioSink
```

## 当前能力

- UART 固定二进制帧输入，支持拆包、粘包、噪声恢复和缓冲区上限。
- DebugSource Web 触发，复用正常 `Event -> TaskListener` 链路。
- 类型化设备和函数参数，启动时校验重复 key、引用和 GPIO signal。
- 声明式函数注册，无运行时字符串工厂和未知 ID `panic!`。
- Web、UART、GPIO 独立输出，一个 sink 失败不会阻断其他 sink。
- GPIO 作为 Message 输出参与任务开始和结束生命周期。
- Web 消息缓存、JSONL 持久化、历史恢复和嵌入式控制台。

## 配置

程序一次性使用以下新版配置，不兼容已删除的 `web.yaml` 和
`func_param.toml`：

```text
config/
├── app.yaml
├── message.yaml
├── bindings.toml
├── device.toml
└── functions.toml
```

### Message

`config/message.yaml` 同时配置 Web、UART 和 GPIO：

```yaml
message:
  web:
    on: true
    host: 127.0.0.1
    port: 3000
  uart:
    on: true
    serial: /dev/ttyV0
    baud: 9600
    data_bit: 8
    stop_bit: 1
    parity_bit: false
  gpio:
    on: true
    active_low: true
    run_pin: 27
    signals:
      color: 17
      qr: 22
```

Camera 不再持有 UART 或 GPIO，只保存设备信息：

```toml
[[devices.list]]
device_id = "color_camera"
kind = "Camera"
path = "/dev/video2"
```

### Binding

UART binding 使用数字 `source_key`，DebugSource 使用字符串 key：

```toml
[[bindings.uart_source]]
task_id = "uart_color_detect"
source_key = 1
device_id = "color_camera"
function_id = "color_detect"

[[bindings.debug_source]]
task_id = "debug_color_detect"
source_key = "color"
device_id = "color_camera"
function_id = "color_detect"
```

配置加载会拒绝重复 key、未知设备、未知函数和不存在的 GPIO signal。

### Function

`config/functions.toml` 决定参数和输出通道：

```toml
[[functions.entries]]
function_id = "color_detect"
returns = { web = true, uart = true, gpio = "color" }

[functions.entries.params]
debug_model = false
loop_count = 5
radius_ratio = 0.4
detect_area_access_rate = 0.8
color_ranges = [
    { name = "red", hsv = [0, 50, 160, 255, 110, 255] },
]
```

函数返回统一的 `TaskOutput`，不直接操作输出通道：

```rust
pub struct TaskOutput {
    pub code: u16,
    pub text: String,
    pub value: Option<String>,
    pub image: Option<String>,
}
```

`returns.web`、`returns.uart` 和 `returns.gpio` 由 `MessageRouter` 在
`after_func` 阶段处理。

## UART 协议

输入帧固定为四个字节：

```text
[0xAA, CMD, PARAM, 0x55]
```

`PARAM` 是单次任务的运行时参数；不需要参数的命令发送 `0x00`。

| 命令 | 发送字节 | 行为 |
| --- | --- | --- |
| 颜色识别 | `AA 01 00 55` | 分发 `color_detect` binding |
| 二维码识别 | `AA 02 00 55` | 分发 `qr_detect` binding |
| Cross 识别 | `AA 03 PARAM 55` | 分发 `cross_detect` binding并传入参数 |
| 停止任务 | `AA 04 00 55` | 预留，只记录日志 |
| 状态/心跳 | `AA 05 00 55` | 预留，只记录日志 |

UART 是字节流。`UartSource` 使用 `Vec<u8>` 保存 pending 数据：

- HEAD 前的垃圾字节会被丢弃。
- 不足四个字节的数据会保留到下一次读取。
- TAIL 错误时丢弃一个字节并重新同步。
- 多帧会连续解析。
- pending 超过 64 字节会记录警告并清空。

UART 输出由唯一 transport 线程持有串口。`UartSink` 发送函数结果中的
`value`，没有 `value` 时发送 `text`，并统一追加一个换行。

## DebugSource API

Web 控制台默认地址：

```text
http://127.0.0.1:3000
```

接口：

| 方法 | 路径 | 说明 |
| --- | --- | --- |
| `GET` | `/message` | 当前消息 |
| `GET` | `/history` | 最近消息 |
| `GET` | `/debug/bindings` | 可触发 DebugSource binding |
| `POST` | `/debug/trigger` | 按 `source_key` 触发任务 |

请求示例：

```json
{
  "source_key": "color"
}
```

`POST /debug/trigger` 的状态码：

- `202 Accepted`：事件已进入任务队列。
- `404 Not Found`：未知 `source_key`。
- `503 Service Unavailable`：事件队列已经关闭或 DebugSource 不可用。

DebugSource 不直接调用函数，也不操作 GPIO 输出。它根据 binding 生成普通
`UsualEvent`，因此设备查找、函数查找和生命周期与 UART 输入完全相同。

## 函数生命周期

任务执行概念上分为三段：

1. `pre_func`：根据配置取得已校验的 typed params，并发送 GPIO
   `TaskStarted`。
2. `func`：在 `spawn_blocking` 中执行视觉或硬件业务函数。
3. `after_func`：将成功或错误结果路由到 Web/UART，并发送 GPIO
   `TaskFinished`。

即使函数返回错误或 panic，结束阶段仍会执行，GPIO 会恢复到完成状态。
未知 `device_id`、未知 `function_id` 和设备类型不匹配都返回可恢复错误。

## 新增函数

1. 在 `src/config/type.rs` 定义可反序列化的参数结构。
2. 在 `src/func/functions.rs` 实现参数校验和业务函数：

```rust
fn example(
    params: &ExampleParams,
    camera: &CameraDevice,
) -> anyhow::Result<FunctionResult> {
    Ok(FunctionResult::value("example finished", "result"))
}
```

3. 在同一文件的 `declare_functions!` 中声明：

```rust
example(params: ExampleParams, device: CameraDevice) => example,
```

4. 在 `config/functions.toml` 配置同名 `function_id`、typed params 和
   `returns`。
5. 在 `config/bindings.toml` 将 UART 或 DebugSource key 绑定到函数。

这是 Rust 静态类型环境下的显式声明式注册。新增函数不需要修改 match 工厂、
dispatcher、executor 或输出通道代码。

## 运行与测试

环境要求：

- Rust 2024 toolchain。
- OpenCV 开发库。
- 真实运行 UART 时需要对应串口设备权限。
- 真实运行 GPIO 时需要 Raspberry Pi 兼容 GPIO 环境。
- 摄像头和 GUI 测试需要对应硬件与显示环境。
- 虚拟 UART 测试使用 `socat`；缺少时相关测试会跳过测试主体。

```bash
cargo check
cargo test --all-targets
cargo run
```

当前默认配置会尝试打开 `/dev/ttyV0` 和 GPIO。缺少硬件时，transport/sink
会记录不可用错误；需要本地纯 Web 调试时可通过 `RUBO_*` 环境覆盖关闭硬件。

测试中标记为 `ignored` 的项目包括真实摄像头、GUI、长驻 Web 服务和完整运行时。

## 已知保留项

- `cross_detect` 当前仍是占位结果，未实现真实路口识别算法。
- UART `0x04` 和 `0x05` 只作为协议标识保留。
- TimerSource 和 LoopSource 仍是基础实现。
- 远期通用 SDK 设想记录在 `changeTask/toSDK.md`，本轮不实施。

本轮分层重构的设计、计划和逐阶段 Git 回档点见：

- `docs/superpowers/specs/2026-06-07-layered-engine-refactor-design.md`
- `docs/superpowers/plans/2026-06-07-layered-engine-refactor.md`
- `changeTask/completed.md`
