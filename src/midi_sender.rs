use std::sync::{Arc, Mutex};
use std::thread::sleep;
use log::{error};
use midly::live::LiveEvent;
use midly::MidiMessage;
use crate::error_notifier::{ErrorNotifier, SharedErrorNotifier};
use crate::i_midi_manager::{SharedOutput};

/// A trait that defines the interface for sending MIDI messages.
///
/// For the `I` prefix, see `IUiMethods`s doc comment.
pub trait IMidiSender: Send {

    /// Returns a notifier of any MIDI send error.
    fn error_notifier(&self) -> SharedErrorNotifier;

    /// Sends a batch of MIDI messages.
    fn send_batch(&mut self, batch: Vec<Box<[u8]>>, delay_after_each_send_ms: u8);

    /// Sends a MIDI Control Change message.
    /// Parameter `channel` is 1-based.
    fn send_control_change(&mut self, channel: u8, cc_no: u8, value: u8, delay_after_send_ms: u8);

    /// Sends a single MIDI message.
    fn send_message(&mut self, message: &[u8], delay_after_send_ms: u8);
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

    /// Creates a MIDI control change message.
    /// Parameter `channel` is 1-based.
    pub fn create_control_change(channel: u8, cc_no: u8, value: u8) -> Vec<u8> {
        Self::create_channel_message(
            channel,
            MidiMessage::Controller {
                controller: cc_no.into(),
                value: value.into(),
            },
        )
   }

    /// Creates a MIDI note (polyphonic) aftertouch (pressure) message.
    /// Parameter `channel` is 1-based.
    pub fn create_note_aftertouch(channel: u8, key: u8, pressure: u8) -> Vec<u8> {
        Self::create_channel_message(
            channel,
            MidiMessage::Aftertouch {
                key: key.into(),
                vel: pressure.into(),
            },
        )
    }

    /// Creates a MIDI channel message.
    /// Parameter `channel` is 1-based.
    fn create_channel_message(channel: u8, message: MidiMessage) -> Vec<u8> {
        let live_event = LiveEvent::Midi {
            channel: (channel - 1).into(), // 0-based channel number.
            message,
        };
        let mut buf = Vec::new();
        live_event.write(&mut buf).unwrap();
        buf
    }
}

impl IMidiSender for MidiSender {
    /// Returns a notifier of any MIDI send error.
    fn error_notifier(&self) -> SharedErrorNotifier {
        self.error_notifier.clone()
    }

    /// Sends a batch of MIDI messages.
    fn send_batch(&mut self, batch: Vec<Box<[u8]>>, delay_after_each_send_ms: u8) {
        for message in batch {
            self.send_message(&message, delay_after_each_send_ms);
        }
    }

    fn send_control_change(&mut self, channel: u8, cc_no: u8, value: u8, delay_after_send_ms: u8) {
        self.send_message(&Self::create_control_change(channel, cc_no, value),
                          delay_after_send_ms);
    }

    /// Sends a single MIDI message.
    fn send_message(&mut self, message: &[u8], delay_after_send_ms: u8) {
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
            if delay_after_send_ms > 0 {
                sleep(std::time::Duration::from_millis(delay_after_send_ms as u64));
            }
        }
    }
}

pub type SharedMidiSender = Arc<Mutex<Box<dyn IMidiSender>>>;
