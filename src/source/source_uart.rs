use std::collections::HashMap;
use std::time::Duration;

use crate::config::binding::UartBinding;
use crate::device::UartDeviceConfig;
use crate::source::{BaseSource, Source, make_event_usual};
use anyhow::anyhow;
use log::warn;
use tracing::{debug, info};

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
        uart_config: UartDeviceConfig,
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

        let binding_map: HashMap<String, UartBinding> = uart_binding
            .into_iter()
            .map(|binding| (binding.source_key.to_string(), binding))
            .collect();

        let mut uart = uart_config.open_uart()?;
        let mut buffer = [0u8; 64];
        let mut pending = String::new();

        info!(
            "UartSource listening on {} with {} command binding(s)",
            uart_config.serial,
            binding_map.len()
        );

        loop {
            match uart.read(&mut buffer) {
                Ok(0) => {
                    tokio::time::sleep(Duration::from_millis(20)).await;
                }
                Ok(n) => {
                    let chunk = String::from_utf8_lossy(&buffer[..n]);
                    pending.push_str(&chunk);
                    self.dispatch_pending_commands(&mut pending, &binding_map)
                        .await;
                }
                Err(e) => {
                    warn!("UartSource read error: {:?}", e);
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
            }
        }
    }

    async fn dispatch_pending_commands(
        &self,
        pending: &mut String,
        binding_map: &HashMap<String, UartBinding>,
    ) {
        while let Some(pos) = pending.find(['\n', '\r']) {
            let command = pending[..pos].trim().to_string();
            pending.drain(..=pos);
            if !command.is_empty() {
                self.dispatch_command(&command, binding_map).await;
            }
        }

        let command = pending.trim_matches(char::is_control).trim().to_string();
        if binding_map.contains_key(&command) {
            pending.clear();
            self.dispatch_command(&command, binding_map).await;
        } else if pending.len() > 256 {
            warn!("UartSource pending command buffer too long, clearing it");
            pending.clear();
        } else {
            debug!("UartSource pending command buffer: {:?}", pending);
        }
    }

    async fn dispatch_command(&self, command: &str, binding_map: &HashMap<String, UartBinding>) {
        let Some(bind) = binding_map.get(command) else {
            warn!("UartSource ignored unknown command {:?}", command);
            return;
        };

        let event = make_event_usual(
            bind.task_id.as_str(),
            bind.function_id.as_str(),
            bind.device_id.as_str(),
        );

        info!("UartSource dispatching command {:?} as {:?}", command, bind);

        match self.send(event).await {
            Ok(()) => info!("UartSource sent event {:?}", bind),
            Err(e) => warn!("UartSource send event error: {:?}", e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::UartSource;
    use crate::config::binding::UartBinding;
    use crate::device::{ResponseDeviceConfig, UartDeviceConfig, send_response_line};
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

        fn config(&self) -> Result<UartDeviceConfig> {
            UartDeviceConfig::new(self.engine.display().to_string(), 9600, 8, 1, false)
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
        let uart_config = pair.config()?;

        let source_task = tokio::spawn(async move { source.start(bindings, uart_config).await });
        tokio::time::sleep(Duration::from_millis(100)).await;

        let mut peer = pair.open_peer_write()?;
        peer.write_all(b"1\n")?;
        peer.flush()?;

        let event = tokio::time::timeout(Duration::from_secs(2), rx.recv())
            .await
            .context("timeout waiting uart event")?
            .context("uart event channel closed")?;
        assert_eq!(
            event,
            Event::UsualEvent(
                "uart_test_task".to_string(),
                "color_detect".to_string(),
                "uart_test_camera".to_string()
            )
        );

        source_task.abort();
        Ok(())
    }

    #[test]
    fn test_uart_response_writes_line_to_virtual_serial() -> Result<()> {
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

        let response = ResponseDeviceConfig {
            uart: pair.config()?,
            lights: None,
        };

        send_response_line(&response, "ok:42")?;
        let line = line_rx
            .recv_timeout(Duration::from_secs(2))
            .context("timeout waiting for uart output")??;

        drop(pair);
        let _ = reader.join();
        assert_eq!(line, "ok:42\n");
        Ok(())
    }
}
