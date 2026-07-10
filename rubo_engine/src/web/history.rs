use std::collections::VecDeque;

use crate::WebOutputFrame;

#[derive(Debug, Clone)]
pub struct WebHistory {
    limit: usize,
    frames: VecDeque<WebOutputFrame>,
}

impl WebHistory {
    pub fn new(limit: usize) -> Self {
        Self {
            limit,
            frames: VecDeque::new(),
        }
    }

    pub fn push(&mut self, frame: WebOutputFrame) {
        self.frames.push_back(frame);
        while self.frames.len() > self.limit {
            self.frames.pop_front();
        }
    }

    pub fn latest(&self, limit: usize) -> Vec<WebOutputFrame> {
        self.frames.iter().rev().take(limit).cloned().collect()
    }

    pub fn get(&self, id: u64) -> Option<WebOutputFrame> {
        self.frames.iter().find(|frame| frame.id() == id).cloned()
    }

    pub fn count(&self) -> usize {
        self.frames.len()
    }

    pub fn error_count(&self) -> usize {
        self.frames
            .iter()
            .filter(|frame| frame.state().is_error())
            .count()
    }

    pub fn last_output_at_ms(&self) -> Option<u64> {
        self.frames.back().map(WebOutputFrame::created_at_ms)
    }

    pub fn all_latest_first(&self) -> Vec<WebOutputFrame> {
        self.frames.iter().rev().cloned().collect()
    }
}
