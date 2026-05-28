use std::sync::{Arc, Mutex, MutexGuard, OnceLock};
use crate::i_midi::{IMidi, SharedMidi};
use crate::midi::Midi;

pub struct MidiStatic;

impl MidiStatic {
    pub fn are_ports_connected() -> bool {
        Self::midi().are_ports_connected()
    }

    pub fn close() {
        Self::midi().close();
    }

    pub fn has_downloaded_init_data() -> bool {
        Self::midi().has_downloaded_init_data()
    }

    pub fn is_output_port_connected() -> bool {
        Self::midi().is_output_port_connected()
    }

    pub fn is_receiving_data() -> bool {
        Self::midi().is_receiving_data()
    }

    /// Locks and returns the singleton Midi instance.
    pub fn midi() -> MutexGuard<'static, Box<dyn IMidi + Send>> {
        Self::shared().lock().unwrap()
    }

    /// Returns a clone of the thread-safe singleton Midi instance.
    pub fn midi_clone() -> SharedMidi {
        Arc::clone(Self::shared())
    }

    /// Replaces the default Midi instance for testing.
    pub fn set_midi(midi: Box<dyn IMidi + Send>) {
        *Self::midi() = midi;
    }

    pub fn start_instrument_connection_monitor() {
        Self::midi().start_instrument_connection_monitor();
    }

    pub fn stop_instrument_connection_monitor() {
        Self::midi().stop_instrument_connection_monitor();
    }

    fn shared() -> &'static SharedMidi {
        MIDI.get_or_init(|| Arc::new(Mutex::new(Box::new(Midi::new()) as Box<dyn IMidi + Send>)))
    }
}

static MIDI: OnceLock<SharedMidi> = OnceLock::new();
