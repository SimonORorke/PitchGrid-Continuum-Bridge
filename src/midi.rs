use std::error::Error;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use lazy_static::lazy_static;
use midir::{
    MidiInput, MidiInputConnection, MidiInputPort,
    MidiOutput, MidiOutputConnection, MidiOutputPort};
use midly::{live::LiveEvent, MidiMessage};
use crate::midi_ports::{Io, MidiIo};
use crate::port_strategy::PortStrategy;

#[derive(Clone, Copy)]
pub enum PortType {
    Input,
    Output,
}

struct MidiData {
    output_connection: Option<MidiOutputConnection>,
    is_preset_loading: Arc<AtomicBool>,
    preset_loaded_callbacks: Arc<Mutex<Vec<Box<dyn Fn() + Send + Sync + 'static>>>>,
    preset_loading_callbacks: Arc<Mutex<Vec<Box<dyn Fn() + Send + Sync + 'static>>>>,
    tuning_updated_callbacks: Arc<Mutex<Vec<Box<dyn Fn() + Send + Sync + 'static>>>>,
}

lazy_static! {
    static ref MIDI_DATA: Mutex<MidiData> = Mutex::new(MidiData {
        output_connection: None,
        is_preset_loading: Arc::new(Default::default()),
        preset_loaded_callbacks: Arc::new(Mutex::new(Vec::new())),
        preset_loading_callbacks: Arc::new(Mutex::new(Vec::new())),
        tuning_updated_callbacks: Arc::new(Mutex::new(Vec::new())),
    });
}

pub struct Midi {
    input: Io<MidiInputPort>,
    output: Io<MidiOutputPort>,
    input_connection: Option<MidiInputConnection<()>>,
}

impl Midi {
    const INPUT_CLIENT_NAME: &str = "My MIDI Input";
    const OUTPUT_CLIENT_NAME: &str = "My MIDI Output";

    pub fn new() -> Self {
        Self {
            input: Io::<MidiInputPort>::new(
                Box::new(Self::create_midi_input())),
            output: Io::<MidiOutputPort>::new(
                Box::new(Self::create_midi_output())),
            input_connection: None,
        }
    }

