use std::sync::{Arc, Mutex, OnceLock};
use crate::global::{SharedMidi,};
use crate::midi::Midi;

pub fn are_ports_connected() -> bool {
    let midi = midi_clone();
    let midi_guard = midi.lock().unwrap();
    midi_guard.are_ports_connected()
}

pub fn close() {
    let midi = midi_clone();
    let mut midi_guard = midi.lock().unwrap();
    midi_guard.close();
}

pub fn is_receiving_data() -> bool {
    let midi = midi_clone();
    let midi_guard = midi.lock().unwrap();
    midi_guard.is_receiving_data()
}

/// Returns a clone of the thread-safe singleton Midi instance.
pub fn midi_clone() -> SharedMidi {
    let midi = MIDI.get_or_init(|| Arc::new(Mutex::new(Midi::new())));
    Arc::clone(midi)
}

pub fn start_instru_connection_monitor() {
    let midi = midi_clone();
    let mut midi_guard = midi.lock().unwrap();
    midi_guard.start_instru_connection_monitor();
}

pub fn stop_instru_connection_monitor() {
    let midi = midi_clone();
    let mut midi_guard = midi.lock().unwrap();
    midi_guard.stop_instru_connection_monitor();
}

static MIDI: OnceLock<SharedMidi> = OnceLock::new();
