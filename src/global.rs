use std::sync::{Arc, Mutex};
use crate::midi::Midi;

pub enum MessageType {
    Error,
    Info,
    Warning
}

#[derive(Clone, Copy, Debug)]
pub enum PortType {
    Input,
    Output,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Rounding {
    None,
    Initial,
    Max,
}

pub fn default_rounding() -> Rounding { Rounding::Max }

pub fn override_note_names() -> Vec<String> {
    vec!["".to_string(),
         "F#".to_string(), "G".to_string(), "G#".to_string(),
         "A".to_string(), "A#".to_string(), "B".to_string(),
         "C".to_string(),
         "C#".to_string(),"D".to_string(), "D#".to_string(),
         "E".to_string(), "F".to_string(), ]
}

pub fn rounding_names() -> Vec<String> {
    vec!["None".to_string(), "Initial".to_string(), "Max".to_string(), ]
}

pub type SharedMidi = Arc<Mutex<Midi>>;
pub const APP_TITLE: &str = "PitchGrid-Continuum Bridge";
pub const VERSION: &str = env!("CARGO_PKG_VERSION");