    pub fn close(&mut self) {
        // println!("Midi.close");
        self.disconnect_input_port();
        self.disconnect_output_port();
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
        self.disconnect_input_port();
        let input: &mut Io<MidiInputPort> = &mut self.input;
        if let Some(port) = input.ports().get(index) {
            // println!("Midi.connect_input_port: found port");
            let port_name = port.name();
            let midi_port = port.midi_port();
            let midi_input = Self::create_midi_input();
            match midi_input.connect(
                midi_port,
                &port_name,
                move |_, message, _| {
                    Self::on_message_received(message)
                },
                ()) {
                Ok(connection) => {
                    input.set_port(port.clone());
                    let connection_option = Option::from(connection);
                    self.input_connection = connection_option;
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
        self.disconnect_output_port();
        let output: &mut Io<MidiOutputPort> = &mut self.output;
        if let Some(port) = output.ports().get(index) {
            let port_name = port.name();
            let midi_port = port.midi_port();
            let midi_output = Self::create_midi_output();
            match midi_output.connect(midi_port, &port_name) {
                Ok(connection) => {
                    let mut data = MIDI_DATA.lock()?;
                    data.output_connection = Option::from(connection);
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

    fn disconnect_input_port(&mut self) {
        // println!("Midi.disconnect_input_port start");
        let input_connection =
            self.input_connection.take();
        if let Some(connection) = input_connection {
            connection.close();
            let input = &mut self.input;
            input.set_port_to_none();
        }
    }

    fn disconnect_output_port(&mut self) {
        // println!("Midi.disconnect_output_port start");
        let mut data = MIDI_DATA.lock().unwrap();
        let output_connection = 
            data.output_connection.take();
        if let Some(connection) = output_connection {
            connection.close();
            let output = &mut self.output;
            output.set_port_to_none();
        }
    }

    pub fn init(&mut self,
                input_port_name: &str, output_port_name: &str)
                    -> Result<(), Box<dyn Error>> {
        self.input.populate_ports(input_port_name)?;
        self.output.populate_ports(output_port_name)?;
        Ok(())
    }

    pub fn io(&self, port_strategy: &dyn PortStrategy) -> &dyn MidiIo {
        port_strategy.io(self)
    }

    pub fn is_input_connected(&self) -> bool {
        self.input_connection.is_some()
    }

    pub fn is_output_connected(&self) -> bool {
        let data = MIDI_DATA.lock().unwrap();
        data.output_connection.is_some()
    }

    pub fn refresh_ports(
            &mut self, port_name: &str, port_strategy: &dyn PortStrategy)
                -> Result<(), Box<dyn Error>> {
        match port_strategy.port_type() {
            PortType::Input => self.refresh_input_ports(
                port_name)?,
            PortType::Output => self.refresh_output_ports(
                port_name)?,
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

    /// Send a MIDI polyphonic aftertouch (pressure) message.
    /// Parameter `channel` is 1-based.
    pub fn send_polyphonic_aftertouch(channel: u8, key: u8, pressure: u8) {
        Self::send_channel_message(channel, MidiMessage::Aftertouch {
            key: key.into(),
            vel: pressure.into(),
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

    pub fn add_preset_loaded_callback(
        &mut self, callback: Box<dyn Fn() + Send + Sync + 'static>) {
        println!("Midi.add_preset_loaded_callback");
        let callbacks =
            MIDI_DATA.lock().unwrap().preset_loaded_callbacks.clone();
        callbacks.lock().unwrap().push(callback);
    }

    pub fn add_preset_loading_callback(
        &mut self, callback: Box<dyn Fn() + Send + Sync + 'static>) {
        println!("Midi.add_preset_loading_callback");
        let callbacks =
            MIDI_DATA.lock().unwrap().preset_loading_callbacks.clone();
        callbacks.lock().unwrap().push(callback);
    }

    pub fn add_tuning_updated_callback(
            &mut self, callback: Box<dyn Fn() + Send + Sync + 'static>) {
        // println!("Midi.add_tuning_updated_callback");
        let callbacks =
            MIDI_DATA.lock().unwrap().tuning_updated_callbacks.clone();
        callbacks.lock().unwrap().push(callback);
    }

    pub fn input(&self) -> &Io<MidiInputPort> {
        &self.input
    }

    pub fn output(&self) -> &Io<MidiOutputPort> {
        &self.output
    }

    fn on_message_received(message: &[u8]) {
        let event = LiveEvent::parse(message).unwrap();
        match event {
            LiveEvent::Midi { channel, message } => match message {
                MidiMessage::Controller { controller, value } => {
                    let channel1 = u8::from(channel) + 1; // 1-based channel number.
                    if channel1 == 16
                        // Ignore heartbeats
                        && controller != 82 && controller != 111 && controller != 114
                        && controller != 118 {
                        println!("Midi.on_message_received: ch{} cc{} value {}",
                                 channel1, controller, value);
                    }
                    // Call back if the pitch table has been updated and loaded.
                    if channel1 == 16 && controller == 51 {
                        println!("midi.on_message_received: pitch table updated");
                        // This means that the pitch table has been loaded,
                        // which will have been requested after the pitch table update
                        // was sent to the instrument.
                        // The instrument does not notify us to confirm that the pitch table
                        // has been updated. But now we effectively know that the pitch table has
                        // been updated and loaded.
                        let data = MIDI_DATA.lock().unwrap();
                        Self::call_callbacks(data.tuning_updated_callbacks.clone());
                    }
                    // Call back if a preset load has been requested.
                    if channel1 == 16 && controller == 109 && value == 16 {
                        println!("Midi.on_message_received: preset load requested");
                        let data = MIDI_DATA.lock().unwrap();
                        data.is_preset_loading.store(true, Ordering::Relaxed);
                        Self::call_callbacks(data.preset_loading_callbacks.clone());
                    }
                },
                MidiMessage::ProgramChange { .. } => {
                    let channel1 = u8::from(channel) + 1; // 1-based channel number.
                    if channel1 == 16 {
                        let data = MIDI_DATA.lock().unwrap();
                        let is_preset_loading =
                            data.is_preset_loading.load(Ordering::Relaxed);
                        if is_preset_loading {
                            // This is the last item in the preset data sent when a preset
                            // has been loaded.
                            println!("Midi.on_message_received: preset loaded");
                            data.is_preset_loading.store(false, Ordering::Relaxed);
                            Self::call_callbacks(data.preset_loaded_callbacks.clone());
                        }
                    }
                },
                _ => {}
            },
            _ => {}
        }
        // Self::send_message(message);
    }

    /// Call the subscribed callback functions on a separate thread.
    fn call_callbacks(callbacks: Arc<Mutex<Vec<Box<dyn Fn() + Send + Sync + 'static>>>>) {
        rayon::spawn(move || {
            let callbacks_guard = callbacks.lock().unwrap();
            for callback in callbacks_guard.iter() {
                callback();
            }
        });
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
        let connection_option = 
            data.output_connection.as_mut();
        if let Some(connection) = connection_option {
            connection.send(message)
                .unwrap_or_else(|_| println!("Error when sending message ..."));
        }
    }
}
