use std::any::Any;
use std::fmt::Display;
use midir::{MidiInputPort, MidiOutputPort};

pub struct Port {
    index: usize,
    name: String,
    midi_port: Box<dyn Any>,
}

impl Port {
    pub fn new<T: Any + 'static>(index: usize, name: String, midi_port: T) -> Self {
        Self { index, name, midi_port: Box::new(midi_port) }
    }

    pub fn index(&self) -> usize {
        self.index
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn midi_port<T: Any>(&self) -> &T {
        &*self.midi_port.downcast_ref::<T>().unwrap()
    }
}