use std::sync::mpsc;
use std::time::Duration;
use log::{debug, warn};
use crate::presentation::Presentation;

/// Watches for the instrument's confirmation that a tuning update has been applied.
///
/// `start` is called when a tuning update is sent; `cancel` when the confirmation arrives. If no
/// confirmation arrives within 2 seconds, the watchdog reports the failure straight to the view.
/// The probable cause of a timeout is that MIDI output is not connected to one of the editor's
/// "Ext All Data" MIDI inputs.
///
/// The 2-second timeout is currently hard-coded; injecting it (so tests can use a much shorter wait)
/// is a candidate follow-up.
pub struct TuningUpdateWatchdog {
    stopper_sender: Option<mpsc::Sender<()>>,
    is_awaiting: bool,
    presentation: Presentation,
}

impl TuningUpdateWatchdog {
    pub fn new(presentation: Presentation) -> Self {
        Self {
            stopper_sender: None,
            is_awaiting: false,
            presentation,
        }
    }

    /// Start (or restart) waiting for the tuning-update confirmation. The wait runs on a background
    /// thread, so it owns an `Arc` clone of the view rather than borrowing `self`.
    pub fn start(&mut self) {
        let (stopper_sender, stopper_receiver) = mpsc::channel();
        self.stopper_sender = Some(stopper_sender);
        self.is_awaiting = true;
        let presentation = self.presentation.clone();
        rayon::spawn(move || {
            Self::run(stopper_receiver, presentation);
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
    fn run(stopper_receiver: mpsc::Receiver<()>, presentation: Presentation) {
        // There's one scenario where this check is known not to behave as expected.
        // Editor MIDI:
        //     Input  LB1 (A)
        //     Output LB2 (A)
        // As we are using loopback endpoints, the following are the correct MIDI connections in
        // this application:
        //     Input  LB2 (B)
        //     Output LB1 (B)
        // But try the following MIDI connections in this application:
        //     Input  LB2 (B)
        //     Output LB2 (A)
        // In this scenario, this application's MIDI input is correct, but the incorrect output is
        // the same as the editor's output.
        // Expected behaviour:
        //     As our output is incorrect, the instrument tuning and the tuning shown in the
        //     editor should not be updated.
        //     We should not receive confirmation that the tuning has been updated.
        // Actual behavour:
        //     As with the expected behaviour, the instrument tuning and the tuning shown in the
        //     editor are not updated.
        //     However, we receive Grid message ch16 cc51 g, where g is our seleted pitch table
        //     number. We interpret this as confirmation that the tuning has been updated.
        //
        // Explanation
        //
        // Something like the following must be happening.
        // As Windows MIDI devices are currently shared with no way to make them exclusive,
        // there's nothing to stop us sending MIDI direct to the instrument, bypassing the editor.
        // But from the instrument's perspective, our tuning data looks like invalid data from
        // the editor, rather than a valid request from an external software component.
        // So the firmware does not implement the request.
        // As we request the current preset to be updated with the tuning with the same cc51 Grid
        // message, what we currently interpret as update confirmation is just our
        // request echoed back, which is expected. There is currently a firmware bug where,
        // for some presets, the confirmation message is not sent when the preset's tuning has been
        // updated. Our temporary workaround is to treat the echoed back request as confirmation.
        //
        // Pending fix
        //
        // Once the firmware bug is fixed, in MidiManager.on_message_received we can remove the workaround
        // and revert to interpreting not the first cc51 Grid message received, our request, but
        // the second as confirmation.
        // That should make the problem go away. I've tested it with a preset that still sends
        // the confirmation message even with the firmware bug.
        // To test that INSTRUMENT_TUNING_UPDATE_NOT_CONFIRMED is shown on timeout,
        // uncomment the following line and comment out the next one.
        // if let Ok(_) = stopper_receiver.recv_timeout(Duration::from_millis(50)) {
        if stopper_receiver.recv_timeout(Duration::from_secs(2)).is_ok() {
            // Sleep was interrupted: tuning has been updated.
            debug!("Tuning updated");
            return;
        }
        warn!("Tuning update not confirmed");
        // Report straight to the view via the captured presentation handle. We deliberately do NOT
        // clear `is_awaiting` from here (this thread no longer holds the watchdog): it is cleared by
        // `cancel`, whose stop-signal send tolerates this thread having already exited and dropped
        // the receiver.
        presentation.tuning_update_not_confirmed();
    }
}
