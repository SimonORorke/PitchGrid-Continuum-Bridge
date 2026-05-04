use std::sync::{Arc, Mutex, OnceLock};
use crate::global::{SharedMidi,};
use crate::midi::Midi;

pub fn are_ports_connected() -> bool {
    let shared_midi = midi_clone();
    let midi = shared_midi.lock().unwrap();
    midi.are_ports_connected()
}

pub fn close() {
    let shared_midi = midi_clone();
    let mut midi = shared_midi.lock().unwrap();
    midi.close();
}

pub fn has_downloaded_init_data() -> bool {
    let shared_midi = midi_clone();
    let midi = shared_midi.lock().unwrap();
    midi.has_downloaded_init_data()
}

// pub fn is_downloading_init_data() -> bool {
//     let shared_midi = midi_clone();
//     let midi = shared_midi.lock().unwrap();
//     midi.is_downloading_init_data()
// }

pub fn is_output_port_connected() -> bool {
    let shared_midi = midi_clone();
    let midi = shared_midi.lock().unwrap();
    midi.is_output_port_connected()
}

pub fn is_receiving_data() -> bool {
    let shared_midi = midi_clone();
    let midi = shared_midi.lock().unwrap();
    midi.is_receiving_data()
}

/// Returns a clone of the thread-safe singleton Midi instance.
pub fn midi_clone() -> SharedMidi {
    let midi = MIDI.get_or_init(|| Arc::new(Mutex::new(Midi::new())));
    Arc::clone(midi)
}

pub fn start_instrument_connection_monitor() {
    let shared_midi = midi_clone();
    let mut midi = shared_midi.lock().unwrap();
    midi.start_instrument_connection_monitor();
}

pub fn stop_instrument_connection_monitor() {
    let shared_midi = midi_clone();
    let mut midi = shared_midi.lock().unwrap();
    midi.stop_instrument_connection_monitor();
}

static MIDI: OnceLock<SharedMidi> = OnceLock::new();
