use std::sync::{Arc, Mutex};
use log::{error};
use crate::error_notifier::{ErrorNotifier, SharedErrorNotifier};
use crate::i_midi_manager::{SharedOutput};

/// A trait that defines the interface for sending MIDI messages.
///
/// For the `I` prefix, see `IUiMethods`s doc comment.
pub trait IMidiSender: Send {
// pub trait IMidiSender: std::fmt::Debug + Send + Sync {

    /// Returns a notifier of any MIDI send error.
    fn error_notifier(&self) -> SharedErrorNotifier;

    /// Sends a batch of MIDI messages.
    fn send_batch(&mut self, batch: Vec<Box<[u8]>>);

    /// Sends a single MIDI message.
    fn send_message(&mut self, message: &[u8]);
}

/// A service for sending MIDI messages to the instrument via the shared output connection.
pub struct MidiSender {
    output: SharedOutput,
    error_notifier: SharedErrorNotifier,
}

impl MidiSender {
    pub fn new(output: SharedOutput) -> Self {
        MidiSender {
            output,
            error_notifier: Arc::new(Mutex::new(ErrorNotifier::new())),
        }
    }
}

// // `MidiOutputConnection` is not `Debug`, so the trait's `Debug` bound is satisfied by hand.
// impl fmt::Debug for MidiSender {
//     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//         f.write_str("MidiSender")
//     }
// }

impl IMidiSender for MidiSender {
    /// Returns a notifier of any MIDI send error.
    fn error_notifier(&self) -> SharedErrorNotifier {
        self.error_notifier.clone()
    }

    /// Sends a batch of MIDI messages.
    fn send_batch(&mut self, batch: Vec<Box<[u8]>>) {
        for message in batch {
            self.send_message(&message);
        }
    }

    /// Sends a single MIDI message.
    fn send_message(&mut self, message: &[u8]) {
        let mut connection_option =
            self.output.lock().unwrap();
        if let Some(connection) = connection_option.as_mut() {
            connection.send(message).unwrap_or_else(|_| {
                // MIDI send errors are rare and unreproducible, but they do happen.
                // I think it may sometimes happen when turning the instrument off and on again
                // while everything is connected. I'm normally doing that for testing.
                // But a user might need to do it too: some glitches that occur on the instrument
                // are fixed if the instrument is bounced.
                // So it's worth reporting the error to the user, suggesting a fix.
                error!("Error when sending MIDI message: {:?}", message);
                self.error_notifier.lock().unwrap().notify_error();
                // Panic for stack trace diagnostics.
                // panic!("Error when sending MIDI message: {:?}", message);
            });
        }
    }
}

pub type SharedMidiSender = Arc<Mutex<Box<dyn IMidiSender>>>;
