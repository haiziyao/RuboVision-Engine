use std::collections::VecDeque;
use std::sync::Arc;

use tokio::sync::RwLock;

use super::model::WebMessage;

#[derive(Clone)]
pub struct WebState {
    pub latest: Arc<RwLock<Option<WebMessage>>>,
    pub history: Arc<RwLock<VecDeque<WebMessage>>>,
    pub history_limit: usize,
}

impl WebState {
    pub fn new(history_limit: usize) -> Self {
        Self {
            latest: Arc::new(RwLock::new(None)),
            history: Arc::new(RwLock::new(VecDeque::new())),
            history_limit,
        }
    }

    pub async fn push_message(&self, msg: WebMessage) {
        {
            let mut latest = self.latest.write().await;
            *latest = Some(msg.clone());
        }

        {
            let mut history = self.history.write().await;
            history.push_front(msg);
            while history.len() > self.history_limit {
                history.pop_back();
            }
        }
    }
}
