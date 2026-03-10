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

#[derive(Clone, Copy, Debug)]
pub enum PresetLoading {
    Replace = 0,
    Preserve = 1,
}

struct MidiData {
    config_received_callbacks: Arc<Mutex<Vec<Box<dyn Fn() + Send + Sync + 'static>>>>,
    // has_config_been_received: Arc<AtomicBool>,
    /// Initial Preset Loading Surface Processing global setting
    initial_surface_processing: Arc<Mutex<Option<PresetLoading>>>,
    instru_connected_changed_callbacks: Arc<Mutex<Vec<Box<dyn Fn() + Send + Sync + 'static>>>>,
    is_getting_config: Arc<AtomicBool>,
    is_streaming_initial_matrix: Arc<AtomicBool>,
    is_instru_connected: Arc<AtomicBool>,
    output_connection: Option<MidiOutputConnection>,
    tuning_updated_callbacks: Arc<Mutex<Vec<Box<dyn Fn() + Send + Sync + 'static>>>>,
}

lazy_static! {
    static ref MIDI_DATA: Mutex<MidiData> = Mutex::new(MidiData {
        config_received_callbacks: Arc::new(Mutex::new(Vec::new())),
        // has_config_been_received: Arc::new(Default::default()),
        initial_surface_processing: Arc::new(Mutex::new(None)),
        instru_connected_changed_callbacks: Arc::new(Mutex::new(vec![])),
        is_getting_config: Arc::new(Default::default()),
        is_streaming_initial_matrix: Arc::new(Default::default()),
        is_instru_connected: Arc::new(Default::default()),
        output_connection: None,
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

    pub fn add_config_received_callback(
        &mut self, callback: Box<dyn Fn() + Send + Sync + 'static>) {
        // println!("Midi.add_tuning_updated_callback");
        let callbacks =
            MIDI_DATA.lock().unwrap().config_received_callbacks.clone();
        callbacks.lock().unwrap().push(callback);
    }

    pub fn add_instru_connected_changed_callback(
        &mut self, callback: Box<dyn Fn() + Send + Sync + 'static>) {
        // println!("Midi.add_tuning_updated_callback");
        let callbacks =
            MIDI_DATA.lock().unwrap().instru_connected_changed_callbacks.clone();
        callbacks.lock().unwrap().push(callback);
    }

    pub fn add_tuning_updated_callback(
        &mut self, callback: Box<dyn Fn() + Send + Sync + 'static>) {
        // println!("Midi.add_tuning_updated_callback");
        let callbacks =
            MIDI_DATA.lock().unwrap().tuning_updated_callbacks.clone();
        callbacks.lock().unwrap().push(callback);
    }

    /// Return whether both input and output ports are connected.
    pub fn are_ports_connected(&self) -> bool {
        if self.input_connection.is_none() {
            return false;
        }
        let data = MIDI_DATA.lock().unwrap();
        data.output_connection.is_some()
    }

    pub fn close(&mut self) {
        println!("Midi.close");
        {
            let data = MIDI_DATA.lock().unwrap();
            println!("Midi.close: Got data");
            if data.output_connection.is_some() {
                println!("Midi.close: Cloning initial_surface_processing");
                let initial_surface_processing =
                    Arc::clone(&data.initial_surface_processing);
                println!("Midi.close: Getting initial_surface_processing guard");
                let initial_surface_processing_guard =
                    initial_surface_processing.lock().unwrap();
                println!("Midi.close: Getting initial_surface_processing");
                if let Some(initial_surface_processing) =
                    *initial_surface_processing_guard {
                    println!("Midi.close: send_surface_processing data");
                    Self::send_surface_processing(initial_surface_processing);
                }
            }
        }
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

    pub fn init(&mut self,
                input_port_name: &str, output_port_name: &str)
                -> Result<(), Box<dyn Error>> {
        self.input.populate_ports(input_port_name)?;
        self.output.populate_ports(output_port_name)?;
        Ok(())
    }

    pub fn input(&self) -> &Io<MidiInputPort> {
        &self.input
    }

    pub fn io(&self, port_strategy: &dyn PortStrategy) -> &dyn MidiIo {
        port_strategy.io(self)
    }

    pub fn output(&self) -> &Io<MidiOutputPort> {
        &self.output
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

    /// Request instrument configuration data.
    /// We currently only need the Preset Loading Surface Processing global setting.
    /// But the only way to get it is to request all the current preset and config data.
    pub fn request_config(&self) {
        println!("Midi.request_config");
        {
            let data = MIDI_DATA.lock().unwrap();
            println!("Midi.request_config: Got data");
            *data.initial_surface_processing.lock().unwrap() = None;
            data.is_getting_config.store(true, Ordering::Relaxed);
            data.is_streaming_initial_matrix.store(false, Ordering::Relaxed);
        }
        Self::send_control_change(16, 109, 16);  // configToMidi
    }

    /// Send a MIDI control change message.
    /// Parameter `channel` is 1-based.
    pub fn send_control_change(channel: u8, cc_no: u8, value: u8) {
        Self::send_channel_message(channel, MidiMessage::Controller {
            controller: cc_no.into(),
            value: value.into(),
        });
    }

    pub fn send_matrix_poke(poke_id: u8, poke_value: u8) {
        Self::send_control_change(
            16, 56, 20); // Matrix Poke command
        Self::send_polyphonic_aftertouch(
            16, poke_id, poke_value); // Perform the Poke
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

    /// Send Preset Loading Surface Processing global setting.
    pub fn send_surface_processing(on_preset_loading: PresetLoading) {
        let poke_id = on_preset_loading as u8;
        Self::send_matrix_poke(56, poke_id); // PreservSurf
        // The editor then does this.  But it seems not to make a difference here.
        // Write current global settings to flash
        // send_control_change(16, 109, 8); // Task curGloToFlash
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

    fn on_message_received(message: &[u8]) {
        let event = LiveEvent::parse(message).unwrap();
        match event {
            LiveEvent::Midi { channel, message } => match message {
                MidiMessage::Controller { controller, value } => {
                    let channel1 = u8::from(channel) + 1; // 1-based channel number.
                    if channel1 == 16 {
                        // if controller != 82 && controller != 111 && controller != 114
                        //     && controller != 118 {  // Heartbeats ignored
                        //     println!("Midi.on_message_received: ch{} cc{} value {}",
                        //              channel1, controller, value);
                        // }
                        if controller == 51 {
                            // The pitch table has been updated and loaded.
                            // This message means that the pitch table has been loaded,
                            // which will have been requested after the pitch table update
                            // was sent to the instrument.
                            // The instrument does not notify us to confirm that the pitch table
                            // has been updated. But now we effectively know that the pitch table has
                            // been updated and loaded.
                            // println!("midi.on_message_received: pitch table updated");
                            let data = MIDI_DATA.lock().unwrap();
                            Self::call_callbacks(data.tuning_updated_callbacks.clone());
                        }
                        if controller == 56 && value == 20 {
                            // s_Mat_Poke: Start of Matrix stream
                            println!("midi.on_message_received: Start of Matrix stream");
                            let data = MIDI_DATA.lock().unwrap();
                            println!("midi.on_message_received: Got data");
                            let initial_surface_processing =
                                data.initial_surface_processing.lock().unwrap();
                            if initial_surface_processing.is_none() {
                                println!("Midi.on_message_received: Start of initial matrix stream");
                                data.is_streaming_initial_matrix.store(true, Ordering::Relaxed);
                            }
                        }
                    }
                },
                MidiMessage::Aftertouch { key, vel } => {
                    let channel1 = u8::from(channel) + 1; // 1-based channel number.
                    if channel1 == 16 && key == 56 {
                        // PreservSurf: Preset Loading Surface Processing (global)
                        let data = MIDI_DATA.lock().unwrap();
                        let is_streaming_initial_matrix =
                            data.is_streaming_initial_matrix.load(Ordering::Relaxed);
                        if is_streaming_initial_matrix {
                            let mut initial_surface_processing =
                                data.initial_surface_processing.lock().unwrap();
                            let preset_loading: PresetLoading = match u8::from(vel) {
                                0 => PresetLoading::Replace,
                                _ => PresetLoading::Preserve,
                            };
                            *initial_surface_processing = Option::from(preset_loading);
                            // We are not waiting for anything else from the matrix stream,
                            // and it does not have an end-of-stream message.
                            data.is_streaming_initial_matrix.store(false, Ordering::Relaxed);
                            println!("Midi.on_message_received: initial_surface_processing = {:?}",
                                     preset_loading);
                        }
                    }
                },
                MidiMessage::ProgramChange { .. } => {
                    let channel1 = u8::from(channel) + 1; // 1-based channel number.
                    if channel1 == 16 {
                        let data = MIDI_DATA.lock().unwrap();
                        let is_getting_config =
                            data.is_getting_config.load(Ordering::Relaxed);
                        if is_getting_config {
                            // This is the last item sent when config has been requested.
                            println!("Midi.on_message_received: config received");
                            data.is_getting_config.store(false, Ordering::Relaxed);
                            // data.has_config_been_received.store(true, Ordering::Relaxed);
                            Self::call_callbacks(data.config_received_callbacks.clone());
                        }
                    }
                },
                _ => {}
            },
            _ => {}
        }
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
        println!("Midi.send_message");
        let mut data = MIDI_DATA.lock().unwrap();
        println!("Midi.send_message: Got data");
        let connection_option =
            data.output_connection.as_mut();
        if let Some(connection) = connection_option {
            connection.send(message)
                .unwrap_or_else(|_| println!("Error when sending message ..."));
        }
    }
}
