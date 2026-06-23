pub struct ErrorNotifier {
    has_error: bool,
}

impl ErrorNotifier {
    pub fn new() -> Self {
        Self {
            has_error: false,
        }
    }

    fn clear_error(&mut self) {
        self.has_error = false;
    }

    fn has_error(&self) -> bool {
        self.has_error
    }

    fn notify_error(&mut self) {
        self.has_error = true;
    }
}