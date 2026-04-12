use std::sync::{Arc, Mutex};
use crate::midi::Midi;

pub enum MessageType {
    Error,
    Info,
    Warning
}

#[derive(Clone, Debug)]
pub enum PortType {
    Input,
    Output,
}

pub fn override_note_names() -> Vec<String> {
    vec!["".to_string(),
         "F#".to_string(), "G".to_string(), "G#".to_string(),
         "A".to_string(), "A#".to_string(), "B".to_string(),
         "C".to_string(),
         "C#".to_string(),"D".to_string(), "D#".to_string(),
         "E".to_string(), "F".to_string(), ]
}

pub type SharedMidi = Arc<Mutex<Midi>>;
