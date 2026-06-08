你是一名 Rust 后端/嵌入式通信开发助手。请帮我把当前 UART 命令解析逻辑从“字符串命令 + \n/\r 分隔”改造成更工程化的“二进制帧协议”。

背景：
当前项目中 UartSource 通过 uart.read(&mut buffer) 读取串口数据，然后把数据转成 String，追加到 pending，再通过 pending.find(['\n', '\r']) 解析命令。现在我们希望废弃字符串命令，改成固定二进制帧格式。

目标：
把 UART 命令协议改为固定 3 字节帧：

[HEAD, CMD, TAIL]

其中：
- HEAD 固定为 0xAA
- TAIL 固定为 0x55
- CMD 为 1 字节命令编号

命令编号暂定：
- 0x01：颜色识别任务
- 0x02：二维码识别任务
- 0x03：十字/路口识别任务
- 0x04：停止当前任务
- 0x05：状态查询/心跳，若当前项目没有对应逻辑，可以先预留，不要强行实现复杂业务

设计要求：
1. UART 是字节流，不能假设一次 read 刚好读到完整 3 字节命令。
2. pending 不再使用 String，改为 Vec<u8>。
3. read 成功后直接 pending.extend_from_slice(&buffer[..n])，不要再做 String::from_utf8_lossy。
4. 新的 dispatch_pending_commands 应该从 pending 中不断解析完整帧。
5. 解析逻辑要求：
   - 先在 pending 中查找 HEAD 0xAA。
   - 如果 HEAD 前面有垃圾字节，丢弃。
   - 如果剩余数据不足 3 字节，保留 pending，等待下一次 read。
   - 如果第 3 个字节不是 TAIL 0x55，说明帧错误，丢弃一个字节后继续重新找 HEAD，用于恢复同步。
   - 如果帧合法，取出 pending[1] 作为 cmd，删除这 3 个字节，然后分发命令。
6. 如果 pending 长度超过一个合理上限，例如 64 或 256 字节，要打印 warn 日志并清空，防止异常串口数据导致缓冲区无限增长。
7. binding_map 的 key 不再使用 String，建议改为 u8，也就是 HashMap<u8, UartBinding>。
8. dispatch_command 的参数也从 &str 改为 u8，内部根据 cmd 去 binding_map 查找。
9. 保持现有 UartSource 的整体结构、异步结构、日志风格、任务分发逻辑尽量不变，只替换 UART 命令解析协议层。
10. 请尽量少改业务逻辑，不要重构无关模块。
11. 修改后代码要能通过 cargo check。
12. 请补充必要的注释，说明 UART 是字节流，所以需要 pending 缓冲与帧同步恢复。
13. 如果项目里有配置文件中声明 UART binding 的地方，请把原来的字符串命令映射改为数字命令映射；如果不方便直接改配置结构，请给出兼容方案，例如配置仍写字符串，但初始化 binding_map 时转换成 u8。

请重点修改以下函数或相关调用链：
- UART read loop 中 pending 的类型和追加逻辑
- dispatch_pending_commands
- dispatch_command
- binding_map 的构建方式

新的核心解析逻辑可以参考：

const UART_FRAME_HEAD: u8 = 0xAA;
const UART_FRAME_TAIL: u8 = 0x55;
const UART_FRAME_LEN: usize = 3;
const UART_PENDING_MAX_LEN: usize = 64;

async fn dispatch_pending_commands(
    &self,
    pending: &mut Vec<u8>,
    binding_map: &HashMap<u8, UartBinding>,
) {
    loop {
        let Some(head_pos) = pending.iter().position(|&b| b == UART_FRAME_HEAD) else {
            if !pending.is_empty() {
                debug!("UartSource dropping bytes without frame head: {:?}", pending);
                pending.clear();
            }
            return;
        };

        if head_pos > 0 {
            debug!("UartSource dropping noise bytes before frame head: {:?}", &pending[..head_pos]);
            pending.drain(..head_pos);
        }

        if pending.len() < UART_FRAME_LEN {
            return;
        }

        if pending[2] != UART_FRAME_TAIL {
            warn!("UartSource invalid frame tail, dropping one byte: {:?}", &pending[..UART_FRAME_LEN]);
            pending.drain(..1);
            continue;
        }

        let cmd = pending[1];
        pending.drain(..UART_FRAME_LEN);

        self.dispatch_command(cmd, binding_map).await;
    }

    // 注意：如果 Rust 编译器提示 loop 后面的代码不可达，
    // pending 长度保护可以放在每次 read 追加之后，或放在函数开头。
}

read loop 可以参考：

let mut pending: Vec<u8> = Vec::new();
let mut buffer = [0u8; 64];

loop {
    match uart.read(&mut buffer) {
        Ok(0) => {
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        Ok(n) => {
            pending.extend_from_slice(&buffer[..n]);

            if pending.len() > UART_PENDING_MAX_LEN {
                warn!("UartSource pending frame buffer too long, clearing it");
                pending.clear();
                continue;
            }

            self.dispatch_pending_commands(&mut pending, &binding_map).await;
        }
        Err(e) => {
            warn!("UartSource read error: {:?}", e);
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }
}

请根据项目现有代码完成实际修改，而不是只给伪代码。最后请说明你改了哪些文件、哪些函数，以及新的 UART 发送端应该如何发送命令，例如：
- 颜色识别：AA 01 55
- 二维码识别：AA 02 55
- 十字/路口识别：AA 03 55
- 停止任务：AA 04 55