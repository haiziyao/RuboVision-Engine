# RuboVision 电控 UART 接入协议

本文档面向负责生成、审查或修改电控程序的 Agent。本文档描述的是当前已经部署到香橙派上的真实协议，不是未来方案。

验证基线：

- RuboVision 示例项目：`example` 分支
- RuboEngine：`0.3.0`
- 服务：`rubo-vision.service`
- 最后联调日期：2026-07-15
- 已验证输入：`61 31 0D 0A`
- 已验证链路：UART Source -> `uart_debug` -> `debug_fun` -> Web Sink + UART Sink

本文档中的 MUST、MUST NOT 和 SHOULD 分别表示必须、禁止和建议。

## 1. 协议结论

电控发送给 RuboVision 的每条请求固定为 4 字节：

```text
+--------+---------+----------+----------+
| 0x61   | COMMAND | 0x0D     | 0x0A     |
| 'a'    | 1 byte  | CR       | LF       |
+--------+---------+----------+----------+
```

公式：

```text
REQUEST = PREFIX + CONTENT + SUFFIX
PREFIX  = [0x61]
CONTENT = [COMMAND]
SUFFIX  = [0x0D, 0x0A]
```

RuboVision 返回给电控的是一行 UTF-8 文本：

```text
RESPONSE = RESULT_VALUE + 0x0A
```

响应只有 `LF`，不保证带 `CR`，没有帧头、长度、请求编号和校验和。

## 2. 电气连接

当前接口是 3.3V TTL UART，不是 RS-232，也不是 RS-485。

香橙派当前使用：

| 信号 | 香橙派物理针脚 | 方向 |
|---|---:|---|
| UART RX | 11 | 电控发送，香橙派接收 |
| UART TX | 36 | 香橙派发送，电控接收 |
| GND | 6 | 共地 |

接线规则：

```text
电控 TX  -> 香橙派 RX（物理针脚 11）
电控 RX  <- 香橙派 TX（物理针脚 36）
电控 GND -- 香橙派 GND（物理针脚 6）
```

必须满足：

- 双方 GND MUST 相连。
- TX 和 RX MUST 交叉连接。
- 电控 TX 高电平 MUST 是 3.3V 逻辑电平。
- MUST NOT 将 5V TX 直接连接到香橙派 RX。
- MUST NOT 使用串口模块的 VCC 给香橙派供电。
- 使用 HW-597B 时必须先选择 3.3V 档，并实测 TXD 对 GND 的高电平。

## 3. 串口参数

双方必须使用完全相同的参数：

| 参数 | 值 |
|---|---|
| 串口设备 | 香橙派 `/dev/ttyAMA1` |
| 波特率 | 9600 |
| 数据位 | 8 |
| 校验位 | None |
| 停止位 | 1 |
| 流控 | None |
| 工作方式 | 全双工 |

常用简称：`9600 8N1`。

## 4. 命令表

COMMAND 是一个原始二进制字节，不是十进制数字字符串。

| 功能 | COMMAND | 完整请求十六进制 | C 字节数组 |
|---|---:|---|---|
| 颜色识别 | `0x01` | `61 01 0D 0A` | `{0x61, 0x01, 0x0D, 0x0A}` |
| 二维码识别 | `0x02` | `61 02 0D 0A` | `{0x61, 0x02, 0x0D, 0x0A}` |
| 同心圆环定位 | `0x03` | `61 03 0D 0A` | `{0x61, 0x03, 0x0D, 0x0A}` |
| 黑环识别 | `0x04` | `61 04 0D 0A` | `{0x61, 0x04, 0x0D, 0x0A}` |
| A/B/C 字母识别 | `0x05` | `61 05 0D 0A` | `{0x61, 0x05, 0x0D, 0x0A}` |
| 彩色柱定位 | `0x06` | `61 06 0D 0A` | `{0x61, 0x06, 0x0D, 0x0A}` |
| Debug 联调 | `0x31` | `61 31 0D 0A` | `{0x61, 0x31, 0x0D, 0x0A}` |

### 4.1 必须注意的不对称规则

Debug 使用 ASCII 字符 `'1'`，其字节值是 `0x31`。

视觉功能使用二进制字节 `0x01` 到 `0x06`，不是 ASCII 字符 `'1'` 到 `'6'`。

因此：

```text
61 31 0D 0A = Debug
61 01 0D 0A = 颜色识别
```

下面两条请求不是同一条请求：

```c
const uint8_t debug_request[] = {'a', '1', '\r', '\n'};
const uint8_t color_request[] = {'a', 0x01, '\r', '\n'};
```

