# RuboVision Engine 架构设计文档（改进版）

## 1. 项目定位

RuboVision Engine 是一个部署在 **Raspberry Pi 5** 上的机器人视觉节点，用于 RoboCup 车型竞赛中的视觉任务处理。

系统职责包括：

* 颜色识别
* 二维码识别
* Cross 检测
* UART 与电控通信
* GPIO 状态指示

该项目的目标不是构建一个通用计算机视觉框架，而是提供一个 **稳定、可部署、可扩展的机器人视觉节点**。

设计原则：

* 稳定优先
* 工程可维护优先
* 可调试优先
* 性能足够即可

---

# 2. 系统总体架构

系统采用 **分层架构 + 消息驱动模型**。

核心思想：

* 设备由专用 worker 线程独占
* 任务由调度器统一派发
* 各模块之间通过消息通信

系统主要由以下五层组成：

```
Application Layer
        │
        ▼
Scheduler / Runtime Layer
        │
        ▼
Vision Service Layer
        │
        ▼
Device Layer
        │
        ▼
Configuration Layer
```

---

# 3. 启动流程

程序启动流程如下：

```
main()
 │
 ├─ 读取配置文件
 │
 ├─ 解析命令行参数
 │
 ├─ 构建 AppConfig
 │
 ├─ 初始化设备
 │
 ├─ 启动 Worker 线程
 │
 ├─ 启动调度线程
 │
 └─ 启动串口监听
```

系统运行之后由 **UART 命令驱动任务执行**。

---

# 4. 模块设计

## 4.1 Configuration Layer

负责读取和管理配置。

配置来源包括：

* TOML 配置文件
* 命令行参数
* （未来）Web 调试界面

最终统一为：

```
AppConfig
```

示例结构：

```rust
pub struct AppConfig {
    pub color_camera: CameraConfig,
    pub qr_camera: CameraConfig,
    pub cross_camera: CameraConfig,
    pub serial: SerialConfig,
    pub gpio: GpioConfig,
}
```

职责：

* 配置解析
* 参数校验
* 提供统一配置接口

---

# 5. Device Layer

设备层封装所有硬件设备。

包括：

* Camera
* UART
* GPIO

设备层只提供 **最基本能力**。

示例：

```rust
trait Camera {
    fn open() -> Result<Self>;
    fn read_frame(&mut self) -> Frame;
}
```

设备层不包含业务逻辑。

---

# 6. Vision Service Layer

该层实现具体视觉任务。

例如：

### color_detect

处理流程：

1. 获取摄像头帧
2. ROI 区域裁剪
3. HSV 颜色筛选
4. 面积筛选
5. 连续计数判断

返回结果：

```
ColorResult
```

---

### qr_detect

使用 quircs 进行二维码解析。

处理流程：

```
frame -> decode -> result
```

返回：

```
QRData
```

---

# 7. Scheduler / Runtime Layer

调度层是系统核心。

职责：

* 接收命令
* 选择任务
* 调用 worker
* 返回结果

示例命令：

```
a1 -> color detect
b1 -> qr detect
```

任务流程：

```
UART
 │
 ▼
Command Parser
 │
 ▼
Scheduler
 │
 ├─ Color Worker
 │
 ├─ QR Worker
 │
 └─ Cross Worker
```

---

# 8. Worker 线程模型

每个设备由 **独立 worker 线程管理**。

这样可以避免设备竞争。

示例：

### color worker

```
loop {
    wait message

    capture frame

    run color detection

    send result
}
```

线程结构：

```
Thread 1: UART Listener
Thread 2: Scheduler
Thread 3: Color Worker
Thread 4: QR Worker
```

该结构适配 Raspberry Pi 4 核 CPU。

---

# 9. 消息通信

系统内部使用 **mpsc channel** 进行通信。

通信方向：

```
UART -> Scheduler
Scheduler -> Worker
Worker -> Scheduler
Scheduler -> UART
```

消息类型示例：

```rust
enum TaskMessage {
    ColorDetect,
    QRDetect,
}
```

---

# 10. GPIO 状态管理

GPIO 指示灯用于表达系统状态。

建议状态：

| 状态   | LED |
| ---- | --- |
| 启动中  | 黄   |
| 就绪   | 绿   |
| 任务执行 | 蓝   |
| 错误   | 红   |

GPIO 控制由 Scheduler 统一管理。

---

# 11. Web 调试接口（未来）

未来计划增加 Web 调试界面。

功能包括：

* 实时图像预览
* 参数调节
* 调试日志
* 任务触发

Web 服务为 **可选模块**。

---

# 12. 部署设计

系统使用 systemd 管理。

服务特点：

* 开机自启动
* 自动重启
* 日志统一管理

示例：

```
systemctl start rubocar
systemctl stop rubocar
journalctl -u rubocar
```

---

# 13. 后续架构演进

未来版本计划：

## v0.x

基础功能实现。

## v1.x

模块拆分：

```
rubovision-core
rubovision-device
rubovision-vision
rubovision-runtime
```

## v2.x

插件化任务系统。

---

# 14. 设计总结

本项目采用：

* 分层架构
* 消息驱动
* 设备独占 worker
* 配置驱动

优势：

* 稳定
* 易调试
* 易扩展

该架构适合机器人视觉节点长期运行。

未来可逐步扩展为完整机器人视觉框架。
