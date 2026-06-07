use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use tokio::io::AsyncWriteExt;
use tokio::sync::{RwLock, mpsc};
use tracing::{debug, info, warn};

use crate::source::DebugSource;

use super::model::WebMessage;

#[derive(Clone)]
pub struct WebState {
    cache: Arc<RwLock<WebCache>>,
    history_limit: usize,
    next_id: Arc<AtomicU64>,
    persistence_tx: Option<mpsc::Sender<WebMessage>>,
    debug_source: Option<DebugSource>,
}

struct WebCache {
    latest: Option<WebMessage>,
    history: VecDeque<WebMessage>,
}

impl WebState {
    pub async fn new(history_limit: usize, persistence_path: impl Into<PathBuf>) -> Result<Self> {
        let persistence_path = persistence_path.into();
        let restored = load_history(&persistence_path, history_limit)
            .await
            .with_context(|| {
                format!(
                    "failed to load web message history from {}",
                    persistence_path.display()
                )
            })?;
        let latest = restored.front().cloned();
        let next_id = restored.iter().map(|message| message.id).max().unwrap_or(0) + 1;
        let (tx, rx) = mpsc::channel::<WebMessage>(128);

        spawn_persistence_writer(persistence_path, rx);

        Ok(Self {
            cache: Arc::new(RwLock::new(WebCache {
                latest,
                history: restored,
            })),
            history_limit,
            next_id: Arc::new(AtomicU64::new(next_id)),
            persistence_tx: Some(tx),
            debug_source: None,
        })
    }

    #[cfg(test)]
    pub fn in_memory(history_limit: usize) -> Self {
        Self {
            cache: Arc::new(RwLock::new(WebCache {
                latest: None,
                history: VecDeque::new(),
            })),
            history_limit,
            next_id: Arc::new(AtomicU64::new(1)),
            persistence_tx: None,
            debug_source: None,
        }
    }

    pub fn with_debug_source(mut self, debug_source: DebugSource) -> Self {
        self.debug_source = Some(debug_source);
        self
    }

    pub fn debug_source(&self) -> Option<&DebugSource> {
        self.debug_source.as_ref()
    }

    pub async fn push_message(&self, msg: WebMessage) {
        let msg = self.stamp_message(msg);

        {
            let mut cache = self.cache.write().await;
            cache.latest = Some(msg.clone());
            cache.history.push_front(msg.clone());
            while cache.history.len() > self.history_limit {
                cache.history.pop_back();
            }
        }

        if let Some(tx) = self.persistence_tx.as_ref()
            && let Err(e) = tx.send(msg).await
        {
            warn!(target: "web", "failed to queue web message persistence: {e}");
        }
    }

    pub async fn latest(&self) -> Option<WebMessage> {
        self.cache.read().await.latest.clone()
    }

    pub async fn history(&self) -> Vec<WebMessage> {
        self.cache.read().await.history.iter().cloned().collect()
    }

    fn stamp_message(&self, msg: WebMessage) -> WebMessage {
        let id = if msg.id == 0 {
            self.next_id.fetch_add(1, Ordering::Relaxed)
        } else {
            let next = msg.id.saturating_add(1);
            let _ = self
                .next_id
                .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |current| {
                    (current < next).then_some(next)
                });
            msg.id
        };

        let created_at_ms = if msg.created_at_ms == 0 {
            now_ms()
        } else {
            msg.created_at_ms
        };

        msg.with_runtime_meta(id, created_at_ms)
    }
}

async fn load_history(path: &Path, limit: usize) -> Result<VecDeque<WebMessage>> {
    if !path.exists() {
        return Ok(VecDeque::new());
    }

    let content = tokio::fs::read_to_string(path).await?;
    let mut messages = Vec::new();
    for (index, line) in content.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }

        match serde_json::from_str::<WebMessage>(line) {
            Ok(message) if message.id > 0 && message.code != 204 => messages.push(message),
            Ok(_) => {
                debug!(target: "web", "ignored empty persisted web message at line {}", index + 1)
            }
            Err(e) => warn!(target: "web", "ignored invalid web history line {}: {e}", index + 1),
        }
    }

    let mut history = VecDeque::new();
    for message in messages.into_iter().rev().take(limit) {
        history.push_back(message);
    }

    Ok(history)
}

fn spawn_persistence_writer(path: PathBuf, mut rx: mpsc::Receiver<WebMessage>) {
    tokio::spawn(async move {
        if let Some(parent) = path.parent()
            && let Err(e) = tokio::fs::create_dir_all(parent).await
        {
            warn!(
                target: "web",
                "failed to create web history directory {}: {e}",
                parent.display()
            );
            drain_without_persisting(&mut rx).await;
            return;
        }

        let mut file = match tokio::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .await
        {
            Ok(file) => file,
            Err(e) => {
                warn!(
                    target: "web",
                    "failed to open web history file {}: {e}",
                    path.display()
                );
                drain_without_persisting(&mut rx).await;
                return;
            }
        };

        info!(target: "web", "Web history persistence writer started: {}", path.display());
        while let Some(message) = rx.recv().await {
            match serde_json::to_string(&message) {
                Ok(line) => {
                    if let Err(e) = file.write_all(line.as_bytes()).await {
                        warn!(target: "web", "failed to write web history line: {e}");
                        continue;
                    }
                    if let Err(e) = file.write_all(b"\n").await {
                        warn!(target: "web", "failed to write web history newline: {e}");
                    }
                }
                Err(e) => warn!(target: "web", "failed to serialize web message: {e}"),
            }
        }
    });
}

async fn drain_without_persisting(rx: &mut mpsc::Receiver<WebMessage>) {
    while rx.recv().await.is_some() {}
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn push_message_sets_runtime_metadata_and_history() {
        let state = WebState::in_memory(2);

        state.push_message(WebMessage::ok("one")).await;
        state.push_message(WebMessage::error("two")).await;
        state.push_message(WebMessage::ok("three")).await;

        let latest = state.latest().await.expect("latest message");
        assert_eq!(latest.id, 3);
        assert!(latest.created_at_ms > 0);
        assert_eq!(latest.text, "three");

        let history = state.history().await;
        assert_eq!(history.len(), 2);
        assert_eq!(history[0].text, "three");
        assert_eq!(history[1].text, "two");
    }
}
