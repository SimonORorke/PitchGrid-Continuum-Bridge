use std::error::Error;
use std::sync::Mutex;
use lazy_static::lazy_static;
use midir::{
    MidiInput, MidiInputConnection, MidiInputPort,
    MidiOutput, MidiOutputConnection, MidiOutputPort};
use midly::{live::LiveEvent, MidiMessage};
use crate::midi_ports::{Io, MidiIo};

#[derive(Clone, Copy)]
pub enum PortType {
    Input,
    Output,
}

struct MidiData {
    pub output_connection: Option<MidiOutputConnection>,
}

lazy_static! {
    static ref MIDI_DATA: Mutex<MidiData> = Mutex::new(MidiData {
        output_connection: None,
    });
}

pub struct Midi {
    input: Io<MidiInputPort>,
    input_connection: Option<MidiInputConnection<()>>,
    output: Io<MidiOutputPort>,
}

// impl Midi {
//     pub(crate) fn send_pitch_table_to_instrument(p0: i32) {
//         todo!()
//     }
// }

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
                    Self::on_message_received(message)
                },
                ()) {
                Ok(connection) => {
                    self.input_connection = Option::from(connection);
                    self.input.set_port(port.clone());
                }
                Err(_) =>
                    // See comment in connect_output_port.
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
                    let mut data = MIDI_DATA.lock()?;
                    data.output_connection = Option::from(connection);
                    self.output.set_port(port.clone());
                }
                Err(_) =>
                    // Devices that have their own MIDI drivers may support shared connections.
                    // iConnectivity devices do.
                    // Also, the new Windows MIDI Services does by default.
                    // I don't know about other operating systems.
                    // On 7th Feb 2026, I asked in the iConnectivity User Community FB group,
                    // in a post headed 'Exclusive lock on MIDI ports?',
                    // whether an iConnectivity might in future support exclusive connections,
                    // which would be useful for this application. There was no response.
                    // So on 14th Feb 2026, I raised a support ticket for the feature request.
                    // So far, no response.
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
            self.input.set_port_to_none();
        }
    }

    fn disconnect_output_port(&mut self) {
        // println!("Midi.disconnect_output_port start");
        let mut data = MIDI_DATA.lock().unwrap();
        if let Some(connection) = data.output_connection.take() {
            connection.close();
            self.output.set_port_to_none();
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

    /// Send a MIDI control change message.
    /// Parameter `channel` is 1-based.
    pub fn send_control_change(channel: u8, cc_no: u8, value: u8) {
        Self::send_channel_message(channel, MidiMessage::Controller {
            controller: cc_no.into(),
            value: value.into(),
        });
    }

    /// Send a MIDI program change message.
    /// Parameter `channel` is 1-based.
    /// Parameter `program` is 0-based.
    #[allow(dead_code)]
    pub fn send_program_change(channel: u8, program: u8) {
        Self::send_channel_message(channel, MidiMessage::ProgramChange {
            program: program.into(),
        });
    }

    fn on_message_received(message: &[u8]) {
        let event = LiveEvent::parse(message).unwrap();
        match event {
            LiveEvent::Midi { channel, message } => match message {
                MidiMessage::NoteOn { key, vel } => {
                    let channel1 = u8::from(channel) + 1; // 1-based channel number.
                    println!("rx: NoteOn ch{} {} {}", channel1, key, vel);
                },
                MidiMessage::Controller { controller, value } => {
                    let channel1 = u8::from(channel) + 1; // 1-based channel number.
                    if channel1 == 16 && controller != 82 {
                        println!("rx: ch{} cc{} {}", channel1, controller, value);
                        if controller == 109 {
                            if value == 54 {
                                println!("Start of user preset list");
                            } else if value == 55 {
                                println!("End of user preset list");
                            }
                        }
                    }
                },
                _ => {}
            },
            _ => {}
        }
        // Needed if we need to forward from Haken Editor
        //Self::send_message(message);
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

    /// Send a MIDI channel message.
    /// Parameter `channel` is 1-based.
    fn send_channel_message(channel: u8, message: MidiMessage) {
        let live_event = LiveEvent::Midi {
            channel: (channel - 1).into(), // 0-based channel number.
            message,
        };
        let mut buf = Vec::new();
        live_event.write(&mut buf).unwrap();
        Self::send_message(&buf[..]);
    }

    fn send_message(message: &[u8]) {
        let mut data = MIDI_DATA.lock().unwrap();
        if let Some(output_connection)
            = data.output_connection.as_mut() {
            output_connection.send(message)
                .unwrap_or_else(|_| println!("Error when sending message ..."));
        }
    }
}
