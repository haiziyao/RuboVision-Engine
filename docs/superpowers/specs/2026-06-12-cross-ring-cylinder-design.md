# Cross 同心圆与圆柱校正设计

**日期：** 2026-06-12  
**范围：** 实现预留的 `cross` 视觉函数，并让 UART 的单次触发参数穿过解析、调度和函数执行链路。

## 目标

`cross` 在同一套函数注册和输出流程下提供两类能力：

- 参数 `0`：识别黑色同心圆的共同圆心，输出圆心相对可调屏幕目标点的像素偏移。
- 参数 `1..5`：识别对应颜色的圆柱顶面，输出圆柱中心相对黑色同心圆圆心的像素偏移。

坐标使用 OpenCV 图像坐标：

- `dx > 0` 表示目标在基准点右侧。
- `dy > 0` 表示目标在基准点下方。

摄像头与目标平面垂直，本功能不做透视或地面坐标换算。

## 已确认决策

### 函数与参数

配置和注册中的基础 `function_id` 始终为 `cross`。模式和颜色不编码进函数名，而由每次触发携带的 `runtime_param` 决定：

| runtime_param | 行为 |
| --- | --- |
| `0` | 同心圆中心相对屏幕目标点 |
| `1` | 红色圆柱相对同心圆中心 |
| `2` | 蓝色圆柱相对同心圆中心 |
| `3` | 绿色圆柱相对同心圆中心 |
| `4` | 黑色圆柱相对同心圆中心 |
| `5` | 白色圆柱相对同心圆中心 |

编号、名称和 HSV 范围保存在 TOML 的颜色表中，后续可修改；函数逻辑不写死颜色名称。未配置或超出 `0..5` 的参数在打开摄像头前返回错误。

### UART 协议

UART 输入帧由固定三字节升级为固定四字节：

```text
AA CMD PARAM 55
```

例如：

```text
AA 03 00 55  # cross 参数 0
AA 03 01 55  # cross 参数 1
AA 03 05 55  # cross 参数 5
```

解析规则：

1. 在字节流中寻找帧头 `0xAA`，丢弃帧头前噪声。
2. 等待四个字节。
3. 读取第 2 字节为 `CMD`，第 3 字节为 `PARAM`。
4. 第 4 字节必须为帧尾 `0x55`；否则丢弃一个字节并重新同步。
5. `CMD` 根据 binding 解析为固定的 `function_id` 和设备，`PARAM` 原样写入任务事件。

旧三字节帧 `AA CMD 55` 不再触发。无需动态参数的函数要求发送 `PARAM=0`。停止和状态命令仍按 `CMD` 分类，其 `PARAM` 暂不使用。

## 框架数据流

任务事件增加独立的 `runtime_param: u8`。数据流为：

```text
UART frame
  -> UartSource(CMD binding + PARAM)
  -> task Event(function_id, device_id, runtime_param)
  -> TaskDispatcher
  -> FunctionWorker
  -> typed function(static params, runtime_param, device)
```

静态 TOML 参数仍在注册阶段反序列化并校验。动态参数不修改静态参数，也不通过函数名解析。

函数声明宏和 runner 接口统一增加运行时参数。现有函数接收但忽略该值，保持原有行为。Timer、Loop 和未指定参数的 DebugSource 产生 `runtime_param=0`。

Web DebugSource 的触发请求可选携带 `runtime_param`，缺省为 `0`，便于人工触发 `cross` 的各模式，但 binding 中的 `function_id` 仍为 `cross`。

## Cross 静态配置

`CrossDetectParams` 至少包含：

```toml
[functions.entries.params]
debug_model = false
loop_count = 3
target_correction = { x = 0, y = 0 }
black_threshold = 90
min_radius = 20.0
max_radius = 600.0
center_tolerance = 12.0
min_arc_points = 24
min_ring_score = 50
colors = [
  { id = 1, name = "red",   hsv = [0, 20, 100, 255, 80, 255], min_area = 500.0, min_circularity = 0.65 },
  { id = 2, name = "blue",  hsv = [90, 140, 80, 255, 50, 255], min_area = 500.0, min_circularity = 0.65 },
  { id = 3, name = "green", hsv = [35, 90, 70, 255, 50, 255], min_area = 500.0, min_circularity = 0.65 },
  { id = 4, name = "black", hsv = [0, 179, 0, 255, 0, 70], min_area = 500.0, min_circularity = 0.65 },
  { id = 5, name = "white", hsv = [0, 179, 0, 60, 140, 255], min_area = 500.0, min_circularity = 0.65 },
]
```

具体初始阈值允许在实现和实拍调试中微调。校验要求包括：

- `loop_count > 0`
- 阈值、HSV 和评分范围合法
- 半径上下限有序
- `center_tolerance > 0`
- 颜色 ID 唯一且覆盖 `1..5`
- 面积和圆度阈值合法

`config/functions.toml` 中将函数名改为 `cross`，并打开 UART 返回。`config/bindings.toml` 的 UART 和 Debug binding 同样引用 `cross`。

## 同心圆中心检测

采用“多圆弧候选 + 共同圆心评分”，不依赖完整闭合轮廓。

### 预处理

1. BGR 转灰度。
2. 轻量高斯平滑，降低金属表面的细密纹理。
3. 通过黑色阈值生成 mask。
4. 使用小核形态学开闭运算去除孤立噪点并连接短缺口。
5. 提取轮廓或边缘链。

### 圆弧候选

