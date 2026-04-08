use axum::routing::any_service;
use tokio::sync::mpsc::error::SendError;
use tokio::sync::mpsc::Sender;
use tracing::info;

#[derive(Debug)]
#[derive(Eq, Hash, PartialEq)]
pub enum Event{
    UsualEvent(String,String,String),
    DebugEvent(String),
}

pub fn make_event_usual(task_id:&str,func_id:&str,device_id:&str) ->Event{
    Event::UsualEvent(task_id.to_string(),func_id.to_string(),device_id.to_string())
}
 

pub trait Source{

    fn base(&self) -> &BaseSource;
    fn base_mut(&mut self) -> &mut BaseSource;

    // TODO 不太清楚要不要加一个返回值，加返回值肯定易于排错。。。。
    fn set_sender(&mut self, tx: Sender<Event>){
        self.base_mut().sender = Some(tx);
        info!("Set sender successfully");
    }
    

    fn get_sender(&self) -> Option<&Sender<Event>>{
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