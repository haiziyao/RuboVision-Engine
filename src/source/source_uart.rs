use std::collections::HashMap;

use crate::config::binding::UartBinding;
use crate::source::{BaseSource, Source, make_event_usual};
use anyhow::anyhow;
use log::warn;
use tokio::sync::mpsc;
use tracing::{debug, info};

const UART_FRAME_HEAD: u8 = 0xAA;
const UART_FRAME_TAIL: u8 = 0x55;
const UART_FRAME_LEN: usize = 3;
const UART_PENDING_MAX_LEN: usize = 64;
const UART_COMMAND_STOP_RESERVED: u8 = 0x04;
const UART_COMMAND_STATUS_RESERVED: u8 = 0x05;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum UartCommandKind {
    Task(u8),
    ReservedStop,
    ReservedStatus,
}

#[derive(Default)]
pub struct UartSource {
    pub base: BaseSource,
    #[allow(dead_code)]
    pub port: String,
}

impl Source for UartSource {
    fn base(&self) -> &BaseSource {
        &self.base
    }

    fn base_mut(&mut self) -> &mut BaseSource {
        &mut self.base
    }
}
#[warn(unused)]
impl UartSource {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn start(
        &self,
        uart_binding: Vec<UartBinding>,
        mut incoming: mpsc::Receiver<Vec<u8>>,
    ) -> anyhow::Result<()> {
        // to get the sender
        let Some(_tx) = self.get_sender() else {
            warn!("LoopSource.listen called before sender was initialized");
            return Err(anyhow!("source sender is not initialized"));
        };

        let Some(_) = uart_binding.first() else {
            info!("UartSource has no binding, skipped");
            return Ok(());
        };

        let binding_map: HashMap<u8, UartBinding> = uart_binding
            .into_iter()
            .map(|binding| (binding.source_key, binding))
            .collect();

        // UART is a byte stream, so reads can split or combine frames. Keep
        // pending bytes until complete frames can be synchronized and parsed.
        let mut pending = Vec::new();

        info!(
            "UartSource listening with {} command binding(s)",
            binding_map.len()
        );

        while let Some(bytes) = incoming.recv().await {
            pending.extend_from_slice(&bytes);
            self.dispatch_pending_commands(&mut pending, &binding_map)
                .await;
        }

        Err(anyhow!("UART transport input channel closed"))
    }

    async fn dispatch_pending_commands(
        &self,
        pending: &mut Vec<u8>,
        binding_map: &HashMap<u8, UartBinding>,
    ) {
        for command in take_uart_commands(pending) {
            self.dispatch_command(command, binding_map).await;
        }
    }

    async fn dispatch_command(&self, command: u8, binding_map: &HashMap<u8, UartBinding>) {
        match classify_uart_command(command) {
            UartCommandKind::ReservedStop => {
                info!("UartSource received reserved stop command 0x04");
                return;
            }
            UartCommandKind::ReservedStatus => {
                info!("UartSource received reserved status command 0x05");
                return;
            }
            UartCommandKind::Task(_) => {}
        }

        let Some(bind) = binding_map.get(&command) else {
            warn!("UartSource ignored unknown command 0x{command:02X}");
            return;
        };

        let event = make_event_usual(
            bind.task_id.as_str(),
            bind.function_id.as_str(),
            bind.device_id.as_str(),
            0,
        );

        info!(
            "UartSource dispatching command 0x{command:02X} as {:?}",
            bind
        );

        match self.send(event).await {
            Ok(()) => info!("UartSource sent event {:?}", bind),
            Err(e) => warn!("UartSource send event error: {:?}", e),
        }
    }
}

fn classify_uart_command(command: u8) -> UartCommandKind {
    match command {
        UART_COMMAND_STOP_RESERVED => UartCommandKind::ReservedStop,
        UART_COMMAND_STATUS_RESERVED => UartCommandKind::ReservedStatus,
        command => UartCommandKind::Task(command),
    }
}

