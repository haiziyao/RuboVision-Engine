# RuboVision Engine 分层渐进重构设计

**日期：** 2026-06-07

**状态：** 已由用户逐节确认

**范围：** `changeTask/CHANGETASK.md` 与 `changeTask/tobeChanged.md` 中当前任务

**暂不实施：** `changeTask/toSDK.md`

## 1. 目标

本次重构把当前配置、输入源、函数执行和结果返回之间的耦合拆开，同时保持现有颜色识别、二维码识别、路口识别占位、Web 消息历史和 GPIO 指示灯功能可用。

最终使用方式为：

1. 在 `src/func/functions.rs` 编写业务函数。
2. 在同文件的声明宏中登记函数和参数类型。
3. 在配置中声明设备、强类型参数、触发 binding 与返回目标。
4. 框架自动完成事件分发、参数准备、阻塞任务执行和 Web/UART/GPIO 返回。

本次采用分层渐进重构。每一阶段都保持可编译、可测试，并创建独立 Git 提交，方便单独回档。

## 2. 已确认的设计决策

- 配置采用一次性切换，不保留旧格式兼容层。
- 函数参数从 `Vec<String>` 改为强类型结构体。
- 函数发现采用声明宏，不新增过程宏 crate，也不依赖运行时反射。
- 业务函数只返回业务结果，返回目标由配置决定。
- UART、Web 和 GPIO 都视为消息输出能力。
- DebugSource 从 Web 接收 `source_key`，通过 binding 生成普通任务事件。
- UART `0x04` 停止命令和 `0x05` 状态命令本次仅识别并记录日志，不实现取消或状态业务。
- 前端最后实施，保留现有功能并增加 DebugSource 控制区。

## 3. 总体架构

```text
UART frame / Web debug source_key
                |
                v
             Source
                |
              Event
                |
         TaskListener/Dispatcher
                |
         FunctionFactory/Worker
                |
       pre_func -> func -> after_func
                         |
                   FunctionResult
                         |
                    MessageRouter
                  /       |       \
               Web      UART      GPIO
```

输入侧只负责把外部信号转换为 `Event`。业务函数只负责计算 `FunctionResult`。输出侧由 `after_func` 和 `MessageRouter` 根据配置统一处理。

## 4. 强类型配置

### 4.1 文件布局

新增 `src/config/type.rs`，集中保存跨模块使用的运行时配置类型。现有按主题拆分的配置文件可以继续保留反序列化辅助实现，但对外统一导出强类型结构。

配置文件调整为：

```text
config/app.yaml
config/message.yaml
config/bindings.toml
config/device.toml
config/functions.toml
```

- 删除 `config/web.yaml`。
- 删除 `config/func_param.toml`。
- `message.yaml` 配置 Web、UART 和 GPIO 消息能力。
- `device.toml` 只配置摄像头等业务设备，不再持有 UART。
- `functions.toml` 使用 TOML 原生字段和数组表达强类型参数，不再使用 `key=value` 字符串。

### 4.2 运行时结构

`RuntimeConfig` 统一包含：

```rust
pub struct RuntimeConfig {
    pub app: AppConfig,
    pub message: MessageConfig,
    pub bindings: BindingsConfig,
    pub devices: DeviceConfig,
    pub functions: FunctionConfig,
}
```

函数配置包含函数 ID、参数数据和返回目标。具体参数类型由声明宏中的注册项决定，并在注册阶段反序列化：

```rust
pub struct FunctionEntryConfig {
    pub function_id: String,
    pub returns: ReturnTargets,
    pub params: toml::Value,
}

pub struct ReturnTargets {
    pub web: bool,
    pub uart: bool,
    pub gpio: Option<String>,
}
```

`gpio` 保存具名灯光信号，例如 `"color"` 或 `"qr"`；缺省表示不发送 GPIO 消息。这样新增任务灯不需要继续扩展布尔字段或在函数 ID 上写死判断。

业务参数使用独立结构体，例如：

```rust
pub struct ColorDetectParams {
    pub debug_mode: bool,
    pub loop_count: i32,
    pub radius_ratio: f64,
    pub detect_area_access_rate: f64,
    pub color_ranges: Vec<ColorRangeConfig>,
}
```

配置加载后立即验证必要字段、数值范围、重复 ID、未知函数、未知设备和 binding 引用。错误通过 `Result` 返回，避免在运行任务时 `panic!`。

## 5. 输入 Source

### 5.1 UART 二进制帧

UART 固定使用三字节帧：

```text
[0xAA, CMD, 0x55]
```

命令编号：

| CMD | 含义 | 本次行为 |
| --- | --- | --- |
| `0x01` | 颜色识别 | 根据 binding 分发 |
| `0x02` | 二维码识别 | 根据 binding 分发 |
| `0x03` | 十字/路口识别 | 根据 binding 分发 |
| `0x04` | 停止当前任务 | 记录预留日志 |
| `0x05` | 状态查询/心跳 | 记录预留日志 |

`UartBinding.source_key` 一次性改为 `u8`。默认配置使用十进制数字 `1`、`2`、`3`，日志以十六进制显示。