对方 Agent MUST NOT 使用 `sprintf("a%d\r\n", command)` 生成视觉命令。该写法会生成 ASCII 数字，导致 Binding 无法匹配。

## 5. Engine 如何解析请求

当前 Source 配置：

```toml
[uart]
baud = 9600
data_bit = 8
kind = "uart"
parity_bit = false
serial = "/dev/ttyAMA1"
stop_bit = 1
prefix = [97]
suffix = [13, 10]
content_bytes = 1
```

十进制配置值对应：

```text
97 = 0x61 = 'a'
13 = 0x0D = CR
10 = 0x0A = LF
```

Engine 从完整帧中只提取中间的 COMMAND 字节，并将其无符号十进制值转换为 Binding key：

```text
0x01 -> key "1"
0x02 -> key "2"
0x03 -> key "3"
0x04 -> key "4"
0x05 -> key "5"
0x06 -> key "6"
0x31 -> key "49"
```

Debug Binding 的实际配置：

```toml
[uart_debug]
debug = true
devices = []
function = "debug_fun"
sinks = ["web", "uart"]

[uart_debug.source]
event = "49"
id = "uart"
```

UART 是字节流。Engine 会缓存半帧、拆分连续帧，并丢弃有效前缀前的无关字节。因此一次底层读取不需要正好对应一帧，但发送方仍应一次发送完整4字节请求。

## 6. 响应格式

UART Sink 不发送完整 JSON，也不发送图片。它只提取 Function 返回对象中的 `value` 字段，然后追加一个 `LF`：

```text
UTF-8(value) + 0x0A
```

例如 Debug 成功响应：

```text
文本：debug success\n
十六进制：64 65 62 75 67 20 73 75 63 63 65 73 73 0A
```

电控接收程序 MUST 以 `0x0A` 作为一条响应的结束标志。为了兼容其他终端，解析前 SHOULD 删除末尾可选的 `0x0D`，但当前 Engine 只主动追加 `0x0A`。

### 6.1 Debug 返回

成功时固定为：

```text
debug success\n
```

### 6.2 颜色识别返回

当前可能值：

```text
red\n
blue\n
green\n
black\n
white\n
unknown\n
```

电控 SHOULD 将未知字符串作为协议异常处理，不要默认映射成某一种颜色。

### 6.3 二维码识别返回

成功时返回二维码中的 UTF-8 文本，再追加 `LF`：

```text
<qr-content>\n
```

当前协议没有长度字段，因此用于联调的二维码内容 SHOULD NOT 包含嵌入式换行符。电控接收缓冲区不得只按十几个字节设计，二维码结果可能比其他结果长。

### 6.4 同心圆环定位返回

格式：

```text
CROSS,<runtime_param>,<found>,<dx>,<dy>,<score>\n
```

字段定义：

| 字段 | 类型 | 含义 |
|---|---|---|
| `runtime_param` | `uint8` 文本 | 当前 UART 命令下固定为0 |
| `found` | `0` 或 `1` | 是否找到目标 |
| `dx` | 有符号十进制整数 | X方向偏差 |
| `dy` | 有符号十进制整数 | Y方向偏差 |
| `score` | `0..100` | 识别评分 |

示例：

```text
CROSS,0,1,-12,8,92\n
CROSS,0,0,0,0,0\n
```

电控 MUST 按逗号拆成6个字段，并检查第一个字段严格等于 `CROSS`。

### 6.5 黑环识别返回

格式：

```text
RING,<found>,<dx>,<dy>,<score>\n
```

示例：

```text
RING,1,-5,11,88\n
RING,0,0,0,0\n
```

电控 MUST 按逗号拆成5个字段，并检查第一个字段严格等于 `RING`。

### 6.6 A/B/C 字母识别返回

成功时只返回识别出的单个大写字母：

```text
A\n
B\n
C\n
```

其他字符串不是有效的字母识别结果。识别失败会进入错误返回，不会返回默认字母。

### 6.7 彩色柱定位返回

格式：

```text
BLOCK,<color>,<found>,<dx>,<dy>\n
```

字段定义：

| 字段 | 类型 | 含义 |
|---|---|---|
| `color` | 文本 | 当前配置为 `red`、`blue`、`green`、`black`、`white`；未找到时为 `unknown` |
| `found` | `0` 或 `1` | 是否找到任一配置颜色的柱子 |
| `dx` | 有符号十进制整数 | 彩色柱中心相对目标点的 X 偏差，向右为正 |
| `dy` | 有符号十进制整数 | 彩色柱中心相对目标点的 Y 偏差，向下为正 |

示例：