fn take_uart_commands(pending: &mut Vec<u8>) -> Vec<u8> {
    if pending.len() > UART_PENDING_MAX_LEN {
        warn!(
            "UartSource pending frame buffer too long ({} bytes), clearing it",
            pending.len()
        );
        pending.clear();
        return Vec::new();
    }

    let mut commands = Vec::new();
    loop {
        let Some(head_pos) = pending.iter().position(|&byte| byte == UART_FRAME_HEAD) else {
            if !pending.is_empty() {
                debug!(
                    "UartSource dropping bytes without frame head: {:?}",
                    pending
                );
                pending.clear();
            }
            break;
        };

        if head_pos > 0 {
            debug!(
                "UartSource dropping noise bytes before frame head: {:?}",
                &pending[..head_pos]
            );
            pending.drain(..head_pos);
        }

        if pending.len() < UART_FRAME_LEN {
            break;
        }

        if pending[2] != UART_FRAME_TAIL {
            warn!(
                "UartSource invalid frame tail, dropping one byte: {:?}",
                &pending[..UART_FRAME_LEN]
            );
            pending.drain(..1);
            continue;
        }

        commands.push(pending[1]);
        pending.drain(..UART_FRAME_LEN);
    }

    commands
}

#[cfg(test)]
mod tests {
    use super::{
        UART_PENDING_MAX_LEN, UartCommandKind, UartSource, classify_uart_command,
        take_uart_commands,
    };
    use crate::config::UartConfig;
    use crate::config::binding::UartBinding;
    use crate::message::{UartSink, start_uart_transport};
    use crate::source::{Event, Source};
    use anyhow::{Context, Result, anyhow, bail};
    use std::fs::{self, File, OpenOptions};
    use std::io::{Read, Write};
    use std::path::PathBuf;
    use std::process::{Child, Command, Stdio};
    use std::sync::mpsc as std_mpsc;
    use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
    use tokio::sync::mpsc;

    struct VirtualSerialPair {
        engine: PathBuf,
        peer: PathBuf,
        dir: PathBuf,
        child: Child,
    }

    impl VirtualSerialPair {
        fn new(name: &str) -> Result<Option<Self>> {
            if !socat_available() {
                eprintln!("socat not found; skip virtual uart test");
                return Ok(None);
            }

            let stamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .context("system time before unix epoch")?
                .as_nanos();
            let dir = std::env::temp_dir().join(format!(
                "rubovision-uart-{name}-{}-{stamp}",
                std::process::id()
            ));
            fs::create_dir_all(&dir)
                .with_context(|| format!("failed to create {}", dir.display()))?;

            let engine = dir.join("engine");
            let peer = dir.join("peer");
            let child = Command::new("socat")
                .arg("-d")
                .arg("-d")
                .arg(format!("pty,raw,echo=0,link={}", engine.display()))
                .arg(format!("pty,raw,echo=0,link={}", peer.display()))
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .context("failed to start socat")?;

            let mut pair = Self {
                engine,
                peer,
                dir,
                child,
            };
            pair.wait_ready()?;
            Ok(Some(pair))
        }

        fn config(&self) -> UartConfig {
            UartConfig {
                on: true,
                serial: self.engine.display().to_string(),
                baud: 9600,
                data_bit: 8,
                stop_bit: 1,
                parity_bit: false,
            }
        }

        fn open_peer_write(&self) -> Result<File> {
            OpenOptions::new()
                .write(true)
                .open(&self.peer)
                .with_context(|| format!("failed to open peer for write {}", self.peer.display()))
        }

        fn wait_ready(&mut self) -> Result<()> {
            let deadline = Instant::now() + Duration::from_secs(2);
            while Instant::now() < deadline {
                if self.engine.exists() && self.peer.exists() {
                    return Ok(());
                }
                if let Some(status) = self.child.try_wait()? {
                    return Err(anyhow!("socat exited before creating ptys: {status}"));
                }
                std::thread::sleep(Duration::from_millis(20));
            }
            bail!(
                "timeout waiting for virtual serial pair {} and {}",
                self.engine.display(),
                self.peer.display()
            )
        }
    }

    impl Drop for VirtualSerialPair {
        fn drop(&mut self) {
            let _ = self.child.kill();
            let _ = self.child.wait();
            let _ = fs::remove_dir_all(&self.dir);
        }
    }

