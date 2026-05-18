use crate::midi::Midi;

/// A trait that defines the interface for sending MIDI messages.
///
/// For the The `I` prefix, see `ITuner`s doc comment.
pub trait IMidiSender: std::fmt::Debug + Send + Sync {
    fn send_control_change(&self, channel: u8, cc_no: u8, value: u8);
    fn send_matrix_poke(&self, poke_id: u8, poke_value: u8);
}


/// A service for sending MIDI messages.
#[derive(Clone, Debug)]
pub struct MidiSender {
}

impl MidiSender {
    pub fn new() -> Self {
        MidiSender {}
    }
}

impl IMidiSender for MidiSender {
    fn send_control_change(&self, channel: u8, cc_no: u8, value: u8) {
        Midi::send_control_change(channel, cc_no, value); // Grid
    }

    fn send_matrix_poke(&self, poke_id: u8, poke_value: u8) {
        Midi::send_matrix_poke(poke_id, poke_value);
    }
}