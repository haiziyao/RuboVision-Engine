# RuboVision Ubuntu 实现设计

## 1. 文档范围

本文确定 `rubo_vision` 在 Ubuntu 硬件环境中的下一阶段实现方向。本阶段只确定行为、模块边界和验证条件，不修改 Rust 功能代码，也不提前确定新增公共 Trait、方法和字段的名称。

设计目标是恢复旧项目已有的 GPIO、UART 和 OpenCV 能力，同时继续通过 `rubo_engine` 完成 Source、Message、Function、Output 和 Sink 编排。

## 2. 框架边界

`rubo_engine` 负责：

- Message 并发调度。
- Function 执行生命周期通知。
- Function 计时和错误转 Output。
- Device 访问限制。
- Output 到普通 Sink 的路由。

`rubo_vision` 负责：

- UART 串口读写、触发规则和输出编码。
- GPIO 端口及有效电平。
- Camera 的打开和当前帧读取。
- OpenCV 识别逻辑及其终止条件。
- 项目配置和硬件错误日志。

UART 帧协议不得写入 Engine，也不得写入视觉 Function。Engine 和 Function 只处理 Message、配置值和业务结果。

## 3. 运行链路

```text
./config
   |
   v
rubo_vision 启动
   |
   +-- UART 后台线程独占串口
   |      +-- 输入 -> UART Source -> Message
   |      +-- 输出 <- UART Sink <- Output
   |
   +-- Camera Device 启动时打开摄像头
   |
   v
Engine Dispatcher
   |
   +-- 每个 Message 创建独立执行任务
   +-- 不同 Device 可以并行
   +-- 相同 Camera Device 通过互斥访问自动排队
   |
   v
Executor
   |
   +-- Function 开始前通知 GPIO
   +-- Function 获取当前帧
   +-- OpenCV 计算进入阻塞线程池
   +-- 成功、错误或 panic 后通知 GPIO
   |
   v
Output -> Web Sink / UART Sink
```

## 4. Message 与 Function 并发

同一个 Source 不再等待上一条 Message 完整执行后才处理下一条 Message。每条 Message 都进入独立执行任务。

并发限制由 Device 决定：

- 使用不同 Camera Device 的 Function 可以同时执行。
- 使用同一个 Camera Device 的 Function 必须排队读取该设备。
- Function 本身不增加全局互斥限制。
- UART Source 可以继续接收并分发下一条有效命令。

OpenCV 同步计算不得直接占用 Tokio 异步执行线程，应在阻塞线程池中执行。

## 5. GPIO 执行状态

GPIO 表示 Function 正在执行，不是 Function 完成后的业务输出，因此不能依赖普通 Output Sink 才触发。

Executor 的生命周期顺序为：

```text
Function 即将执行
  -> 通知 GPIO 开始
  -> 执行 Function
  -> 生成成功或错误 Output
  -> 通知 GPIO 结束
  -> 路由普通 Output Sink
```

GPIO 使用三个现有端口，暂定行为为：

- 第一个 Function 开始执行时，三个灯全部点亮。
- 多个 Function 并发执行时，三个灯保持点亮。
- 最后一个 Function 结束后，三个灯全部熄灭。
- Function 返回错误或发生 panic 时，同样必须完成熄灭操作。
- 程序退出时必须强制恢复全部熄灭状态。

GPIO 后台处理需要维护活动任务数量，避免一个并发任务结束后提前关闭仍在运行任务的状态灯。

端口号和有效电平继续由 `sink.toml` 配置，不写死在 Engine 中。

## 6. UART 单实例读写

整个进程只允许一个后台线程持有并操作 UART 句柄。Source 和 Sink 不得分别打开同一个串口。

UART 后台线程提供两个方向：

- 接收方向：读取串口字节，交给 UART Source 转换成 Message。
- 发送方向：接收 UART Sink 提交的结果并写入同一串口句柄。

运行行为：

- 串口打开失败或连接断开后，按固定间隔重新尝试连接。
- 运行时停止期间收到的命令直接丢弃并记录警告。
- 运行时停止期间不缓存待执行命令。
- 断线期间无法发送的结果直接丢弃并记录日志。
- 重连后不补发旧结果。
- Engine 运行时重启不重复创建第二个 UART 句柄。

UART 最终帧协议暂不确定。触发条件和输出转换必须由 `rubo_vision` 配置控制。当前阶段 UART 业务输出只要求发送颜色名称，后续可以通过配置调整。

## 7. Camera Device

Camera Device 不再只保存设备路径。每个 Camera Device 应在创建时打开对应的摄像头，并通过互斥设备能力管理需要可变访问的 `VideoCapture`。

Camera Device 只向 Function 提供一个获取当前有效帧的能力：

