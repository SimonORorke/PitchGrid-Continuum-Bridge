use std::error::Error;
use std::sync::Mutex;
use lazy_static::lazy_static;
use midir::{
    MidiInput, MidiInputConnection, MidiInputPort,
    MidiOutput, MidiOutputConnection, MidiOutputPort};
use crate::midi_ports::{Io, MidiIo};

#[derive(Clone, Copy)]
pub enum PortType {
    Input,
    Output,
}

struct Data {
    pub output_connection: Option<MidiOutputConnection>,
}

lazy_static! {
    static ref DATA: Mutex<Data> = Mutex::new(Data {
        output_connection: None,
    });
}

pub struct Midi {
    input: Io<MidiInputPort>,
    input_connection: Option<MidiInputConnection<()>>,
    output: Io<MidiOutputPort>,
}

impl Midi {
    const INPUT_CLIENT_NAME: &str = "My MIDI Input";
    const OUTPUT_CLIENT_NAME: &str = "My MIDI Output";

    pub fn new() -> Self {
        Self {
            input: Io::<MidiInputPort>::new(
                Box::new(Self::create_midi_input())),
            input_connection: None,
            output: Io::<MidiOutputPort>::new(
                Box::new(Self::create_midi_output())),
        }
    }

    pub fn input(&self) -> &Io<MidiInputPort> { &self.input }
    pub fn output(&self) -> &Io<MidiOutputPort> { &self.output }

    pub fn close(&mut self) {
        // println!("Midi.close");
        self.disconnect_input_port();
        self.disconnect_output_port();
    }

    pub fn connect_port(&mut self, port_type: &PortType, index: usize) -> Result<(), Box<dyn Error>> {
        match port_type {
            PortType::Input => self.connect_input_port(index)?,
            PortType::Output => self.connect_output_port(index)?,
        }
        Ok(())
    }

    fn connect_input_port(&mut self, index: usize) -> Result<(), Box<dyn Error>> {
        self.disconnect_input_port();
        if let Some(port) = self.input.ports().get(index) {
            let port_name = port.name();
            let midi_port = port.midi_port();
            let midi_input = Self::create_midi_input();
            match midi_input.connect(
                midi_port,
                &port_name,
                |_, message, _| {
                    Self::forward_midi_message(message)
                },
                ()) {
                Ok(connection) => {
                    self.input_connection = Option::from(connection);
                    self.input.set_port(port.clone());
                }
                Err(_) =>
                    return Err(format!(
                        "Cannot connect MIDI input port {}. The port may be in use.", port_name)
                        .into())
            }
        }
        Ok(())
    }

    fn connect_output_port(&mut self, index: usize) -> Result<(), Box<dyn Error>> {
        self.disconnect_output_port();
        if let Some(port) = self.output.ports().get(index) {
            let port_name = port.name();
            let midi_port = port.midi_port();
            let midi_output = Self::create_midi_output();
            match midi_output.connect(midi_port, &port_name) {
                Ok(connection) => {
                    let mut data = DATA.lock()?;
                    data.output_connection = Option::from(connection);
                    self.output.set_port(port.clone());
                }
                Err(_) =>
                    return Err(format!(
                        "Cannot connect MIDI output port {}. The port may be in use.", port.name())
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

    fn disconnect_input_port(&mut self) {
        // println!("Midi.disconnect_input_port start");
        if let Some(connection) = self.input_connection.take() {
            connection.close();
        }
    }

    fn disconnect_output_port(&mut self) {
        // println!("Midi.disconnect_output_port start");
        let mut data = DATA.lock().unwrap();
        if let Some(connection) = data.output_connection.take() {
            connection.close();
        }
    }

    fn forward_midi_message(message: &[u8]) {
        let mut data = DATA.lock().unwrap();
        if let Some(output_connection)
            = data.output_connection.as_mut() {
            output_connection.send(message)
                .unwrap_or_else(|_| println!("Error when forwarding message ..."));
        }
    }

    pub fn init(&mut self,
                input_port_name: &str, output_port_name: &str) -> Result<(), Box<dyn Error>> {
        self.input.populate_ports(input_port_name)?;
        self.output.populate_ports(output_port_name)?;
        Ok(())
    }

    pub fn io(&self, port_type: &PortType) -> &dyn MidiIo {
        match port_type {
            PortType::Input => &self.input,
            PortType::Output => &self.output,
        }
    }

    pub fn refresh_ports(&mut self,
                         port_name: &str, port_type: &PortType) -> Result<(), Box<dyn Error>> {
        match port_type {
            PortType::Input => self.refresh_input_ports(port_name)?,
            PortType::Output => self.refresh_output_ports(port_name)?,
        }
        Ok(())
    }

    fn refresh_input_ports(&mut self, input_port_name: &str) -> Result<(), Box<dyn Error>> {
        // println!("Midi.refresh_input_ports: start");
        self.disconnect_input_port();
        self.input.populate_ports(input_port_name)?;
        Ok(())
    }

    fn refresh_output_ports(&mut self, output_port_name: &str) -> Result<(), Box<dyn Error>> {
        // println!("Midi.refresh_output_ports: start");
        self.disconnect_output_port();
        self.output.populate_ports(output_port_name)?;
        Ok(())
    }
}
