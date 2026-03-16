use std::error::Error;
use midir::MidiIO;

#[derive(Clone)]
pub struct Port<T: ?Sized> {
    index: usize,
    device_name: String,
    midi_port: Box<T>,
}

impl<T> Port<T> {
    pub fn index(&self) -> usize { self.index }
    pub fn device_name(&self) -> String { self.device_name.clone() }
    pub fn midi_port(&self) -> &T { &self.midi_port }
}

pub trait IoDevice {
    fn index(&self) -> usize;
    fn name(&self) -> String;
}

impl<T: ?Sized> IoDevice for Port<T> {
    fn index(&self) -> usize { self.index }
    fn name(&self) -> String { self.device_name.clone() }
}

pub struct Io<T> {
    midi_io: Box<dyn MidiIO<Port=T> + Send>,
    port: Box<Option<Port<T>>>,
    ports: Box<Vec<Port<T>>>,
}

impl<T: Clone + Send + 'static> Io<T> {
    pub fn new(midi_io: Box<dyn MidiIO<Port=T> + Send>) -> Self {
        Self { midi_io, port: Box::new(None), ports: Box::new(Vec::new()) }
    }

    pub fn ports(&self) -> Vec<Port<T>> {
        self.ports.as_ref().clone()
    }
    
    pub fn set_port(&mut self, port: Port<T>) {
        self.port = Box::new(self.ports.get(port.index()).cloned());
    }

    pub fn set_port_to_none(&mut self) {
        self.port = Box::new(None);
    }
}

/// Not "MidiIo", which is too similar to the midir trait MidiIO.
pub trait IIo: Send {
    fn device(&self) -> Option<&dyn IoDevice>;
    fn device_names(&self) -> Vec<String>;
    fn populate_devices(&mut self, persisted_device_name: &str) -> Result<(), Box<dyn Error>>;
}

impl<T: Clone + Send + 'static> IIo for Io<T> {
    fn device(&self) -> Option<&dyn IoDevice> {
        self.port
            .as_ref()               // Box<Option<...>> -> &Option<...>
            .as_ref()               // &Option<...> -> Option<&...>
            .map(|p| p as &dyn IoDevice)
    }

    fn device_names(&self) -> Vec<String> {
        self.ports
            .iter()
            .map(|port| port.device_name.clone())
            .collect()
    }

    fn populate_devices(&mut self, persisted_device_name: &str) -> Result<(), Box<dyn Error>> {
        self.ports.clear();
        self.ports.extend(
            self.midi_io.ports().iter()
                .enumerate()
                .map(|(index, port)| {
                    let name = self.midi_io.port_name(port).unwrap_or_default();
                    Port {
                        index,
                        device_name: name,
                        midi_port: Box::new(port.clone()),
                    }
                })
        );
        self.port = Box::new(
            self.ports
                .iter()
                .find(|port| port.device_name == persisted_device_name)
                .cloned()
        );
        Ok(())
    }
}
