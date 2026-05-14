use std::cell::RefCell;
use pitchgrid_continuum::midi_sending::{IMidiSender};

/// MIDI send stats since `MockMidiSender` was instantiated.
pub fn sent_midi() -> SentMidi {
    SENT_MIDI.with(|s| s.borrow().clone())
}

#[derive(Clone, Debug)]
pub struct MockMidiSender {}

impl MockMidiSender {

    /// Creates a new `MockMidiSender` instance, resetting the MIDI send stats that are accessed
    /// via `sent_midi()`.
    pub fn new() -> Self {
        println!("MockMidiSender:new: resetting MIDI sent stats.");
        SENT_MIDI.replace(SentMidi::new());
        MockMidiSender {}
    }
}

impl IMidiSender for MockMidiSender {
    fn send_control_change(&self, channel: u8, cc_no: u8, value: u8) {
        SENT_MIDI.with_borrow_mut(|s| {
            s.control_change_count += 1;
            s.last_control_change_channel = channel;
            s.last_control_change_cc_no = cc_no;
            s.last_control_change_value = value;
        });
    }

    fn send_matrix_poke(&self, poke_id: u8, poke_value: u8) {
        SENT_MIDI.with_borrow_mut(|s| {
            println!("MockMidiSender:send_matrix_poke: s.matrix_poke_count = {}", s.matrix_poke_count);
            s.matrix_poke_count += 1;
            println!("MockMidiSender:send_matrix_poke: added 1 to s.matrix_poke_count, which now = {}", s.matrix_poke_count);
            s.last_matrix_poke_id = poke_id;
            s.last_matrix_poke_value = poke_value;
        });
    }
}

#[derive(Clone, Debug)]
pub struct SentMidi {
    pub control_change_count: u16,
    pub last_control_change_channel: u8,
    pub last_control_change_cc_no: u8,
    pub last_control_change_value: u8,
    pub last_matrix_poke_id: u8,
    pub last_matrix_poke_value: u8,
    pub matrix_poke_count: u16,
}

impl SentMidi {
    pub fn new() -> Self {
        SentMidi {
            control_change_count: 0,
            last_control_change_channel: 0,
            last_control_change_cc_no: 0,
            last_control_change_value: 0,
            last_matrix_poke_id: 0,
            last_matrix_poke_value: 0,
            matrix_poke_count: 0,
        }
    }
}

thread_local! {
    static SENT_MIDI: RefCell<SentMidi> = RefCell::new(SentMidi::new());
}
