# Task TODO

## 2026-06-07 分层重构

已完成：

- 配置一次性切换为 `message.yaml`、`functions.toml` 和 typed params。
- UART 输入切换为 `AA CMD PARAM 55` 四字节二进制帧。
- UART transport 单一所有权，以及 Web/UART/GPIO Message sinks。
- GPIO 任务开始/结束消息和错误/panic 收尾。
- 声明式函数注册，业务函数迁移到 `src/func/functions.rs`。
- 未知设备、函数和设备类型不匹配改为可恢复错误。
- DebugSource binding、Web API 和嵌入式任务控制台。
- Web 消息历史、持久化和原有页面功能保持。

完整提交与验证记录见 `changeTask/completed.md`。

## 明确保留

- [x] 实现真实 `cross` 同心圆中心与彩色圆柱校正算法。
- [ ] 为 UART `0x04` 增加可取消任务模型。
- [ ] 为 UART `0x05` 设计状态/心跳响应。
- [ ] 完善 TimerSource 和 LoopSource 的调度语义。
- [ ] 评估摄像头复用、并发访问和设备生命周期管理。
- [ ] 将 `changeTask/toSDK.md` 的远期设想单独设计为稳定 SDK。

## 硬件验证

- [ ] 在目标设备验证真实 UART 输入和结果输出。
- [ ] 在目标设备验证 active-low GPIO 灯光。
- [ ] 使用真实摄像头运行被忽略的 OpenCV/GUI 测试。
