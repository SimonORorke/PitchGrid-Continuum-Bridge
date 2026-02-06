use std::fmt::Display;

pub struct Port<T> {
    index: usize,
    name: String,
    midi_port: T,
}

impl<T> Port<T> {
    pub fn new(index: usize, name: String, midi_port: T) -> Self {
        Self {
            index,
            name,
            midi_port,
        }
    }

    pub fn index(&self) -> usize {
        self.index
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn midi_port(&self) -> &T {
        &self.midi_port
    }

    pub fn midi_port_mut(&mut self) -> &mut T {
        &mut self.midi_port
    }

    pub fn into_midi_port(self) -> T {
        self.midi_port
    }
}

impl<T: Clone> Clone for Port<T> {
    fn clone(&self) -> Self {
        Self { index: self.index, name: self.name.clone(),
            midi_port: self.midi_port.clone() }
    }
}

impl<T> Display for Port<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[index: {}, name: {}]", self.index(), self.name())
    }
}

struct Io<T> {
    connected_port: Option<Port<T>>,
    ports: Vec<Port<T>>,
}

impl<T> Io<T> {
    fn new(ports: Vec<Port<T>>) -> Self {
        Self { connected_port: None, ports }
    }
    fn connected_port(&self) -> Option<&Port<T>> { self.connected_port.as_ref() }
    fn set_connected_port(&mut self, port: Port<T>) { self.connected_port = Some(port) }
    fn ports(&self) -> &Vec<Port<T>> { self.ports.as_ref() }
}