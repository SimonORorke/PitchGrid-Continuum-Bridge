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

// impl IoPort for Port<dyn IoPort> {
//     fn index(&self) -> usize { self.index }
//     fn name(&self) -> String { self.name.clone() }
// }

// impl<T: IoPort + ?Sized> Port<T> {
//     pub fn new(index: usize, name: String, midi_port: Box<T>) -> Self {
//         Self { index, name, midi_port, }
//     }
//
//     // pub fn index(&self) -> usize { self.index }
//     // pub fn name(&self) -> &str { &self.name }
//     pub fn midi_port(&self) -> &T { &self.midi_port }
// }
//
// impl<T: IoPort + ?Sized> IoPort for Port<T> {
//     fn index(&self) -> usize { self.index }
//     fn name(&self) -> String { self.name.clone() }
// }
//
// impl<T: Clone> Clone for Port<T> {
//     fn clone(&self) -> Self {
//         Self { index: self.index, name: self.name.clone(),
//             midi_port: self.midi_port.clone() }
//     }
// }
//
// impl<T: IoPort + ?Sized> Display for Port<T> {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         write!(f, "[index: {}, name: {}]", self.index(), self.name())
//     }
// }

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
    // fn set_port(&mut self, port: Box<dyn IoPort>);
    // fn ports(&self) -> Vec<Port<dyn IoPort>>;
    fn port_names(&self) -> Vec<String>;
    // fn find_port_by_name(&self, name: &str) -> Option<&dyn IoPort>;
    fn populate_ports(&mut self, persisted_port_name: &str) -> Result<(), Box<dyn Error>>;
}

impl<T: Clone + 'static> MidiIo for Io<T> {
    fn port(&self) -> Option<&dyn IoPort> {
        self.port
            .as_ref()               // Box<Option<...>> -> &Option<...>
            .as_ref()               // &Option<...> -> Option<&...>
            .map(|p| p as &dyn IoPort)
    }

    // fn ports(&self) -> Vec<Port<dyn IoPort>> {
    //     self.ports
    //         .iter()
    //         .map(|port| Port {
    //             index: port.index,
    //             name: port.name.clone(),
    //             midi_port: Box::new(port.clone()) as Box<dyn IoPort>,
    //         })
    //         .collect()
    // }

    fn port_names(&self) -> Vec<String> {
        self.ports
            .iter()
            .map(|port| port.name.clone())
            .collect()
    }

    // fn find_port_by_index(&self, index: usize) -> Option<&dyn IoPort> {
    //     self.ports
    //         .iter()
    //         .find(|port| port.index == index)
    //         .map(|p| p as &dyn IoPort)
    //
    // }

    // fn find_port_by_name(&self, name: &str) -> Option<&dyn IoPort> {
    //     if name.is_empty() {
    //         return None;
    //     }
    //     self.ports
    //         .iter()
    //         .find(|port| port.name == name)
    //         .map(|p| p as &dyn IoPort)
    // }

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
