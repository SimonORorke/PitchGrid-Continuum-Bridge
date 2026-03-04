use std::error::Error;
use std::sync::{Arc, Mutex};
use lazy_static::lazy_static;
use midir::{
    MidiInput, MidiInputConnection, MidiInputPort,
    MidiOutput, MidiOutputConnection, MidiOutputPort};
use midly::{live::LiveEvent, MidiMessage};
use crate::midi_ports::{Io, MidiIo};
use crate::port_strategy::PortStrategy;

#[derive(Clone, Copy)]
pub enum ConnectionTo {
    Editor,
    Instru, // Instrument
}

#[derive(Clone, Copy)]
pub enum PortType {
    Input,
    Output,
}

struct MidiData {
    editor_output_connection: Option<MidiOutputConnection>,
    instru_output_connection: Option<MidiOutputConnection>,
    on_tuning_updated: Arc<Option<Box<dyn Fn() + Send + Sync + 'static>>>,
}

lazy_static! {
    static ref MIDI_DATA: Mutex<MidiData> = Mutex::new(MidiData {
        editor_output_connection: None,
        instru_output_connection: None,
        on_tuning_updated: Arc::new(None),});
}

pub struct Midi {
    editor_input: Io<MidiInputPort>,
    editor_output: Io<MidiOutputPort>,
    editor_input_connection: Option<MidiInputConnection<()>>,
    instru_input: Io<MidiInputPort>,
    instru_output: Io<MidiOutputPort>,
    instru_input_connection: Option<MidiInputConnection<()>>,
}

impl Midi {
    const INPUT_CLIENT_NAME: &str = "My MIDI Input";
    const OUTPUT_CLIENT_NAME: &str = "My MIDI Output";

    pub fn new() -> Self {
        Self {
            editor_input: Io::<MidiInputPort>::new(
                Box::new(Self::create_midi_input())),
            editor_output: Io::<MidiOutputPort>::new(
                Box::new(Self::create_midi_output())),
            editor_input_connection: None,
            instru_input: Io::<MidiInputPort>::new(
                Box::new(Self::create_midi_input())),
            instru_output: Io::<MidiOutputPort>::new(
                Box::new(Self::create_midi_output())),
            instru_input_connection: None,
        }
    }

    pub fn close(&mut self) {
        // println!("Midi.close");
        self.disconnect_input_port(&ConnectionTo::Editor);
        self.disconnect_input_port(&ConnectionTo::Instru);
        self.disconnect_output_port(&ConnectionTo::Editor);
        self.disconnect_output_port(&ConnectionTo::Instru);
    }

    pub fn connect_port(
        &mut self, index: usize, port_strategy: &dyn PortStrategy) -> Result<(), Box<dyn Error>> {
        match port_strategy.port_type() {
            PortType::Input => self.connect_input_port(index, port_strategy)?,
            PortType::Output => self.connect_output_port(index, port_strategy)?,
        }
        Ok(())
    }

    fn connect_input_port(
            &mut self, index: usize, port_strategy: &dyn PortStrategy) -> Result<(), Box<dyn Error>> {
        // println!("Midi.connect_input_port: start");
        self.disconnect_input_port(port_strategy.connection_to());
        let connection_to = *port_strategy.connection_to();
        let input: &mut Io<MidiInputPort> = match connection_to {
            ConnectionTo::Editor => &mut self.editor_input,
            ConnectionTo::Instru => &mut self.instru_input,
        };
        if let Some(port) = input.ports().get(index) {
            // println!("Midi.connect_input_port: found port");
            let port_name = port.name();
            let midi_port = port.midi_port();
            let midi_input = Self::create_midi_input();
            match midi_input.connect(
                midi_port,
                &port_name,
                move |_, message, _| {
                    match connection_to {
                        ConnectionTo::Editor => Self::on_editor_message_received(message),
                        ConnectionTo::Instru => Self::on_instru_message_received(message),
                    }
                },
                ()) {
                Ok(connection) => {
                    input.set_port(port.clone());
                    let connection_option = Option::from(connection);
                    match connection_to { 
                        ConnectionTo::Editor => self.editor_input_connection = connection_option,
                        ConnectionTo::Instru => self.instru_input_connection = connection_option,
                    }
                    // println!("Midi.connect_input_port: success");
                }
                Err(_) => {
                    // println!("Midi.connect_input_port: error");
                    // See comment in connect_output_port.
                    return Err(port_strategy.msg_cannot_connect(&port_name).into())
                }
            }
        }
        Ok(())
    }