UART 是字节流，`UartSource` 使用 `Vec<u8>` 保存 pending 数据：

1. 读取后直接 `extend_from_slice`。
2. 查找首个 `0xAA`，丢弃其前方噪声。
3. 数据不足三字节时保留并等待下次读取。
4. 尾字节不是 `0x55` 时只丢弃一个字节，重新同步。
5. 合法帧取出 CMD，删除三字节并继续解析后续帧。
6. pending 超过 64 字节时警告并清空。

解析器拆成可直接单元测试的同步组件，异步 Source 只负责读取 UART 和发送 Event。测试覆盖拆包、粘包、噪声、错误尾字节、未知命令和缓冲区上限。

发送示例：

```text
颜色识别：AA 01 55
二维码识别：AA 02 55
路口识别：AA 03 55
停止预留：AA 04 55
状态预留：AA 05 55
```

### 5.2 共享 UART transport

UART 不再属于 `Device::Camera`。启动时根据 `message.yaml` 创建进程级 UART transport，并由输入和输出共同持有。

transport 启动一个阻塞线程并独占唯一的 `rppal::uart::Uart` 句柄：

- 线程循环读取字节，并通过 Tokio channel 的 `blocking_send` 交给 `UartSource`。
- `UartSink` 通过独立写入 channel 提交待发送字节。
- transport 线程每轮先处理待写数据，再执行带超时的 UART 读取。
- transport 关闭或串口发生不可恢复错误时关闭两个 channel，并记录明确日志。

这样 UartSource 和 UartSink 不直接访问串口句柄，也不会分别重复打开同一串口。

### 5.3 DebugSource

DebugSource 是开发环境中的虚拟输入源，不直接调用函数，也不绕过 binding。

Web 后端提供：

```text
GET  /debug/bindings
POST /debug/trigger
```

触发请求：

```json
{
  "source_key": "color"
}
```

DebugSource 根据 debug binding 查找该 key，并生成与其他 Source 相同的 `Event`。未知 key 返回明确的 404/校验错误；事件队列关闭返回服务不可用错误。

这里的“劫持 GPIO sender”解释为：DebugSource 复用原本用于外部输入信号的 Event sender，从 Web 模拟一个 `source_key`，而不是直接操作 GPIO 输出引脚。

## 6. Message 输出层

### 6.1 接口

所有输出实现统一泛型接口：

```rust
pub trait MessageSink<M>: Send + Sync {
    fn send(&self, message: M) -> impl Future<Output = anyhow::Result<()>> + Send;
}
```

消息使用强类型结构：

```rust
pub struct WebOutput { /* text, image, status */ }
pub struct UartOutput { /* response bytes/text */ }
pub enum GpioOutput { /* task state */ }
```

`MessageRouter` 直接持有具体的 `WebSink`、`UartSink` 和 `GpioSink`，不把带异步方法的 trait 做成 trait object。它根据 `ReturnTargets` 并发路由。某个 sink 失败不会阻止其他 sink 收到结果；所有失败会汇总写入日志，并在可用时通过 Web 表达。

### 6.2 WebMessage

保留现有 `id`、`created_at_ms`、`code`、`text`、`image`、历史缓存和 JSONL 持久化行为。Function 只返回中立业务结果，由 Web sink 转换成 `WebMessage`。

### 6.3 UartMessage

UART sink 使用共享 UART transport 返回业务结果。第一阶段保持现有结果文本加换行的发送格式，避免在未定义响应帧协议时擅自创造协议。输入命令使用新的三字节二进制帧，输出协议在后续 SDK/协议任务中单独设计。

### 6.4 GpioMessage

GPIO 状态灯作为消息输出：

```rust
pub enum GpioOutput {
    TaskStarted(String),
    TaskFinished(String),
    Reset,
}
```

`message.yaml` 保存运行灯引脚、active-low 设置，以及信号名到任务灯引脚的映射。GPIO sink 负责引脚访问和恢复。业务函数与视觉配置不再直接调用 `Gpio::new()`。即使函数失败，`after_func` 也发送结束或复位消息。

## 7. Function 生命周期

### 7.1 声明宏注册

`src/func/functions.rs` 是业务函数和注册声明的唯一入口：

```rust
declare_functions! {
    color_detect(params: ColorDetectParams, device: CameraDevice) => color_detect,
    qr_detect(params: QrDetectParams, device: CameraDevice) => qr_detect,
    cross_detect(params: CrossDetectParams, device: CameraDevice) => cross_detect,
    debug_fun(params: DebugParams, device: NoDevice) => debug_fun,
}
```

声明宏生成静态注册描述，`FunctionFactory` 用它把配置中的 `function_id` 映射到参数解析器和业务函数。设备参数类型实现内部 `FromDevice` trait，宏生成的适配器在 `pre_func` 阶段完成 `Device` 枚举到具体设备引用的校验和转换。

Rust 稳定版没有运行时扫描普通函数的反射能力。声明宏方案仍要求新增函数时在宏中增加一项，但不需要同时修改另一个 `match` 工厂。过程宏和链接器分布式注册不属于本次范围。

