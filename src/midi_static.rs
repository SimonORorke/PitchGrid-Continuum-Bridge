use std::sync::{Arc, Mutex, OnceLock};
use crate::global::{SharedMidi,};
use crate::midi::Midi;

pub fn are_ports_connected() -> bool {
    let midi = clone_midi();
    let midi_guard = midi.lock().unwrap();
    midi_guard.are_ports_connected()
}

/// Returns a clone of the thread-safe singleton Midi instance.
pub fn clone_midi() -> SharedMidi {
    let midi = MIDI.get_or_init(|| Arc::new(Mutex::new(Midi::new())));
    Arc::clone(midi)
}

pub fn close() {
    let midi = clone_midi();
    let mut midi_guard = midi.lock().unwrap();
    midi_guard.close();
}

pub fn is_receiving_data() -> bool {
    let midi = clone_midi();
    let midi_guard = midi.lock().unwrap();
    midi_guard.is_receiving_data()
}

pub fn start_instru_connection_monitor() {
    let midi = clone_midi();
    let mut midi_guard = midi.lock().unwrap();
    midi_guard.start_instru_connection_monitor();
}

pub fn stop_instru_connection_monitor() {
    let midi = clone_midi();
    let mut midi_guard = midi.lock().unwrap();
    midi_guard.stop_instru_connection_monitor();
}

static MIDI: OnceLock<SharedMidi> = OnceLock::new();
