use std::error::Error;
use std::fmt::Display;
// use midir::MidiIO;

pub struct Port<T: ?Sized> {
    index: usize,
    name: String,
    midi_port: Box<T>,
}

pub trait IoPort {
    fn index(&self) -> usize;
    fn name(&self) -> String;
}

impl<T: ?Sized> Port<T> {
    pub fn new(index: usize, name: String, midi_port: Box<T>) -> Self {
        Self { index, name, midi_port, }
    }

    // pub fn index(&self) -> usize { self.index }
    // pub fn name(&self) -> &str { &self.name }
    pub fn midi_port(&self) -> &T { &self.midi_port }
}

impl<T: IoPort> IoPort for Port<T> {
    fn index(&self) -> usize { self.index }
    fn name(&self) -> String { self.name.clone() }
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

pub struct Io<T> {
    midi_io: Box<dyn MidiIO<Port=T>>,
    port: Option<Port<T>>,
    ports: Vec<Port<T>>,
}

impl<T: Clone> Io<T> {
    pub fn new(midi_io: Box<dyn MidiIO<Port=T>>) -> Self {
        Self { midi_io, port: None, ports: Vec::new() }
    }
    
    pub fn port(&self) -> Option<&Port<T>> { self.port.as_ref() }
    pub fn set_port(&mut self, port: Port<T>) { self.port = Some(port) }

    pub fn find_port_by_index(&self, index: usize) -> Option<Port<T>> {
        self.ports.iter().position(|port| port.index() == index)
            .map(|index| self.ports[index].clone())
    }

    fn find_port_by_name(&self, name: &str) -> Option<Port<T>> {
        if name.is_empty() {
            return None;
        }
        self.ports.iter().position(|port| port.name == name)
            .map(|index| self.ports[index].clone())
    }

    pub fn port_names(&self) -> Vec<String> {
        self.midi_io.ports().iter()
            .map(|port|
                self.midi_io.port_name(&port).unwrap()).collect()
    }

    pub fn populate_ports(&mut self, persisted_port_name: &str) -> Result<(), Box<dyn Error>> {
        self.ports.clear();
        self.ports.extend(
            self.midi_io.ports().iter()
                .enumerate()
                .map(|(index, port)| {
                    let name = self.midi_io.port_name(port).unwrap_or_default();
                    Port::new(index, name, port.clone())
                })
        );
        self.port = self.find_port_by_name(persisted_port_name);
        Ok(())
    }
}