对长度达到 `min_arc_points` 的轮廓段拟合圆或椭圆，并计算：

- 拟合中心和等效半径。
- 点到拟合圆的残差。
- 圆弧覆盖角。
- 轮廓长度。
- 半径是否在配置范围内。

残差过大、覆盖角过小或半径越界的候选被丢弃。圆弧不要求闭合，因此允许圆柱、夹具或画面边缘遮挡。

### 共同圆心

按 `center_tolerance` 对候选中心聚类。每组候选依据以下因素评分：

- 支持该中心的圆弧数量。
- 不同半径层级的数量。
- 总覆盖角和总轮廓长度。
- 各候选拟合残差。
- 中心离散程度。

选择最高分组，并按候选质量加权计算 `ring_center`。得分低于 `min_ring_score` 时结果无效，不根据屏幕中心伪造圆环中心。

该方法允许外圈部分出画，也允许中心区域被圆柱和黄色结构遮挡；只要剩余多个圆弧仍支持同一圆心即可输出。

## 彩色圆柱检测

`runtime_param=1..5` 时，必须先得到有效 `ring_center`，再查找对应颜色：

1. 根据颜色配置生成 HSV mask。
2. 进行形态学去噪和填洞。
3. 仅评估圆环有效区域附近的候选，排除远处同色物体。
4. 按面积、圆度、长宽比、实心填充率和尺寸筛选圆柱顶面。
5. 在多个候选中综合几何得分与距 `ring_center` 的距离选择最佳目标。

黑色圆柱不能只依赖黑色 mask，因为圆环本身也是黑色。它还必须满足较高实心填充率和较大连续面积；细圆环和圆弧不满足该条件。白色圆柱则通过低饱和高亮度、几何形状和圆环区域约束与金属背景区分。

若圆环有效但指定颜色圆柱无可靠候选，返回无效结果。

## 偏移与输出

参数 `0` 的基准点为：

```text
target_x = image_width / 2 + target_correction.x
target_y = image_height / 2 + target_correction.y
dx = ring_center.x - target_x
dy = ring_center.y - target_y
```

参数 `1..5` 的基准点为圆环中心：

```text
dx = cylinder_center.x - ring_center.x
dy = cylinder_center.y - ring_center.y
```

统一输出：

```text
CROSS,param,valid,dx,dy,score
```

示例：

```text
CROSS,0,1,-42,18,91
CROSS,1,1,36,-24,86
CROSS,3,0,0,0,0
```

`score` 为 `0..100` 的整体置信度。圆柱模式的整体分数同时受圆环和圆柱候选限制。

## 采集与调试输出

`cross` 按 `loop_count` 读取多帧并保留最高置信度结果，遵循现有 `black_ring` 的设备层流程。

函数返回 `TaskOutput::value_with_image`：

- `value` 为上述 `CROSS,...` 字符串，供 UART 使用。
- `image` 为标注 JPEG，供 Web 调试。
- 蓝色十字表示可调屏幕目标点。
- 绿色十字表示检测到的圆环中心。
- 圆柱轮廓和中心按目标颜色绘制。
- 左上角显示最终协议字符串。

若所有帧均无有效结果，返回最后一帧标注图和无效值。

## 错误处理

- `runtime_param` 越界或缺少对应颜色配置：函数错误，不打开摄像头。
- 摄像头打开或读取失败：沿用 `anyhow` 错误链，由任务执行层生成错误输出。
- 无可靠圆环或圆柱：正常的 `valid=0` 视觉结果，不视为系统错误。
- 空帧：跳过；全部为空时返回无效结果和可用的最后帧。

## 测试

### UART

- 四字节完整帧。
- 分段输入、粘包和帧头前噪声。
- 错误尾字节后的重新同步。
- `CMD` 与 `PARAM` 均被保留。
- 旧三字节帧不会触发。
- 超长 pending buffer 被清空。

### 框架传参

- Event、Dispatcher、FunctionWorker 和声明宏传递相同参数。
- UART `cross` binding 保持基础函数名并收到 `0..5`。
- Timer、Loop 和默认 DebugSource 使用 `0`。
- Web DebugSource 显式参数可到达函数。
- 现有颜色、二维码、黑环和 debug 函数在参数 `0` 下行为不变。

### 配置

- 合法 Cross 参数可加载。
- 非法阈值、半径、评分、HSV、重复/缺失颜色 ID 被拒绝。
- binding 引用 `cross` 能通过完整配置校验。

### 合成视觉

- 完整多层同心圆：圆心误差不超过约 5 px。
- 外圈出画或多段圆弧被遮挡：证据充足时仍有效。
- 无同心圆或圆弧中心不一致：结果无效。
- 指定颜色圆柱偏心：`dx/dy` 符号和数值正确。
- 圆柱遮挡中心圆环：依靠剩余圆弧恢复圆心。
- 出现非目标颜色：结果无效。
- 黑色实心圆柱与黑色细圆环可区分。
- 配置的目标点修正只影响参数 `0`。
- 输出字符串和 Web 标注图可生成。

### 最终验证

```bash
cargo fmt --check
cargo check
cargo test --all-targets
```

真实摄像头与两张实拍图用于最终人工调参和验收，不替代可重复的合成测试。

## 不在本次范围

- 透视校正、世界坐标或毫米换算。
- 机器学习模型。
- 自动学习 HSV 阈值。
- 同时跟踪多个圆柱。
- 保留旧三字节 UART 输入协议。

