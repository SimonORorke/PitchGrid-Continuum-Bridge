use std::sync::{Arc, Mutex, LazyLock};
use log::trace;
use pitchgrid_continuum::error_notifier::{ErrorNotifier, SharedErrorNotifier};
use pitchgrid_continuum::midi_sender::IMidiSender;

/// Returns a snapshot of the MIDI send stats since the last `MockMidiSender::new()`.
pub fn mock_midi_sender() -> MockMidiSender {
    MOCK_MIDI_SENDER.lock().unwrap().clone()
}

#[derive(Clone, Debug)]
pub struct MockMidiSender {
    pub control_change_count: u16,
    pub control_change_channel: u8,
    pub control_change_cc_no: u8,
    pub control_change_value: u8,
    pub matrix_poke_count: u16,
    pub matrix_poke_id: u8,
    pub matrix_poke_value: u8,
    error_notifier: SharedErrorNotifier,
    simulate_error: bool,
}

impl MockMidiSender {
    fn new_state() -> Self {
        MockMidiSender {
            control_change_count: 0,
            control_change_channel: 0,
            control_change_cc_no: 0,
            control_change_value: 0,
            matrix_poke_count: 0,
            matrix_poke_id: 0,
            matrix_poke_value: 0,
            // Will be replaced by the usable error notifier in `set_error_notifier`.
            error_notifier: Arc::new(Mutex::new(ErrorNotifier::new())),
            simulate_error: false,
        }
    }

    /// Creates a new `MockMidiSender`, resetting the state accessed via `mock_midi_sender()`.
    // Factory: returns the trait object the SUT holds, not Self.
    #[allow(clippy::new_ret_no_self)]
    pub fn new() -> Box<dyn IMidiSender> {
        *MOCK_MIDI_SENDER.lock().unwrap() = MockMidiSender::new_state();
        Box::new(MockMidiSenderImpl {})
    }

    #[allow(dead_code)]
    // `allow(dead_code)` is required to silence a compiler warning,
    // as this function is used in the `presenter_tests` crate but not the `tuner_tests` crate.
    pub fn simulate_error(value: bool) {
        MOCK_MIDI_SENDER.lock().unwrap().simulate_error = value;
    }

    #[allow(dead_code)]
    // `allow(dead_code)` is required to silence a compiler warning,
    // as this function is used in the `presenter_tests` crate but not the `tuner_tests` crate.
    pub fn set_error_notifier(notifier: SharedErrorNotifier) {
        MOCK_MIDI_SENDER.lock().unwrap().error_notifier = notifier;
    }
}

#[derive(Debug)]
struct MockMidiSenderImpl {}

impl IMidiSender for MockMidiSenderImpl {
    fn error_notifier(&self) -> SharedErrorNotifier {
        MOCK_MIDI_SENDER.lock().unwrap().error_notifier.clone()
    }

    fn send_control_change(&mut self, channel: u8, cc_no: u8, value: u8) {
        let mut s = MOCK_MIDI_SENDER.lock().unwrap();
        trace!("send_control_change: channel={}, cc_no={}, value={}, simulate_error={}", channel, cc_no, value, s.simulate_error);
        s.control_change_count += 1;
        s.control_change_channel = channel;
        s.control_change_cc_no = cc_no;
        s.control_change_value = value;
        if s.simulate_error {
            s.error_notifier.lock().unwrap().notify_error();
        }
    }

    fn send_matrix_poke(&mut self, poke_id: u8, poke_value: u8) {
        let mut s = MOCK_MIDI_SENDER.lock().unwrap();
        s.matrix_poke_count += 1;
        s.matrix_poke_id = poke_id;
        s.matrix_poke_value = poke_value;
        if s.simulate_error {
            s.error_notifier.lock().unwrap().notify_error();
        }
    }
}

static MOCK_MIDI_SENDER: LazyLock<Mutex<MockMidiSender>> =
    LazyLock::new(|| Mutex::new(MockMidiSender::new_state()));
