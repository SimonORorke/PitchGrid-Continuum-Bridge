use std::fmt;
use log::{error, trace};
use midly::{MidiMessage, live::LiveEvent};
use crate::i_midi_manager::SharedOutput;

/// A trait that defines the interface for sending MIDI messages.
///
/// For the `I` prefix, see `ITuner`s doc comment.
pub trait IMidiSender: std::fmt::Debug + Send + Sync {
    fn send_control_change(&self, channel: u8, cc_no: u8, value: u8);
    fn send_matrix_poke(&self, poke_id: u8, poke_value: u8);
}

/// A no-op sender used as the `Tuner`'s default until the real one is wired in (see
/// `Controller::new`). Sends are silently dropped, which is also the correct behaviour whenever no
/// MIDI output is connected.
#[derive(Clone, Debug)]
pub struct NullMidiSender;

impl IMidiSender for NullMidiSender {
    fn send_control_change(&self, _channel: u8, _cc_no: u8, _value: u8) {}
    fn send_matrix_poke(&self, _poke_id: u8, _poke_value: u8) {}
}

/// A service for sending MIDI messages to the instrument via the shared output connection.
#[derive(Clone)]
pub struct MidiSender {
    output: SharedOutput,
}

impl MidiSender {
    pub fn new(output: SharedOutput) -> Self {
        MidiSender { output }
    }

    /// Send a MIDI channel message.
    /// Parameter `channel` is 1-based.
    fn send_channel_message(&self, channel: u8, message: MidiMessage) {
        let live_event = LiveEvent::Midi {
            channel: (channel - 1).into(), // 0-based channel number.
            message,
        };
        let mut buf = Vec::new();
        live_event.write(&mut buf).unwrap();
        self.send_message(&buf[..]);
    }

    fn send_message(&self, message: &[u8]) {
        trace!("send_message: message={:?}", message);
        let mut connection_option = self.output.lock().unwrap();
        if let Some(connection) = connection_option.as_mut() {
            connection.send(message).unwrap_or_else(|_| {
                error!("Error when sending MIDI message: {:?}", message);
                // Panic for stack trace diagnostics.
                // panic!("Error when sending MIDI message: {:?}", message);
            });
        }
    }

    /// Send a MIDI note aftertouch (pressure) message.
    /// Parameter `channel` is 1-based.
    fn send_note_aftertouch(&self, channel: u8, key: u8, pressure: u8) {
        self.send_channel_message(
            channel,
            MidiMessage::Aftertouch {
                key: key.into(),
                vel: pressure.into(),
            },
        );
    }
}

// `MidiOutputConnection` is not `Debug`, so the trait's `Debug` bound is satisfied by hand.
impl fmt::Debug for MidiSender {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("MidiSender")
    }
}

impl IMidiSender for MidiSender {
    fn send_control_change(&self, channel: u8, cc_no: u8, value: u8) {
        self.send_channel_message(
            channel,
            MidiMessage::Controller {
                controller: cc_no.into(),
                value: value.into(),
            },
        );
    }

    fn send_matrix_poke(&self, poke_id: u8, poke_value: u8) {
        self.send_control_change(16, 56, 20); // Matrix Poke command
        self.send_note_aftertouch(16, poke_id, poke_value); // Perform the Poke
    }
}
