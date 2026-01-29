use std::error::Error;
use midir::{MidiOutput, MidiOutputConnection, MidiOutputPort};
use crate::settings;

pub struct MidiManager {
    output_connection: Option<MidiOutputConnection>,
    output_ports: Vec<MidiOutputPort>,
    settings: settings::Settings,
}

impl MidiManager {
    const OUTPUT_CLIENT_NAME: &str = "My MIDI Output";

    pub fn new() -> Self {
        Self {
            output_connection: None,
            output_ports: Vec::new(),
            settings: settings::Settings::new(),
        }
    }

    pub fn close(&mut self) -> Result<(), Box<dyn Error>> {
        self.disconnect_from_output_port(true);
        self.settings.write_to_file()?;
        Ok(())
    }

    pub fn connect_output_port(&mut self, index: usize) -> Result<(), Box<dyn Error>> {
        self.disconnect_from_output_port(false);
        if let Some(port) = self.output_ports.get(index) {
            let midi_output = Self::get_midi_output();
            let port_name = midi_output.port_name(&port)?;
            match midi_output.connect(port, &port_name) {
                Ok(connection) => {
                    self.output_connection = Option::from(connection);
                    self.settings.midi_output_port = port_name.to_string();
                }
                Err(_) =>
                    return Err(format!(
                        "Error connecting to MIDI output port {port_name}. The port may be in use.")
                        .into())
            }
        }
        Ok(())
    }

    fn disconnect_from_output_port(&mut self, is_closing: bool) {
        if let Some(connection) = self.output_connection.take() {
            connection.close();
        }
        if !is_closing {
            self.settings.midi_output_port = "".to_string();
        }
    }

    fn find_persisted_output_port(&self, output_port_names: &Vec<String>) -> Option<OutputPort> {
        if self.settings.midi_output_port.is_empty() {
            return None;
        }
        output_port_names.iter().position(|name| name == &self.settings.midi_output_port)
            .map(|index| OutputPort::new(index, self.settings.midi_output_port.to_string()))
    }

    fn get_midi_output() -> MidiOutput {
        MidiOutput::new(Self::OUTPUT_CLIENT_NAME).unwrap()
    }

    pub fn get_output_port_names(&self) -> Vec<String> {
        let midi_output = Self::get_midi_output();
        self.output_ports.iter()
            .map(|port|
                midi_output.port_name(&port).unwrap()).collect()
    }

    pub fn update_output_ports(&mut self) -> Result<OutputPortsData, Box<dyn Error>> {
        self.settings.read_from_file()?;
        let midi_output = Self::get_midi_output();
        self.disconnect_from_output_port(false);
        self.output_ports = midi_output.ports().to_vec();
        let output_port_names: Vec<String> = self.get_output_port_names();
        let persisted_port =
            self.find_persisted_output_port(&output_port_names);
        Ok(OutputPortsData::new(output_port_names, persisted_port))
    }
}

pub struct OutputPort {
    index: usize,
    name: String,
}

impl OutputPort {
    pub fn new(index: usize, name: String) -> Self {
        Self { index, name }
    }

    pub fn get_index(&self) -> usize {
        self.index
    }

    // pub fn get_name(&self) -> &String {
    //     &self.name
    // }
}

impl Clone for OutputPort {
    fn clone(&self) -> Self {
        Self { index: self.index, name: self.name.clone() }
    }
}

pub struct OutputPortsData {
    port_names: Vec<String>,
    persisted_port: Option<OutputPort>,
}

impl OutputPortsData {
    pub fn new(port_names: Vec<String>, connected_port: Option<OutputPort>) -> Self {
        Self { port_names, persisted_port: connected_port }
    }
    
    pub fn get_persisted_port(&self) -> Option<&OutputPort> {
        self.persisted_port.as_ref()
    }

    pub fn get_port_names(&self) -> &Vec<String> {
        &self.port_names
    }
}
