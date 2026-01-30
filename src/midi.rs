use std::borrow::Borrow;
use std::error::Error;
use midir::{
    MidiInput, MidiInputConnection, MidiInputPort,
    MidiOutput, MidiOutputConnection, MidiOutputPort, };
use crate::settings;

pub struct MidiManager {
    connected_input_port: Option<InputPort>,
    connected_output_port: Option<OutputPort>,
    input_ports: Vec<MidiInputPort>,
    output_ports: Vec<MidiOutputPort>,
    settings: settings::Settings,
}

impl MidiManager {
    const INPUT_CLIENT_NAME: &str = "My MIDI Input";
    const OUTPUT_CLIENT_NAME: &str = "My MIDI Output";

    pub fn new() -> Self {
        Self {
            connected_input_port: None,
            connected_output_port: None,
            input_ports: Vec::new(),
            output_ports: Vec::new(),
            settings: settings::Settings::new(),
        }
    }

    pub fn close(&mut self) -> Result<(), Box<dyn Error>> {
        // println!("Midi.close");
        self.disconnect_from_input_port(true);
        self.disconnect_from_output_port(true);
        self.settings.write_to_file()?;
        Ok(())
    }

    fn on_midi_message_received(message: &[u8]) {
        println!("Received MIDI message: {:?}", message);
    }

    pub fn connect_input_port(&mut self, index: usize) -> Result<(), Box<dyn Error>> {
        self.disconnect_from_input_port(false);
        if let Some(port) = self.input_ports.get(index) {
            let midi_input = Self::create_midi_input();
            let port_name = midi_input.port_name(&port)?;
            match midi_input.connect(
                port,
                &port_name,
                |_, message, _| {
                    Self::on_midi_message_received(message)
                },
                ()) {
                Ok(connection) => {
                    self.input_connection = Option::from(connection);
                    self.settings.midi_input_port = port_name.to_string();
                }
                Err(_) =>
                    return Err(format!(
                        "Error connecting to MIDI input port {port_name}. The port may be in use.")
                        .into())
            }
        }
        Ok(())
    }

    pub fn connect_output_port(&mut self, index: usize) -> Result<(), Box<dyn Error>> {
        self.disconnect_from_output_port(false);
        if let Some(port) = self.output_ports.get(index) {
            let midi_output = Self::create_midi_output();
            let port_name = midi_output.port_name(&port)?;
            match midi_output.connect(port, &port_name) {
                Ok(connection) => {
                    self.output_connection = Option::from(connection);
                    self.settings.midi_output_port = port_name.to_string();
                    // println!("Midi.connect_output_port: settings.midi_output_port set to {}.", self.settings.midi_output_port);
                }
                Err(_) =>
                    return Err(format!(
                        "Error connecting to MIDI output port {port_name}. The port may be in use.")
                        .into())
            }
        }
        Ok(())
    }

    fn create_midi_input() -> MidiInput {
        MidiInput::new(Self::INPUT_CLIENT_NAME).unwrap()
    }

    fn create_midi_output() -> MidiOutput {
        MidiOutput::new(Self::OUTPUT_CLIENT_NAME).unwrap()
    }

    pub fn disconnect_from_input_port(&mut self, is_closing: bool) {
        if let Some(connection) = self.input_connection.take() {
            connection.close();
        }
        if !is_closing {
            self.settings.midi_input_port = "".to_string();
            // println!("Midi.disconnect_from_input_port: settings.midi_input_port set to {}.", self.settings.midi_input_port);
        }
    }

    pub fn disconnect_from_output_port(&mut self, is_closing: bool) {
        if let Some(port) = self.connected_output_port.take() {
            connection.close();
        }
        // if let Some(connection) = self.output_connection.take() {
        //     connection.close();
        // }
        if !is_closing {
            self.settings.midi_output_port = "".to_string();
            // println!("Midi.disconnect_from_output_port: settings.midi_output_port set to {}.", self.settings.midi_output_port);
        }
    }

    fn find_persisted_input_port(&self, input_port_names: &Vec<String>) -> Option<InputPort> {
        if self.settings.midi_input_port.is_empty() {
            // println!("Midi.find_persisted_input_port: settings.midi_input_port is empty.");
            return None;
        }
        input_port_names.iter().position(|name| name == &self.settings.midi_input_port)
            .map(|index| InputPort::new(index, self.settings.midi_input_port.to_string()))
    }

    fn find_persisted_output_port(&self, output_port_names: &Vec<String>) -> Option<OutputPort> {
        if self.settings.midi_output_port.is_empty() {
            // println!("Midi.find_persisted_output_port: settings.midi_output_port is empty.");
            return None;
        }
        output_port_names.iter().position(|name| name == &self.settings.midi_output_port)
            .map(|index| OutputPort::new(index, self.settings.midi_output_port.to_string()))
    }

