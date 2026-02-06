use std::fmt::Display;
use midir::{MidiInputPort, MidiOutputPort};

trait Port {
    fn index(&self) -> usize;
    fn name(&self) -> &String;
}

impl Display for dyn Port {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[index: {}, name: {}]", self.index(), self.name())
    }
}

pub struct InputPort {
    index: usize,
    name: String,
    midi_input_port: MidiInputPort,
}

impl InputPort {
    pub fn new(index: usize, name: String, midi_input_port: MidiInputPort) -> Self {
        Self { index, name, midi_input_port }
    }
}

impl Port for InputPort {
    fn index(&self) -> usize {
        self.index
    }
    fn name(&self) -> &String {
        &self.name
    }
}

impl Clone for InputPort {
    fn clone(&self) -> Self {
        Self { index: self.index, name: self.name.clone(),
            midi_input_port: self.midi_input_port.clone() }
    }
}

pub struct OutputPort {
    index: usize,
    name: String,
    midi_output_port: MidiOutputPort,
}

impl OutputPort {
    pub fn new(index: usize, name: String, midi_output_port: MidiOutputPort) -> Self {
        Self { index, name, midi_output_port}
    }
}

impl Port for OutputPort {
    fn index(&self) -> usize { self.index }
    fn name(&self) -> &String { &self.name }
}

impl Clone for OutputPort {
    fn clone(&self) -> Self {
        Self { index: self.index, name: self.name.clone(),
            midi_output_port: self.midi_output_port.clone() }
    }
}

trait Io {
    fn connected_port(&self) -> Option<&impl Port>;
    fn set_connected_port(&mut self, port: impl Port);
    fn ports(&self) -> Vec<impl Port>;
}

struct Input {
    connected_port: Option<InputPort>,
    ports: Vec<InputPort>,
}

// impl Io for Input {
//     fn connected_port(&self) -> Option<&impl Port> { self.connected_port.as_ref() }
//     fn set_connected_port(&mut self, port: impl Port) { self.connected_port = Some(port) }
//     fn ports(&self) -> Vec<impl Port> { self.ports.clone() }
// }

struct Output {
    connected_port: Option<OutputPort>,
    ports: Vec<OutputPort>,
}