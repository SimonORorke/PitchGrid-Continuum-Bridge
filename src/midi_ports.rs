use std::error::Error;

#[derive(Clone)]
pub struct Port<T: ?Sized> {
    index: usize,
    name: String,
    midi_port: Box<T>,
}

impl<T> Port<T> {
    pub fn index(&self) -> usize { self.index }
    pub fn name(&self) -> String { self.name.clone() }
    pub fn midi_port(&self) -> &T { &self.midi_port }
}

pub trait IoPort {
    fn index(&self) -> usize;
    fn name(&self) -> String;
}

impl<T: ?Sized> IoPort for Port<T> {
    fn index(&self) -> usize { self.index }
    fn name(&self) -> String { self.name.clone() }
}

pub struct Io<T> {
    midi_io: Box<dyn midir::MidiIO<Port=T>>,
    port: Box<Option<Port<T>>>,
    ports: Box<Vec<Port<T>>>,
}

impl<T: Clone + 'static> Io<T> {
    pub fn new(midi_io: Box<dyn midir::MidiIO<Port=T>>) -> Self {
        Self { midi_io, port: Box::new(None), ports: Box::new(Vec::new()) }
    }

    pub fn set_port(&mut self, port: Port<T>) {
        let index = port.index();
        let selected = self
            .ports
            .iter()
            .find(|p| p.index == index)
            .cloned()
            .or_else(|| {
                None
            });
        self.port = Box::new(selected);
    }

    pub fn find_port_by_index(&self, index: usize) -> Option<&Port<T>> {
        self.ports
            .iter()
            .find(|port| port.index == index)
            .map(|p| p)
    }
}

pub trait MidiIo {
    fn port(&self) -> Option<&dyn IoPort>;
    fn port_names(&self) -> Vec<String>;
    fn populate_ports(&mut self, persisted_port_name: &str) -> Result<(), Box<dyn Error>>;
}

impl<T: Clone + 'static> MidiIo for Io<T> {
    fn port(&self) -> Option<&dyn IoPort> {
        self.port
            .as_ref()               // Box<Option<...>> -> &Option<...>
            .as_ref()               // &Option<...> -> Option<&...>
            .map(|p| p as &dyn IoPort)
    }

    fn port_names(&self) -> Vec<String> {
        self.ports
            .iter()
            .map(|port| port.name.clone())
            .collect()
    }

    fn populate_ports(&mut self, persisted_port_name: &str) -> Result<(), Box<dyn Error>> {
        self.ports.clear();
        self.ports.extend(
            self.midi_io.ports().iter()
                .enumerate()
                .map(|(index, port)| {
                    let name = self.midi_io.port_name(port).unwrap_or_default();
                    Port {
                        index,
                        name,
                        midi_port: Box::new(port.clone()),
                    }
                })
        );
        self.port = Box::new(
            self.ports
                .iter()
                .find(|port| port.name == persisted_port_name)
                .cloned()
        );
        Ok(())
    }
}
