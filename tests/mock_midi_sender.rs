use std::cell::RefCell;
use pitchgrid_continuum::midi_sender::IMidiSender;

/// Returns a snapshot of the MIDI send stats since the last `MockMidiSender::new()`.
pub fn mock_midi_sender() -> MockMidiSender {
    MOCK_MIDI_SENDER.with(|s| s.borrow().clone())
}

#[derive(Clone, Debug)]
pub struct MockMidiSender {
    pub control_change_count: u16,
    pub control_change_channel: u8,
    pub control_change_cc_no: u8,
    pub control_change_value: u8,
    has_error: bool,
    pub matrix_poke_count: u16,
    pub matrix_poke_id: u8,
    pub matrix_poke_value: u8,
}

impl MockMidiSender {
    fn new_state() -> Self {
        MockMidiSender {
            control_change_count: 0,
            control_change_channel: 0,
            control_change_cc_no: 0,
            control_change_value: 0,
            has_error: false,
            matrix_poke_count: 0,
            matrix_poke_id: 0,
            matrix_poke_value: 0,
        }
    }

    /// Creates a new `MockMidiSender`, resetting the state accessed via `mock_midi_sender()`.
    // Factory: returns the trait object the SUT holds, not Self.
    #[allow(clippy::new_ret_no_self)]
    pub fn new() -> Box<dyn IMidiSender> {
        MOCK_MIDI_SENDER.replace(MockMidiSender::new_state());
        Box::new(MockMidiSenderImpl {})
    }
}

#[derive(Debug)]
struct MockMidiSenderImpl {}

impl IMidiSender for MockMidiSenderImpl {
    fn clear_error(&mut self) {
        MOCK_MIDI_SENDER.with_borrow_mut(|s| {
            s.has_error = false;
        });
    }

    fn has_error(&self) -> bool {
        let mut result = false;
        MOCK_MIDI_SENDER.with_borrow(|s| {
            result = s.has_error;
        });
        result
    }

    fn send_control_change(&mut self, channel: u8, cc_no: u8, value: u8) {
        MOCK_MIDI_SENDER.with_borrow_mut(|s| {
            s.control_change_count += 1;
            s.control_change_channel = channel;
            s.control_change_cc_no = cc_no;
            s.control_change_value = value;
        });
    }

    fn send_matrix_poke(&mut self, poke_id: u8, poke_value: u8) {
        MOCK_MIDI_SENDER.with_borrow_mut(|s| {
            s.matrix_poke_count += 1;
            s.matrix_poke_id = poke_id;
            s.matrix_poke_value = poke_value;
        });
    }
}

thread_local! {
    static MOCK_MIDI_SENDER: RefCell<MockMidiSender> = RefCell::new(MockMidiSender::new_state());
}