```text
BLOCK,blue,1,-90,-170\n
BLOCK,red,1,24,-8\n
BLOCK,unknown,0,0,0\n
```

电控 MUST 按逗号拆成5个字段，并检查第一个字段严格等于 `BLOCK`。未找到时 `color=unknown`、`found=0`，此时 `dx`、`dy` 固定为0。

### 6.8 错误返回

Function 已经匹配但执行失败时，UART 可能收到以下形式的英文文本：

```text
Function: <error message>\n
Runtime: <error message>\n
Dispatch: <error message>\n
```

错误字符串用于诊断，不是稳定的业务数据结构。电控 SHOULD：

1. 识别 `Function:`、`Runtime:`、`Dispatch:` 前缀。
2. 将整行保存到日志。
3. 将本次事务标记为失败。
4. 不依赖后面的英文错误内容做固定业务分支。

无效帧或不存在的 Binding 不保证有 UART 响应，因为 Engine 可能没有可用于返回的 Sink 路由。

## 7. 请求与响应时序

当前协议没有 request ID，无法可靠地区分多个并发请求的响应。因此电控 MUST 使用单请求时序：

```text
IDLE
  |
  | 发送一帧请求
  v
WAIT_RESPONSE
  |
  | 收到 LF 结束的一行，或发生超时
  v
IDLE
```

要求：

- 同一时间 MUST 只有一条在途请求。
- MUST 等待当前响应完成后再发送下一条命令。
- MUST NOT 每秒持续发送视觉命令。
- 当前每秒发送一次 `a1\r\n` 只适合 Debug 联调，正式运行时应停止周期发送。
- 超时后不要立即无限重发，因为原 Function 可能仍在执行。

建议初始超时值：

| 功能 | 建议超时 |
|---|---:|
| Debug | 2秒 |
| 颜色识别 | 15秒 |
| 同心圆环/黑环识别 | 15秒 |
| A/B/C 字母识别 | 15秒 |
| 彩色柱定位 | 15秒 |
| 二维码识别 | 30秒 |

这些是电控侧建议值，不是线协议字段。实际项目可根据摄像头帧率和 `max_frames` 调整。

## 8. STM32 HAL 发送参考实现

```c
#include <stdint.h>
#include "main.h"

typedef enum {
    RUBO_CMD_COLOR      = 0x01,
    RUBO_CMD_QR         = 0x02,
    RUBO_CMD_CONCENTRIC_RING      = 0x03,
    RUBO_CMD_BLACK_RING = 0x04,
    RUBO_CMD_LETTER     = 0x05,
    RUBO_CMD_COLOR_BLOCK = 0x06,
    RUBO_CMD_DEBUG      = 0x31,
} RuboCommand;

HAL_StatusTypeDef rubo_send(UART_HandleTypeDef *uart, RuboCommand command)
{
    const uint8_t frame[4] = {
        0x61,
        (uint8_t)command,
        0x0D,
        0x0A,
    };

    return HAL_UART_Transmit(uart, (uint8_t *)frame, sizeof(frame), 100);
}
```

调用示例：

```c
rubo_send(&huart2, RUBO_CMD_DEBUG);
rubo_send(&huart2, RUBO_CMD_COLOR);
```

不要使用：

```c
// 错误：这会把命令转换成 ASCII 数字。
sprintf(buffer, "a%d\r\n", command);
```

## 9. STM32 HAL 接收参考实现

推荐中断只负责把字节放入环形缓冲区，主循环负责按 `LF` 组装响应。不要在 UART ISR 中做 CSV、JSON 或字符串业务解析。

