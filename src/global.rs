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

pub type SharedMidi = Arc<Mutex<Midi>>;
pub const APP_TITLE: &str = "PitchGrid-Continuum Bridge";
pub const VERSION: &str = env!("CARGO_PKG_VERSION");