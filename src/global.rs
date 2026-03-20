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

// #[derive(Clone, Copy, Debug)]
// pub enum PresetLoading {
//     Replace = 0,
//     Preserve = 1,
// }

pub type SharedMidi = Arc<Mutex<Midi>>;
pub const APP_TITLE: &str = "PitchGrid-Continuum Bridge";