    pub fn get_input_port_names(&self) -> Vec<String> {
        let midi_input = Self::create_midi_input();
        self.input_ports.iter()
            .map(|port|
                midi_input.port_name(&port).unwrap()).collect()
    }

    pub fn get_output_port_names(&self) -> Vec<String> {
        let midi_output = Self::create_midi_output();
        self.output_ports.iter()
            .map(|port|
                midi_output.port_name(&port).unwrap()).collect()
    }

    // pub fn is_input_port_connected(&self) -> bool {
    //     self.input_connection.is_some()
    // }
    // pub fn is_output_port_connected(&self) -> bool { self.output_connection.is_some() }

    pub fn update_input_ports(&mut self) -> Result<InputPortsData, Box<dyn Error>> {
        self.settings.read_from_file()?;
        let midi_input = Self::create_midi_input();
        self.input_ports = midi_input.ports().to_vec();
        let input_port_names: Vec<String> = self.get_input_port_names();
        // println!("Midi.update_input_ports: Found {} input ports.", input_port_names.len());
        let persisted_port =
            self.find_persisted_input_port(&input_port_names);
        // if let Some(port) = persisted_port.as_ref() {
        //     println!("Midi.update_input_ports: Found persisted input port {}.", port.name());
        // } else {
        //     println!("Midi.update_input_ports: Cannot find persisted input port.");
        // }
        Ok(InputPortsData::new(input_port_names, persisted_port))
    }

    pub fn update_output_ports(&mut self) -> Result<OutputPortsData, Box<dyn Error>> {
        self.settings.read_from_file()?;
        let midi_output = Self::create_midi_output();
        self.output_ports = midi_output.ports().to_vec();
        let output_port_names: Vec<String> = self.get_output_port_names();
        let persisted_port =
            self.find_persisted_output_port(&output_port_names);
        // if let Some(port) = persisted_port.as_ref() {
        //     println!("Midi.update_output_ports: Found persisted output port {}.", port.name());
        // } else {
        //     println!("Midi.update_output_ports: Cannot find persisted output port.");
        // }
        Ok(OutputPortsData::new(output_port_names, persisted_port))
    }
}

pub struct InputPort {
    index: usize,
    name: String,
    connection: MidiInputConnection<()>,
}

impl InputPort {
    pub fn new(index: usize, name: String, connection: MidiInputConnection<()>) -> Self {
        Self { index, name, connection }
    }

    pub fn index(&self) -> usize {
        self.index
    }

    pub fn name(&self) -> &String { &self.name }
}

pub struct InputPortsData {
    port_names: Vec<String>,
    persisted_port: Option<InputPort>,
}

impl InputPortsData {
    pub fn new(port_names: Vec<String>, connected_port: Option<InputPort>) -> Self {
        Self { port_names, persisted_port: connected_port }
    }

    pub fn persisted_port(&self) -> Option<&InputPort> {
        self.persisted_port.as_ref()
    }

    pub fn port_names(&self) -> &Vec<String> {
        &self.port_names
    }
}

pub struct OutputPort {
    index: usize,
    name: String,
    connection: MidiOutputConnection,
}

impl OutputPort {
    pub fn new(index: usize, name: String, connection: MidiOutputConnection) -> Self {
        Self { index, name, connection }
    }

    fn close_connection(&mut self) {
        if let Some(connection) = self.connection.take() {
            connection.close();
        }
    }

    // pub fn close_connection (&self) {
    //     self.connection.borrow_mut().close();
    // }
    // pub fn close_connection(&mut self) -> Result<(), Box<dyn Error>> {
    //     if let Some(conn) = self.connection.as_mut() {
    //         conn.close()?;
    //     }
    //     Ok(())
    // }
    // pub fn close_connection (&self) {
    //     if self.connection.is_some() {
    //         self.connection.close();
    //     }
    // }

    pub fn index(&self) -> usize {
        self.index
    }

    pub fn name(&self) -> &String { &self.name }
}

pub struct OutputPortsData {
    port_names: Vec<String>,
    persisted_port: Option<OutputPort>,
}

impl OutputPortsData {
    pub fn new(port_names: Vec<String>, connected_port: Option<OutputPort>) -> Self {
        Self { port_names, persisted_port: connected_port }
    }
    
    pub fn persisted_port(&self) -> Option<&OutputPort> {
        self.persisted_port.as_ref()
    }

    pub fn port_names(&self) -> &Vec<String> {
        &self.port_names
    }
}
