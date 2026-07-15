# Message 简介

`Message` 是 Source 交给 Dispatcher 的输入。它只描述一次触发，不包含 Function、Device 或 Sink 的选择逻辑。

主要内容：

- `key`：Binding 匹配使用的字符串。
- `description`：可选说明。
- `started_at_ms`：Source 已知的开始时间。
- `payload`：Source 附带的原始数据。

## UART 分帧

`UartSource` 使用三个配置确定一条 Message 的边界：

```toml
prefix = [170]
suffix = [85]
content_bytes = 2
```

配置中的数字表示一个原始字节，范围为 `0..=255`。例如 `170` 等于 `0xAA`，`85` 等于 `0x55`。前缀和后缀可以包含多个字节。

完整帧长度为：

```text
prefix 长度 + content_bytes + suffix 长度
```

收到 `[170, 1, 2, 85]` 后，Engine 提取内容 `[1, 2]`，并生成：

```text
Message.key = "1_2"
```

单个内容字节 `[1]` 生成 key `"1"`。Binding 只需要匹配内容生成的 key，不需要重复填写前缀和后缀。

`payload` 保留两份数据：

```json
{
  "frame": [170, 1, 2, 85],
  "content": [1, 2]
}
```

UART 是字节流，一次系统读取可能只得到半帧，也可能同时得到多帧。`UartSource` 会缓存半帧、拆分连续帧，并丢弃有效前缀之前的无效字节。

当 `prefix` 和 `suffix` 都为空、`content_bytes = 1` 时，每个字节生成一条 Message，兼容简单的单字节命令。

Engine 不解释 CMD、PARAM、颜色或业务状态。这些含义由 Binding 和 Function 决定。