    fn connect_output_port(&mut self, index: usize, port_strategy: &dyn PortStrategy)
            -> Result<(), Box<dyn Error>> {
        let connection_to = port_strategy.connection_to();
        self.disconnect_output_port(connection_to);
        let output: &mut Io<MidiOutputPort> = match connection_to {
            ConnectionTo::Editor => &mut self.editor_output,
            ConnectionTo::Instru => &mut self.instru_output,
        };
        if let Some(port) = output.ports().get(index) {
            let port_name = port.name();
            let midi_port = port.midi_port();
            let midi_output = Self::create_midi_output();
            match midi_output.connect(midi_port, &port_name) {
                Ok(connection) => {
                    let mut data = MIDI_DATA.lock()?;
                    match connection_to {
                        ConnectionTo::Editor => data.editor_output_connection = Option::from(connection),
                        ConnectionTo::Instru => data.instru_output_connection = Option::from(connection),
                    }
                    output.set_port(port.clone());
                }
                Err(_) =>
                    // Devices that have their own MIDI drivers may support shared connections.
                    // iConnectivity devices do.
                    // On 7th Feb 2026, I asked in the iConnectivity User Community FB group,
                    // in a post headed 'Exclusive lock on MIDI ports?',
                    // whether an iConnectivity might in future support exclusive connections,
                    // which would be useful for this application. There was no response.
                    // So on 14th Feb 2026, I raised a support ticket for the feature request.
                    // So far, no response.
                    //
                    // Also, the new Windows MIDI Services supports shared connections
                    // ("multi-client") by default.
                    // I don't know about other operating systems.
                    // As of 4th Feb 2026, I now have Windows MIDI Services on my PC.
                    // I don't see how to disable multi-client support.
                    // So I currently cannot test exclusive connections any more.
                    return Err(port_strategy.msg_cannot_connect(&port_name).into())
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

    fn disconnect_input_port(&mut self, connection_to: &ConnectionTo) {
        // println!("Midi.disconnect_input_port start");
        let input_connection = match connection_to {
            ConnectionTo::Editor => self.editor_input_connection.take(),
            ConnectionTo::Instru => self.instru_input_connection.take(),
        };
        if let Some(connection) = input_connection {
            connection.close();
            let input = match connection_to {
                ConnectionTo::Editor => &mut self.editor_input,
                ConnectionTo::Instru => &mut self.instru_input,
            };
            input.set_port_to_none();
        }
    }

    fn disconnect_output_port(&mut self, connection_to: &ConnectionTo) {
        // println!("Midi.disconnect_output_port start");
        let mut data = MIDI_DATA.lock().unwrap();
        let output_connection = match connection_to {
            ConnectionTo::Editor => data.editor_output_connection.take(),
            ConnectionTo::Instru => data.instru_output_connection.take(),
        };
        if let Some(connection) = output_connection {
            connection.close();
            let output = match connection_to {
                ConnectionTo::Editor => &mut self.editor_output,
                ConnectionTo::Instru => &mut self.instru_output,
            };
            output.set_port_to_none();
        }
    }

    pub fn init(&mut self,
                editor_input_port_name: &str, editor_output_port_name: &str,
                instru_input_port_name: &str, instru_output_port_name: &str)
                    -> Result<(), Box<dyn Error>> {
        self.editor_input.populate_ports(editor_input_port_name)?;
        self.editor_output.populate_ports(editor_output_port_name)?;
        self.instru_input.populate_ports(instru_input_port_name)?;
        self.instru_output.populate_ports(instru_output_port_name)?;
        Ok(())
    }

    pub fn io(&self, port_strategy: &dyn PortStrategy) -> &dyn MidiIo {
        port_strategy.io(self)
    }

    pub fn is_instru_input_connected(&self) -> bool {
        self.instru_input_connection.is_some()
    }

    pub fn is_instru_output_connected(&self) -> bool {
        let data = MIDI_DATA.lock().unwrap();
        data.instru_output_connection.is_some()
    }

    pub fn refresh_ports(
            &mut self, port_name: &str, port_strategy: &dyn PortStrategy)
                -> Result<(), Box<dyn Error>> {
        match port_strategy.port_type() {
            PortType::Input => self.refresh_input_ports(
                port_name, port_strategy.connection_to())?,
            PortType::Output => self.refresh_output_ports(
                port_name, port_strategy.connection_to())?,
        }
        Ok(())
    }

    /// Send a MIDI control change message.
    /// Parameter `channel` is 1-based.
    pub fn send_control_change(channel: u8, cc_no: u8, value: u8, connection_to: &ConnectionTo) {
        Self::send_channel_message(channel, MidiMessage::Controller {
            controller: cc_no.into(),
            value: value.into(),
        }, connection_to);
    }

    /// Send a MIDI program change message.
    /// Parameter `channel` is 1-based.
    /// Parameter `program` is 0-based.
    #[allow(dead_code)]
    pub fn send_program_change(channel: u8, program: u8, connection_to: &ConnectionTo) {
        Self::send_channel_message(channel, MidiMessage::ProgramChange {
            program: program.into(),
        }, connection_to);
    }

    pub fn set_on_tuning_updated(
        &mut self, callback: Box<dyn Fn() + Send + Sync + 'static>) {
        let mut data = MIDI_DATA.lock().unwrap();
        data.on_tuning_updated = Arc::new(Some(callback));
    }

    pub fn editor_input(&self) -> &Io<MidiInputPort> {
        &self.editor_input
    }

    pub fn editor_output(&self) -> &Io<MidiOutputPort> {
        &self.editor_output
    }

    pub fn instru_input(&self) -> &Io<MidiInputPort> {
        &self.instru_input
    }

    pub fn instru_output(&self) -> &Io<MidiOutputPort> {
        &self.instru_output
    }

    fn on_editor_message_received(message: &[u8]) {
        Self::send_message(message, &ConnectionTo::Instru);
    }

    fn on_instru_message_received(message: &[u8]) {
        let event = LiveEvent::parse(message).unwrap();
        match event {
            LiveEvent::Midi { channel, message } => match message {
                // MidiMessage::NoteOn { key, vel } => {
                //     let channel1 = u8::from(channel) + 1; // 1-based channel number.
                //     println!("rx: NoteOn ch{} {} {}", channel1, key, vel);
                // },
                MidiMessage::Controller { controller, .. } => {
                    let channel1 = u8::from(channel) + 1; // 1-based channel number.
                    // Call back if the pitch table has been updated and loaded.
                    if channel1 == 16 && controller == 54 {
                        // This means that the pitch table has been loaded,
                        // which will have been requested after the pitch table update
                        // was sent to the instrument.
                        // The instrument does not notify us to confirm that the pitch table
                        // has been updated. But now we effectively know that the pitch table has
                        // been updated and loaded.
                        let on_pitch_table_updated = {
                            let data = MIDI_DATA.lock().unwrap();
                            data.on_tuning_updated.clone()
                        };
                        rayon::spawn(move || {
                            if let Some(callback) =
                                    on_pitch_table_updated.as_ref() {
                                callback();
                            }
                        });
                    }
                },
                _ => {}
            },
            _ => {}
        }
        Self::send_message(message, &ConnectionTo::Editor);
    }

    fn refresh_input_ports(&mut self, input_port_name: &str,
                           connection_to: &ConnectionTo) -> Result<(), Box<dyn Error>> {
        // println!("Midi.refresh_input_ports: start");
        self.disconnect_input_port(connection_to);
        match connection_to {
            ConnectionTo::Editor => self.editor_input.populate_ports(input_port_name)?,
            ConnectionTo::Instru => self.instru_input.populate_ports(input_port_name)?,
        }
        Ok(())
    }

    fn refresh_output_ports(&mut self, output_port_name: &str,
                            connection_to: &ConnectionTo) -> Result<(), Box<dyn Error>> {
        // println!("Midi.refresh_output_ports: start");
        self.disconnect_output_port(connection_to);
        match connection_to {
            ConnectionTo::Editor => self.editor_output.populate_ports(output_port_name)?,
            ConnectionTo::Instru => self.instru_output.populate_ports(output_port_name)?,
        }
        Ok(())
    }

    /// Send a MIDI channel message.
    /// Parameter `channel` is 1-based.
    fn send_channel_message(channel: u8, message: MidiMessage, connection_to: &ConnectionTo) {
        let live_event = LiveEvent::Midi {
            channel: (channel - 1).into(), // 0-based channel number.
            message,
        };
        let mut buf = Vec::new();
        live_event.write(&mut buf).unwrap();
        Self::send_message(&buf[..], connection_to);
    }

    fn send_message(message: &[u8], connection_to: &ConnectionTo) {
        let mut data = MIDI_DATA.lock().unwrap();
        let connection_option = match connection_to {
            ConnectionTo::Editor => data.editor_output_connection.as_mut(),
            ConnectionTo::Instru => data.instru_output_connection.as_mut(),
        };
        if let Some(connection) = connection_option {
            connection.send(message)
                .unwrap_or_else(|_| println!("Error when sending message ..."));
        }
    }
}
