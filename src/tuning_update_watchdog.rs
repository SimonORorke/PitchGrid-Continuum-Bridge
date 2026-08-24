use std::sync::mpsc;
use std::time::Duration;
use log::{debug, trace, warn};
use crate::error_notifier::SharedErrorNotifier;
use crate::presentation::Presentation;

/// Watches for the instrument's confirmation that a tuning update has been applied.
///
/// `start` is called when a tuning update is sent; `cancel` when the confirmation arrives. If no
/// confirmation arrives within 2 seconds, the watchdog reports the failure straight to the view.
/// The probable cause of a timeout is that MIDI output is not connected to one of the editor's
/// "Ext All Data" MIDI inputs.
pub struct TuningUpdateWatchdog {
    midi_send_error_notifier: SharedErrorNotifier,
    stopper_sender: Option<mpsc::Sender<()>>,
    is_awaiting: bool,
    presentation: Presentation,
    timeout_millis: u16,
}

impl TuningUpdateWatchdog {
    /// `timeout_millis` is the number of milliseconds to wait for a tuning update confirmation.
    /// It can be much shorter in tests. For the real-world value, set it to 2000.
    pub fn new(presentation: Presentation, timeout_millis: u16,
               midi_send_error_notifier: SharedErrorNotifier) -> Self {
        Self {
            stopper_sender: None,
            is_awaiting: false,
            presentation,
            timeout_millis,
            midi_send_error_notifier,
        }
    }

    /// Start (or restart) waiting for the tuning-update confirmation. The wait runs on a background
    /// thread, so it owns an `Arc` clone of the view rather than borrowing `self`.
    pub fn start(&mut self) {
        let (stopper_sender, stopper_receiver) = mpsc::channel();
        self.stopper_sender = Some(stopper_sender);
        self.is_awaiting = true;
        let presentation = self.presentation.clone();
        let timeout_millis = self.timeout_millis as u64;
        let midi_send_error_notifier = self.midi_send_error_notifier.clone();
        rayon::spawn(move || {
            Self::run(stopper_receiver, presentation, timeout_millis, midi_send_error_notifier);
        });
    }

    /// Cancel a pending wait because the tuning update has been confirmed.
    pub fn cancel(&mut self) {
        if self.is_awaiting {
            if let Some(stopper_sender) = self.stopper_sender.take() {
                // Ignore a send error: the watchdog may have already timed out and returned,
                // dropping the receiver. That is a normal outcome, not a failure.
                let _ = stopper_sender.send(());
            }
            self.is_awaiting = false;
        }
    }

    /// The watchdog thread body: wait for the confirmation signal, or report a timeout to the view.
    fn run(stopper_receiver: mpsc::Receiver<()>, presentation: Presentation, timeout_millis: u64,
           midi_send_error_notifier: SharedErrorNotifier) {
        // To test that INSTRUMENT_TUNING_UPDATE_NOT_CONFIRMED is shown on timeout,
        // uncomment the following two lines and comment out the next one.
        // if let Ok(_) = stopper_receiver.recv_timeout(Duration::from_millis(50)) {
        //     trace!("timeout_millis = {}", timeout_millis); // To prevent compiler warning unused timeout_millis
        if stopper_receiver.recv_timeout(Duration::from_millis(timeout_millis)).is_ok() {
            // Sleep was interrupted: tuning has been updated.
            debug!("Tuning updated");
            return;
        }
        warn!("Tuning update not confirmed");
        // Report straight to the view via the captured presentation handle. We deliberately do NOT
        // clear `is_awaiting` from here (this thread no longer holds the watchdog): it is cleared by
        // `cancel`, whose stop-signal send tolerates this thread having already exited and dropped
        // the receiver.
        let mut error_notifier = midi_send_error_notifier.lock().unwrap();
        if !error_notifier.has_error() {
            trace!("Showing tuning update not confirmed error");
            presentation.tuning_update_not_confirmed();
        } else {
            trace!("Showing MIDI send error");
            error_notifier.clear_error();
            presentation.midi_send_error();
        }
    }
}
