# changeTask 完成记录

日期：2026-06-07 至 2026-06-08

| 任务 | 状态 | 验证 | Commit |
| --- | --- | --- | --- |
| 分层重构设计 | 完成 | 设计文档已确认 | `fd3f52fbc77fc8dd16b36cbaa6981b7d9fa0dc20` |
| 分阶段实施计划 | 完成 | 计划覆盖配置、协议、消息、函数、Web 和文档 | `7fead47961b0c0376c03029642618da422037d3b` |
| 类型化配置与一次性切换 | 完成 | 配置加载、旧字段拒绝和引用校验测试 | `054598775862b55f6d5140086a2d346031c74786` |
| UART 三字节二进制帧 | 完成 | 拆包、粘包、噪声、坏帧、溢出和虚拟串口测试 | `616407a644983812537a063d9b411c7cb8a07afa` |
| Web/UART/GPIO Message 层 | 完成 | 独立 sink、失败隔离、GPIO 生命周期测试 | `d8a1bc302f80176721ff87646e93b4f80c209e3a` |
| 声明式函数与完整生命周期 | 完成 | 类型参数、未知 ID、错误返回和 panic 收尾测试 | `92a7b15dc859ed7c1c3bb2df0fd149e3b4f473eb` |
| DebugSource 与 Web API | 完成 | binding、事件投递和 `202/404/503` handler 测试 | `de93f287dff4d9d3b69cd791a999ffe028372773` |
| Web 控制台重构 | 完成 | 嵌入资源契约、DOM/API 检查和全量 Rust 测试 | `fdaf74b47c338c0bc4834864cf3ab6befbe877a8` |

## 明确保留

- `changeTask/toSDK.md`：远期 SDK 设想，本轮不实现。
- UART `0x04`：仅识别并记录“停止任务”预留命令，不实现任务取消。
- UART `0x05`：仅识别并记录“状态/心跳”预留命令，不实现业务响应。
- `cross_detect`：保留当前占位结果，不扩展真实路口识别算法。
- `source_key`：作为 binding 标识发送和匹配，不增加额外协议语义。

## 最终验证

- `cargo fmt --check`
- `cargo check`
- `cargo test --all-targets`
- `git diff --check`

本轮最终结果为 `31 passed / 0 failed / 10 ignored`。忽略项需要真实摄像头、GUI 或长驻服务器。

前端 visual companion 因环境缺少 Node 无法启动；无头 Firefox 又因容器 framebuffer
不可用而无法截图。页面通过嵌入资源契约、唯一 DOM ID、API 引用检查和 Web handler
测试验证。