```c
#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#include <string.h>
#include "main.h"

#define RUBO_RX_RING_CAPACITY 1024u
#define RUBO_LINE_CAPACITY    1024u

static UART_HandleTypeDef *rubo_uart;
static uint8_t rubo_irq_byte;
static volatile uint16_t rubo_rx_head;
static volatile uint16_t rubo_rx_tail;
static volatile bool rubo_rx_overflow;
static uint8_t rubo_rx_ring[RUBO_RX_RING_CAPACITY];

void rubo_uart_start(UART_HandleTypeDef *uart)
{
    rubo_uart = uart;
    rubo_rx_head = 0;
    rubo_rx_tail = 0;
    rubo_rx_overflow = false;
    HAL_UART_Receive_IT(rubo_uart, &rubo_irq_byte, 1);
}

void HAL_UART_RxCpltCallback(UART_HandleTypeDef *uart)
{
    if (uart != rubo_uart) {
        return;
    }

    const uint16_t next = (uint16_t)((rubo_rx_head + 1u) % RUBO_RX_RING_CAPACITY);
    if (next == rubo_rx_tail) {
        rubo_rx_overflow = true;
    } else {
        rubo_rx_ring[rubo_rx_head] = rubo_irq_byte;
        rubo_rx_head = next;
    }

    HAL_UART_Receive_IT(rubo_uart, &rubo_irq_byte, 1);
}

static bool rubo_pop_byte(uint8_t *byte)
{
    if (rubo_rx_tail == rubo_rx_head) {
        return false;
    }

    *byte = rubo_rx_ring[rubo_rx_tail];
    rubo_rx_tail = (uint16_t)((rubo_rx_tail + 1u) % RUBO_RX_RING_CAPACITY);
    return true;
}

bool rubo_poll_line(char *output, size_t output_capacity)
{
    static char line[RUBO_LINE_CAPACITY];
    static size_t length;
    uint8_t byte;

    if (output_capacity == 0u) {
        return false;
    }

    while (rubo_pop_byte(&byte)) {
        if (byte == '\n') {
            if (length > 0u && line[length - 1u] == '\r') {
                length--;
            }

            const size_t copy_length =
                length < output_capacity - 1u ? length : output_capacity - 1u;
            memcpy(output, line, copy_length);
            output[copy_length] = '\0';
            length = 0u;
            return true;
        }

        if (length < RUBO_LINE_CAPACITY - 1u) {
            line[length++] = (char)byte;
        } else {
            length = 0u;
            rubo_rx_overflow = true;
        }
    }

    return false;
}

bool rubo_uart_overflowed(void)
{
    const bool overflowed = rubo_rx_overflow;
    rubo_rx_overflow = false;
    return overflowed;
}
```

主循环示例：

```c
char response[RUBO_LINE_CAPACITY];

for (;;) {
    if (rubo_uart_overflowed()) {
        // 记录接收溢出，并将当前事务判定为失败。
    }

    if (rubo_poll_line(response, sizeof(response))) {
        // 在主循环或任务中解析 response。
        // response 不包含末尾 CR/LF。
    }
}
```

如果项目使用 RTOS，可将完整行发送到消息队列，但仍应保持单请求事务。

## 10. 响应解析建议

解析逻辑应结合当前在途命令：

```text
pending = DEBUG:
    只接受 "debug success"

pending = COLOR:
    接受 red/blue/green/black/white/unknown

pending = QR:
    接受一行非空 UTF-8 文本

pending = CROSS:
    必须匹配 CROSS,param,found,dx,dy,score

pending = BLACK_RING:
    必须匹配 RING,found,dx,dy,score

pending = LETTER:
    只接受 A、B、C

pending = COLOR_BLOCK:
    必须匹配 BLOCK,color,found,dx,dy

任意 pending:
    Function:/Runtime:/Dispatch: 开头 -> 失败
    其他格式 -> 协议错误
```

数值字段解析要求：

- `found` 只能是0或1。
- `dx`、`dy` 必须按有符号整数解析。
- `score` 必须在0到100之间。
- 字段数量不正确 MUST 判定为协议错误。
- MUST 检查整数转换是否溢出。

## 11. Python 桌面参考程序

以下程序可作为协议行为参考，不应替代目标电控实现：

```python
import serial

COMMANDS = {
    "color": 0x01,
    "qr": 0x02,
    "concentric_ring": 0x03,
    "black_ring": 0x04,
    "letter": 0x05,
    "color_block": 0x06,
    "debug": 0x31,
}


def request(port: serial.Serial, command: int) -> str:
    frame = bytes((0x61, command, 0x0D, 0x0A))
    port.write(frame)
    port.flush()
    response = port.readline()
    if not response.endswith(b"\n"):
        raise TimeoutError("RuboVision response did not end with LF")
    return response.rstrip(b"\r\n").decode("utf-8")


with serial.Serial("COM10", 9600, timeout=30) as uart:
    print(request(uart, COMMANDS["debug"]))
```

预期输出：

```text
debug success
```

## 12. 测试向量

### 12.1 必须通过

| 输入十六进制 | 期望行为 |
|---|---|
| `61 31 0D 0A` | 执行 `debug_fun`，返回 `debug success\n` |
| `61 01 0D 0A` | 执行颜色识别 |
| `61 02 0D 0A` | 执行二维码识别 |
| `61 03 0D 0A` | 执行同心圆环识别 |
| `61 04 0D 0A` | 执行黑环识别 |
| `61 05 0D 0A` | 执行 A/B/C 字母识别，成功时返回 `A\n`、`B\n` 或 `C\n` |
| `61 06 0D 0A` | 执行彩色柱定位，返回 `BLOCK,<color>,<found>,<dx>,<dy>\n` |