    fn socat_available() -> bool {
        Command::new("socat")
            .arg("-V")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|status| status.success())
            .unwrap_or(false)
    }

    fn read_line_blocking(mut file: File) -> Result<String> {
        let mut bytes = Vec::new();
        let mut buf = [0u8; 64];

        loop {
            match file.read(&mut buf) {
                Ok(0) => std::thread::sleep(Duration::from_millis(20)),
                Ok(n) => {
                    bytes.extend_from_slice(&buf[..n]);
                    if bytes.contains(&b'\n') {
                        return String::from_utf8(bytes).context("uart output is not utf8");
                    }
                }
                Err(e) => return Err(e).context("failed to read peer uart"),
            }
        }
    }

    #[test]
    fn uart_frame_parser_preserves_partial_frame() {
        let mut pending = vec![0xAA, 0x01];

        assert!(take_uart_commands(&mut pending).is_empty());
        assert_eq!(pending, vec![0xAA, 0x01]);

        pending.push(0x55);
        assert_eq!(take_uart_commands(&mut pending), vec![0x01]);
        assert!(pending.is_empty());
    }

    #[test]
    fn uart_frame_parser_handles_multiple_frames() {
        let mut pending = vec![0xAA, 0x01, 0x55, 0xAA, 0x02, 0x55];

        assert_eq!(take_uart_commands(&mut pending), vec![0x01, 0x02]);
        assert!(pending.is_empty());
    }

    #[test]
    fn uart_frame_parser_drops_noise_and_recovers_from_bad_tail() {
        let mut pending = vec![0x00, 0x99, 0xAA, 0x01, 0x00, 0xAA, 0x03, 0x55];

        assert_eq!(take_uart_commands(&mut pending), vec![0x03]);
        assert!(pending.is_empty());
    }

    #[test]
    fn uart_frame_parser_clears_oversized_pending_data() {
        let mut pending = vec![0xAA; UART_PENDING_MAX_LEN + 1];

        assert!(take_uart_commands(&mut pending).is_empty());
        assert!(pending.is_empty());
    }

    #[test]
    fn uart_reserved_commands_are_classified_without_task_dispatch() {
        assert_eq!(classify_uart_command(0x04), UartCommandKind::ReservedStop);
        assert_eq!(classify_uart_command(0x05), UartCommandKind::ReservedStatus);
        assert_eq!(classify_uart_command(0x01), UartCommandKind::Task(0x01));
    }

    #[tokio::test]
    async fn test_uart_source_receives_command_from_virtual_serial() -> Result<()> {
        let Some(pair) = VirtualSerialPair::new("rx")? else {
            return Ok(());
        };

        let (tx, mut rx) = mpsc::channel(4);
        let mut source = UartSource::new();
        source.set_sender(tx);

        let bindings = vec![UartBinding {
            task_id: "uart_test_task".to_string(),
            source_key: 1,
            device_id: "uart_test_camera".to_string(),
            function_id: "color_detect".to_string(),
        }];
        let uart_channels = start_uart_transport(&pair.config())?;

        let source_task =
            tokio::spawn(async move { source.start(bindings, uart_channels.incoming).await });
        tokio::time::sleep(Duration::from_millis(100)).await;

        let mut peer = pair.open_peer_write()?;
        peer.write_all(&[0xAA, 0x01, 0x55])?;
        peer.flush()?;

        let event = tokio::time::timeout(Duration::from_secs(2), rx.recv())
            .await
            .context("timeout waiting uart event")?
            .context("uart event channel closed")?;
        assert_eq!(
            event,
            Event::UsualEvent {
                task_id: "uart_test_task".to_string(),
                function_id: "color_detect".to_string(),
                device_id: "uart_test_camera".to_string(),
                runtime_param: 0,
            }
        );

        source_task.abort();
        Ok(())
    }

    #[tokio::test]
    async fn test_uart_response_writes_line_to_virtual_serial() -> Result<()> {
        let Some(pair) = VirtualSerialPair::new("tx")? else {
            return Ok(());
        };

        let peer = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&pair.peer)
            .with_context(|| format!("failed to open peer for read {}", pair.peer.display()))?;
        let (line_tx, line_rx) = std_mpsc::channel();
        let reader = std::thread::spawn(move || {
            let _ = line_tx.send(read_line_blocking(peer));
        });

        let uart_channels = start_uart_transport(&pair.config())?;
        let sink = UartSink::new(uart_channels.outgoing);

        sink.send_value("ok:42").await?;
        let line = line_rx
            .recv_timeout(Duration::from_secs(2))
            .context("timeout waiting for uart output")??;

        drop(pair);
        let _ = reader.join();
        assert_eq!(line, "ok:42\n");
        Ok(())
    }
}
