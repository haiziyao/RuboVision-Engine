
use std::sync::Arc;

use tokio::sync::{Mutex, mpsc};

use super::model::WebMessage;

#[derive(Clone)]
pub struct WebState {
    pub rx: Arc<Mutex<mpsc::Receiver<WebMessage>>>,
}

impl WebState {
    pub fn new(rx: mpsc::Receiver<WebMessage>) -> Self {
        Self {
            rx: Arc::new(Mutex::new(rx)),
        }
    }
}
