use crate::midi_message_batch::MidiMessageBatch;

/// A batch of Continuum configuration MIDI messages that can be accumulated and then released to
/// be sent to the Continuum instrument.
pub struct ContinuumMidiBatch {
    message_batch: MidiMessageBatch,
}

impl ContinuumMidiBatch {
    pub fn new() -> Self {
        Self {
            message_batch: MidiMessageBatch::new(),
        }
    }

    /// Adds a MIDI control change message to the batch.
    /// Parameter `channel` is 1-based.
    pub fn add_control_change(&mut self, channel: u8, cc_no: u8, value: u8) {
        self.message_batch.add_control_change(channel, cc_no, value);
    }

    /// Adds the MIDI messages for a matrix poke to the batch.
    pub fn add_matrix_poke(&mut self, poke_id: u8, poke_value: u8) {
        self.message_batch.add_control_change(16, 56, 20); // Matrix Poke command
        self.message_batch.add_note_aftertouch(16, poke_id, poke_value); // Perform the Poke
    }

    /// Returns all messages and clears the batch.
    pub fn release_to_send(&mut self) -> Vec<Box<[u8]>> {
        self.message_batch.release_to_send().clone()
    }
}