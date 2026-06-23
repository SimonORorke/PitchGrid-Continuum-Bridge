use std::sync::{Arc, Mutex};

#[derive(Clone, Debug)]
pub struct ErrorNotifier {
    has_error: bool,
}

impl ErrorNotifier {
    pub fn new() -> Self {
        Self {
            has_error: false,
        }
    }

    pub fn clear_error(&mut self) {
        self.has_error = false;
    }

    pub fn has_error(&self) -> bool {
        self.has_error
    }

    pub fn notify_error(&mut self) {
        self.has_error = true;
    }
}

pub type SharedErrorNotifier = Arc<Mutex<ErrorNotifier>>;
