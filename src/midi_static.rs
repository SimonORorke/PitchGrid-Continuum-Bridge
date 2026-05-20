use std::sync::{Arc, Mutex, OnceLock};
use crate::i_midi::{IMidi, SharedMidi};
use crate::midi::Midi;

pub struct MidiStatic;

impl MidiStatic {
    pub fn are_ports_connected() -> bool {
        let shared_midi = Self::midi_clone();
        let midi = shared_midi.lock().unwrap();
        midi.are_ports_connected()
    }

    pub fn close() {
        let shared_midi = Self::midi_clone();
        let mut midi = shared_midi.lock().unwrap();
        midi.close();
    }

    pub fn has_downloaded_init_data() -> bool {
        let shared_midi = Self::midi_clone();
        let midi = shared_midi.lock().unwrap();
        midi.has_downloaded_init_data()
    }

    pub fn is_output_port_connected() -> bool {
        let shared_midi = Self::midi_clone();
        let midi = shared_midi.lock().unwrap();
        midi.is_output_port_connected()
    }

    pub fn is_receiving_data() -> bool {
        let shared_midi = Self::midi_clone();
        let midi = shared_midi.lock().unwrap();
        midi.is_receiving_data()
    }

    /// Returns a clone of the thread-safe singleton Midi instance.
    pub fn midi_clone() -> SharedMidi {
        let midi =
            MIDI.get_or_init(|| Arc::new(Mutex::new(Box::new(Midi::new()) as Box<dyn IMidi + Send>)));
        Arc::clone(midi)
    }

    /// Replaces the default Midi instance for testing.
    pub fn set_midi(midi: Box<dyn IMidi + Send>) {
        *Self::midi_clone().lock().unwrap() = midi;
    }

    pub fn start_instrument_connection_monitor() {
        let shared_midi = Self::midi_clone();
        let mut midi = shared_midi.lock().unwrap();
        midi.start_instrument_connection_monitor();
    }

    pub fn stop_instrument_connection_monitor() {
        let shared_midi = Self::midi_clone();
        let mut midi = shared_midi.lock().unwrap();
        midi.stop_instrument_connection_monitor();
    }
}

static MIDI: OnceLock<SharedMidi> = OnceLock::new();
