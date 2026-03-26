use std::sync::{Arc, Mutex, OnceLock};
use crate::global::{SharedMidi,};
use crate::midi::Midi;

pub fn are_ports_connected() -> bool {
    let midi = clone_midi();
    let midi_guard = midi.lock().unwrap();
    midi_guard.are_ports_connected()
}

pub fn clone_midi() -> SharedMidi {
    let midi = MIDI.get_or_init(|| Arc::new(Mutex::new(Midi::new())));
    Arc::clone(midi)
}

pub fn is_receiving_data() -> bool {
    let midi = clone_midi();
    let midi_guard = midi.lock().unwrap();
    midi_guard.is_receiving_data()
}

static MIDI: OnceLock<SharedMidi> = OnceLock::new();