### 7.2 三阶段执行

统一生命周期：

```text
pre_func -> func -> after_func
```

`pre_func`：

- 读取注册时已经验证的强类型参数。
- 验证 Event 所选设备类型。
- 当 `returns.gpio` 存在时发送对应信号的 GPIO `TaskStarted`。
- 构造业务函数所需上下文。

`func`：

- 只执行颜色、二维码、路口或调试业务。
- 不知道 Web/UART/GPIO sender。
- 返回 `anyhow::Result<FunctionResult>`。

`after_func`：

- 无论成功或失败都会运行。
- 发送 GPIO `TaskFinished` 或 `Reset`。
- 根据 `ReturnTargets` 将结果发送到 Web、UART 和 GPIO。
- 统一格式化错误，避免每个函数重复 `into_web_message` 和串口错误处理。

### 7.3 结果类型

```rust
pub struct FunctionResult {
    pub text: String,
    pub value: Option<String>,
    pub image: Option<String>,
}
```

`value` 用于 UART 等机器消费渠道，`text` 用于人类可读渠道。颜色识别可返回 `value = "red"`，二维码可返回数字字符串。图片保持可选。

### 7.4 设备边界

Camera 只保存摄像头路径等设备信息，不再持有 UART 或 GPIO。未知 `device_id`、未知 `function_id` 和设备类型不匹配都改为可恢复错误，进入统一 `after_func`。

现有 `src/func/usual.rs` 中业务函数迁移到 `functions.rs`。参数转换、生命周期和路由逻辑留在框架模块中，确保 `functions.rs` 只包含参数结构、业务函数和声明宏调用。

## 8. Web 调试面板

前端最后实施，并使用浏览器可视化辅助确认布局。页面保留：

- 当前消息展示。
- 图片展示。
- 最近历史。
- 自动轮询、手动刷新和本地清空。
- 请求、空消息、无效消息和历史同步计数。

新增任务调试区：

- 从 `GET /debug/bindings` 读取可触发项。
- 每项显示 source key、任务、设备和函数。
- 点击后 POST `/debug/trigger`。
- 展示成功、未知 key、队列关闭和网络错误。

页面继续使用仓库内嵌静态资源，不新增前端构建链。重构重点是信息层级、响应式布局、可访问状态和调试操作清晰度。

## 9. 错误处理

- 配置错误在启动阶段返回带字段路径的错误。
- UART 噪声和坏帧只记录并恢复同步，不终止监听循环。
- Source 发送失败记录上下文并返回可诊断错误。
- Function 执行错误进入 `after_func`，保证 GPIO 复位和其他可用通道通知。
- 单个 MessageSink 失败不短路其他 sink。
- Web debug API 使用明确 HTTP 状态码和 JSON 错误体。
- 不再对未知设备或函数使用 `panic!`。

## 10. 测试策略

所有行为修改遵循测试先行：

1. 配置反序列化、验证和旧格式拒绝测试。
2. UART 帧解析纯单元测试。
3. 虚拟串口集成测试改为发送二进制帧。
4. MessageRouter 多 sink、部分失败和路由开关测试。
5. GPIO sink 使用可替换的 pin backend 测试状态序列，不要求开发机真实 GPIO。
6. DebugSource binding 和 Web handler 测试。
7. Function `pre/func/after` 成功与失败生命周期测试。
8. 声明宏注册、未知函数和参数类型错误测试。
9. WebState 历史功能回归测试。
10. 最终执行 `cargo fmt --check`、`cargo check` 和 `cargo test --all-targets`。

硬件与 GUI 测试继续保留为 `#[ignore]`，并在完成记录中说明未自动执行的环境依赖。

## 11. 分阶段提交与回档

计划创建以下独立提交：

1. 设计文档。
2. 强类型配置与 UART 二进制协议。
3. Message 层和共享输出基础设施。
4. DebugSource 与 Web API。
5. Function 生命周期和声明宏。
6. 前端重构。
7. 文档、任务完成记录和最终回归修正。

`changeTask/completed.md` 最终记录：

- 每项任务状态。
- 修改的主要文件。
- 验证命令与结果。
- 每阶段 Git commit hash。
- 未实施内容和硬件测试限制。

## 12. 非目标

- 不实现 `toSDK.md` 中的完整 SDK。
- 不实现 `0x04` 的任务取消。
- 不实现 `0x05` 的复杂状态协议。
- 不设计新的 UART 返回二进制帧协议。
- 不实现真实路口识别算法。
- 不引入过程宏、动态插件或运行时加载。
- 不增加前端 Node.js 构建系统。

## 13. 调研依据

- Rust 普通过程没有运行时反射或自动扫描机制；属性过程宏需要独立 `proc-macro` crate：<https://doc.rust-lang.org/reference/procedural-macros.html>
- `inventory` 可提供链接期分布式注册，但会增加额外机制和依赖，本次不采用：<https://docs.rs/inventory/latest/inventory/>
- `linkme` 也可通过 distributed slice 注册静态项，本次保持声明宏方案：<https://docs.rs/linkme/latest/linkme/>
