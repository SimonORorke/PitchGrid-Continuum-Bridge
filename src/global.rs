use std::sync::{Arc, Mutex};
use crate::midi::Midi;

pub type SharedMidi = Arc<Mutex<Midi>>;
pub const APP_TITLE: &str = "PitchGrid-Continuum Companion";
