use tokio::sync::mpsc::Sender;
use tokio::sync::mpsc::error::SendError;
use tracing::info;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum Event {
    UsualEvent {
        task_id: String,
        function_id: String,
        device_id: String,
        runtime_param: u8,
    },
    DebugEvent(String),
    #[allow(dead_code)]
    OtherEvent(String),
}

pub fn make_event_usual(task_id: &str, func_id: &str, device_id: &str, runtime_param: u8) -> Event {
    Event::UsualEvent {
        task_id: task_id.to_string(),
        function_id: func_id.to_string(),
        device_id: device_id.to_string(),
        runtime_param,
    }
}

pub trait Source {
    fn base(&self) -> &BaseSource;
    fn base_mut(&mut self) -> &mut BaseSource;

    // TODO 不太清楚要不要加一个返回值，加返回值肯定易于排错。。。。
    fn set_sender(&mut self, tx: Sender<Event>) {
        self.base_mut().sender = Some(tx);
        info!("Set sender successfully");
    }

    fn get_sender(&self) -> Option<&Sender<Event>> {
        self.base().sender.as_ref()
    }

    async fn send(&self, event: Event) -> Result<(), SendError<Event>> {
        match self.get_sender() {
            Some(sender) => sender.send(event).await,
            None => Err(SendError(event)),
        }
    }
}

#[derive(Default)]
pub struct BaseSource {
    pub sender: Option<Sender<Event>>,
}

#[cfg(test)]
mod tests {
    use super::{Event, make_event_usual};

    #[test]
    fn usual_event_keeps_runtime_param() {
        assert_eq!(
            make_event_usual("task", "cross", "camera", 5),
            Event::UsualEvent {
                task_id: "task".to_string(),
                function_id: "cross".to_string(),
                device_id: "camera".to_string(),
                runtime_param: 5,
            }
        );
    }
}