### 12.2 必须拒绝或不触发目标 Function

| 输入十六进制 | 原因 |
|---|---|
| `31` | 缺少前缀和后缀 |
| `61 31` | 半帧，Engine 等待后缀 |
| `61 31 0A` | 缺少 CR |
| `61 31 0D 00` | 后缀错误 |
| `61 30 0D 0A` | COMMAND `0x30` 没有 Binding |

### 12.3 连续帧

输入：

```text
61 31 0D 0A 61 31 0D 0A
```

Engine 应解析为两次独立 Debug 请求。正式电控仍不应在未收到第一次响应前发送第二次请求。

### 12.4 分段到达

以下三段依次到达时，Engine 应最终拼成一帧：

```text
第一段：61
第二段：31 0D
第三段：0A
```

## 13. 联调流程

1. 确认电控 TX 是3.3V逻辑电平。
2. 确认 TX/RX 交叉并共地。
3. 确认双方均为 `9600 8N1`，无流控。
4. 确认 RuboVision 服务已启动。
5. 电控只发送一次 `61 31 0D 0A`。
6. 电控等待以 `0A` 结尾的响应。
7. 响应必须等于 `debug success\n`。
8. RuboVision Web 页面应出现 `uart_debug` 的执行记录。
9. Debug 通过后再逐项测试视觉命令。

香橙派检查命令：

```bash
systemctl is-active rubo-vision.service
sudo fuser -v /dev/ttyAMA1
sudo journalctl -u rubo-vision.service -f
```

正常状态：

- 服务状态是 `active`。
- `/dev/ttyAMA1` 由 `rubo_vision` 占用。
- 收到 Debug 帧后，日志包含 `binding=uart_debug func=debug_fun`。
- 日志包含 `sink.route.handled sink=web`。
- 日志包含 `sink.route.handled sink=uart`。

## 14. 常见错误定位

### 14.1 Engine 日志连续出现 key 97、49、13、10

说明 Engine 仍按单字节模式读取，未加载新的前缀/后缀配置。应确认部署配置为：

```toml
prefix = [97]
suffix = [13, 10]
content_bytes = 1
```

然后重新启动服务。

### 14.2 发送 `a1\r\n` 后出现 key 49，但 BindingNotFound

说明分帧正确，但缺少 `uart_debug` Binding，或者其 event 不是 `49`。

### 14.3 电控收到乱码

依次检查：

1. 波特率是否都是9600。
2. 是否为8N1。
3. 是否错误启用了奇偶校验或流控。
4. 是否把 RS-232/RS-485 接口当作 TTL UART。
5. 是否共地。

### 14.4 Engine 能接收，电控收不到返回

依次检查：

1. 香橙派 TX 是否接到电控 RX。
2. 电控接收是否已在发送前启动。
3. 电控是否错误地只接受 CRLF，而 Engine 只返回 LF。
4. 电控 RX 缓冲区是否溢出。
5. 日志是否出现 `sink.route.handled sink=uart`。

### 14.5 每秒出现一次 Debug 记录

说明电控正在周期发送 `a1\r\n`。硬件联调通过后应停止周期发送，改为业务事件触发，否则会持续产生 Web 历史记录和 UART 返回。

### 14.6 服务启动后无法手动读取串口

这是正常的。运行中的 RuboVision 会独占 `/dev/ttyAMA1`。原始串口监听前必须停止服务：

```bash
sudo systemctl stop rubo-vision.service
```

调试结束后恢复：

```bash
sudo systemctl start rubo-vision.service
```

## 15. 对实现 Agent 的最终约束

生成电控代码时必须满足以下检查项：

- 使用固定4字节请求帧。
- 直接写入二进制 COMMAND，不进行十进制字符串格式化。
- Debug 使用 `0x31`，颜色使用 `0x01`，两者不得混淆。
- 接收端以 `LF` 分行，并兼容可选 `CR`。
- 同一时间最多一条在途请求。
- 解析前验证响应与当前命令类型匹配。
- 对缓冲区溢出、超时、UTF-8失败和数值越界提供明确错误状态。
- 不在 UART ISR 中执行复杂解析或业务逻辑。
- 不假设响应固定长度。
- 不把图片或 JSON 作为 UART 返回内容处理。
- 不向香橙派 UART 输入5V逻辑电平。

如果协议配置发生变化，必须同步更新：

- `config/orangepi/source.toml`
- `config/orangepi/binding.toml`
- `src/config.rs` 中的代码声明配置
- 本文档中的命令表和测试向量
