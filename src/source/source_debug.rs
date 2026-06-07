use std::collections::HashMap;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::sync::Arc;

use tokio::sync::mpsc::Sender;

use crate::config::binding::DebugBinding;
use crate::source::{Event, make_event_usual};

#[derive(Clone)]
pub struct DebugSource {
    bindings: Arc<HashMap<String, DebugBinding>>,
    sender: Sender<Event>,
}

impl DebugSource {
    pub fn new(bindings: Vec<DebugBinding>, sender: Sender<Event>) -> Self {
        let bindings = bindings
            .into_iter()
            .map(|binding| (binding.source_key.clone(), binding))
            .collect();

        Self {
            bindings: Arc::new(bindings),
            sender,
        }
    }

    pub fn bindings(&self) -> Vec<DebugBinding> {
        let mut bindings: Vec<_> = self.bindings.values().cloned().collect();
        bindings.sort_by(|left, right| left.source_key.cmp(&right.source_key));
        bindings
    }

    pub async fn trigger(&self, source_key: &str) -> Result<Event, DebugSourceError> {
        let binding = self
            .bindings
            .get(source_key)
            .ok_or_else(|| DebugSourceError::UnknownSourceKey(source_key.to_string()))?;
        let event = make_event_usual(&binding.task_id, &binding.function_id, &binding.device_id);
        self.sender
            .send(event.clone())
            .await
            .map_err(|_| DebugSourceError::EventChannelClosed)?;
        Ok(event)
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum DebugSourceError {
    UnknownSourceKey(String),
    EventChannelClosed,
}

impl Display for DebugSourceError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownSourceKey(source_key) => {
                write!(formatter, "unknown debug source key `{source_key}`")
            }
            Self::EventChannelClosed => write!(formatter, "task event channel is closed"),
        }
    }
}

impl Error for DebugSourceError {}

#[cfg(test)]
mod tests {
    use tokio::sync::mpsc;

    use crate::config::binding::DebugBinding;
    use crate::source::{DebugSource, DebugSourceError, Event};

    fn binding(source_key: &str) -> DebugBinding {
        DebugBinding {
            task_id: "debug_color_detect".to_string(),
            source_key: source_key.to_string(),
            device_id: "color_camera".to_string(),
            function_id: "color_detect".to_string(),
        }
    }

    #[tokio::test]
    async fn trigger_sends_configured_usual_event() {
        let (tx, mut rx) = mpsc::channel(1);
        let source = DebugSource::new(vec![binding("color")], tx);

        let event = source.trigger("color").await.expect("trigger accepted");

        assert_eq!(
            event,
            Event::UsualEvent(
                "debug_color_detect".to_string(),
                "color_detect".to_string(),
                "color_camera".to_string(),
            )
        );
        assert_eq!(rx.recv().await, Some(event));
    }

    #[tokio::test]
    async fn unknown_key_returns_not_found_without_sending_event() {
        let (tx, mut rx) = mpsc::channel(1);
        let source = DebugSource::new(vec![binding("color")], tx);

        let error = source
            .trigger("missing")
            .await
            .expect_err("unknown key must fail");

        assert!(matches!(error, DebugSourceError::UnknownSourceKey(key) if key == "missing"));
        assert!(matches!(
            rx.try_recv(),
            Err(mpsc::error::TryRecvError::Empty)
        ));
    }
}