- 读取摄像头当前帧。
- 空帧或读取失败时返回设备错误。
- 成功取得 Mat 后立即结束 Camera 互斥访问。
- 视觉分析在释放 Camera 后进行。

具体公开方法名称在 Ubuntu 编码前单独确认，本文不锁定 API 名称。

## 8. OpenCV Function

视觉 Function 只负责自身识别过程和业务结果，不负责 GPIO、UART 协议或 Engine 调度。

每个视觉 Function 必须自行保证有限执行：

- 在自身允许的取帧次数或时间范围内寻找结果。
- 找到有效结果后立即返回。
- 到达自身终止条件仍未找到结果时返回 Function Error。
- Engine 不强制中断 Function，也不替 Function 决定识别超时。

视觉 Function 获取帧后，应把 CPU 密集型 OpenCV 计算放入阻塞线程池。这样即使识别耗时较长，Web、UART 接收和其他异步任务仍可继续运行。

## 9. 配置文件职责

保持现有配置文件分类，不增加新的配置文件：

- `application.yaml`：项目名、Web、日志和配置目录。
- `source.toml`：UART 输入设置、触发条件及以后调整的帧规则。
- `device.toml`：Camera 路径及设备配置。
- `function.toml`：识别参数和每个视觉 Function 自身的有限执行条件。
- `sink.toml`：UART 输出转换、GPIO 端口及有效电平。
- `binding.toml`：Source、Function、Device、Web Sink 和 UART Sink 的连接关系。

配置中的 UART 触发条件允许后续直接调整，不要求 Engine 理解其格式。

## 10. 错误行为

- UART 打开失败或断开：记录错误并继续重连，不终止 Web。
- UART 输出失败：记录错误并丢弃当前结果。
- Camera 创建失败：运行时停止；Web 开启时仍允许修改配置和重启。
- Camera 读取失败：Function 返回错误。
- OpenCV 识别失败：Function 返回错误。
- Function 错误或 panic：Executor 仍完成 GPIO 结束通知，并生成一次错误 Output。
- 单个 Sink 失败：记录该 Sink 错误，不重复执行 Function。

## 11. Web 与路径

项目根目录固定为运行时进程工作目录 `.`：

- 配置目录为 `./config/`。
- 链路文件为 `./chain.json`。
- 不使用编译期 `CARGO_MANIFEST_DIR`。
- 不把编译机器的绝对路径写入程序。

Web 开启时，运行时错误不影响配置页面继续工作。Web 关闭时，程序直接运行纯硬件链路，不因缺少 WebState 退出。

## 12. Ubuntu 实现顺序

1. 建立单 UART 句柄的双向后台线程并接入 Source、Sink。
2. 增加 Engine 的通用 Function 生命周期通知。
3. 使用 GPIO 后台处理接收开始和结束通知。
4. 调整 Engine 同一 Source 的 Message 并发调度。
5. 将 Camera 改为持有 VideoCapture 的互斥 Device。
6. 调整视觉 Function 的取帧和阻塞计算边界。
7. 修复无 Web 启动和相对路径。
8. 在 Ubuntu 硬件与 OpenCV 环境完成验证。

## 13. 验收条件

- 两个不同 Camera Device 的 Function 可以并行执行。
- 同一个 Camera Device 的 Function 自动排队。
- 第一个任务开始时三个 GPIO 灯全部点亮。
- 最后一个任务结束后三个 GPIO 灯全部熄灭。
- Function 错误或 panic 后 GPIO 不残留点亮状态。
- UART Source 和 UART Sink 共用唯一串口句柄。
- UART 断开后能够重连，且不执行或补发旧消息。
- OpenCV Function 运行期间 Web 仍可响应。
- 视觉 Function 不会无限等待目标出现。
- Web 关闭后纯硬件链路仍可运行。
- 程序从工作目录读取 `./config/` 并生成 `./chain.json`。

## 14. 明确暂缓事项

以下内容不属于本轮 Ubuntu 实现前的确定范围：

- UART 最终输入帧格式。
- UART 多字段结果编码。
- UART `data_bit`、`stop_bit` 和 `parity_bit` 的实际应用。
- Windows 环境中的 OpenCV 和硬件验证。
- 新增公共 Trait、方法和字段的最终命名。
- 视觉测试函数的进一步重构。

## 15. 分支发布结构

GitHub 仓库为 `haiziyao/RuboVision-Engine`。

`engine` 分支只保留：

```text
Cargo.toml
Cargo.lock
rubo_engine/
rubo_engine_macros/
```

`example` 分支只保留：

```text
Cargo.toml
Cargo.lock
rubo_vision/
  doc/design.md
```

`example` 分支中的 `rubo_vision` 使用 crates.io 发布版本 `rubo_engine = "0.2.0"`，不依赖相邻目录中的本地 crate。
