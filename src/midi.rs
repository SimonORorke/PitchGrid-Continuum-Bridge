use std::cmp::PartialEq;
use lazy_static::lazy_static;
use midir::{
    MidiInput, MidiInputConnection, MidiInputPort, MidiOutput, MidiOutputConnection, MidiOutputPort,
};
use midly::{MidiMessage, live::LiveEvent};
use std::error::Error;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::time::{Duration, Instant};
use crate::global::{PortType, PresetLoading};
use crate::midi_ports::{Io, IIo};
use crate::port_strategy::PortStrategy;

pub struct Midi {
    connection_monitor_stopper_senders: Vec<mpsc::Sender<()>>,
    input: Io<MidiInputPort>,
    input_connection: Option<MidiInputConnection<()>>,
    is_connection_monitor_running: bool,
    output: Io<MidiOutputPort>,
}

impl Midi {
    pub fn new() -> Self {
        Self {
            connection_monitor_stopper_senders: vec![],
            input: Io::<MidiInputPort>::new(Box::new(Self::create_midi_input())),
            input_connection: None,
            is_connection_monitor_running: false,
            output: Io::<MidiOutputPort>::new(Box::new(Self::create_midi_output())),
        }
    }

    pub fn add_config_received_callback(
        &mut self,
        callback: Box<dyn Fn() + Send + Sync + 'static>,
    ) {
        // println!("Midi.add_config_received_callback");
        CONFIG_RECEIVED_CALLBACKS.lock().unwrap().push(callback);
    }

    pub fn add_editor_data_download_completed_callback(
        &mut self,
        callback: Box<dyn Fn() + Send + Sync + 'static>,
    ) {
        // println!("Midi.add_editor_data_download_completed_callback");
        EDITOR_DATA_DOWNLOAD_COMPLETED_CALLBACKS.lock().unwrap().push(callback);
    }

    pub fn add_instru_connected_changed_callback(
        &mut self,
        callback: Box<dyn Fn() + Send + Sync + 'static>,
    ) {
        // println!("Midi.add_tuning_updated_callback");
        INSTRU_CONNECTED_CHANGED_CALLBACKS.lock().unwrap().push(callback);
    }

    pub fn add_tuning_updated_callback(&mut self, callback: Box<dyn Fn() + Send + Sync + 'static>) {
        // println!("Midi.add_tuning_updated_callback");
        TUNING_UPDATED_CALLBACKS.lock().unwrap().push(callback);
    }

    /// Return whether both input and output ports are connected.
    pub fn are_ports_connected(&self) -> bool {
        if self.input_connection.is_none() {
            return false;
        }
        OUTPUT_CONNECTION.lock().unwrap().is_some()
    }

    pub fn close(&mut self) {
        // println!("Midi.close");
        {
            if OUTPUT_CONNECTION.lock().unwrap().is_some() {
                // println!("Midi.close: Cloning initial_surface_processing");
                let initial_surface_processing =
                    Arc::clone(&INITIAL_SURFACE_PROCESSING);
                // println!("Midi.close: Getting initial_surface_processing guard");
                let initial_surface_processing_guard =
                    initial_surface_processing.lock().unwrap();
                // println!("Midi.close: Getting initial_surface_processing");
                if let Some(initial_surface_processing) =
                        *initial_surface_processing_guard {
                    // println!("Midi.close: Sending surface processing {:?}",
                    //          initial_surface_processing);
                    Self::send_surface_processing(initial_surface_processing);
                }
            }
        }
        self.disconnect_input_port();
        self.disconnect_output_port();
    }

    pub fn connect_port(
        &mut self,
        index: usize,
        port_strategy: &dyn PortStrategy,
    ) -> Result<(), Box<dyn Error>> {
        match port_strategy.port_type() {
            PortType::Input => self.connect_input_port(index, port_strategy)?,
            PortType::Output => self.connect_output_port(index, port_strategy)?,
        }
        Ok(())
    }

    pub fn init(
        &mut self,
        input_device_name: &str,
        output_device_name: &str,
    ) -> Result<(), Box<dyn Error>> {
        self.input.populate_devices(input_device_name)?;
        self.output.populate_devices(output_device_name)?;
        Ok(())
    }

    pub fn input(&self) -> &Io<MidiInputPort> {
        &self.input
    }

    pub fn io(&self, port_strategy: &dyn PortStrategy) -> &dyn IIo {
        port_strategy.io(self)
    }

    pub fn is_instru_connected(&self) -> bool {
        IS_INSTRU_CONNECTED.load(Ordering::Relaxed)
    }

    pub fn output(&self) -> &Io<MidiOutputPort> {
        &self.output
    }

    pub fn refresh_devices(
        &mut self,
        device_name: &str,
        port_strategy: &dyn PortStrategy,
    ) -> Result<(), Box<dyn Error>> {
        match port_strategy.port_type() {
            PortType::Input => self.refresh_input_devices(device_name)?,
            PortType::Output => self.refresh_output_devices(device_name)?,
        }
        Ok(())
    }

    /// Request instrument configuration data.
    /// We currently only need the Preset Loading Surface Processing global setting.
    /// But the only way to get it is to request all the current preset and config data.
    pub fn request_config(&self) {
        println!("Midi.request_config");
        {
            *INITIAL_SURFACE_PROCESSING.lock().unwrap() = None;
            IS_GETTING_CONFIG.store(true, Ordering::Relaxed);
            IS_INITIAL_MATRIX_STREAMING.store(false, Ordering::Relaxed);
        }
        Self::send_control_change(16, 109, 16); // configToMidi
    }

    /// Send a MIDI control change message.
    /// Parameter `channel` is 1-based.
    pub fn send_control_change(channel: u8, cc_no: u8, value: u8) {
        Self::send_channel_message(
            channel,
            MidiMessage::Controller {
                controller: cc_no.into(),
                value: value.into(),
            },
        );
    }

    pub fn send_matrix_poke(poke_id: u8, poke_value: u8) {
        Self::send_control_change(16, 56, 20); // Matrix Poke command
        Self::send_polyphonic_aftertouch(16, poke_id, poke_value); // Perform the Poke
    }

    /// Send a MIDI polyphonic aftertouch (pressure) message.
    /// Parameter `channel` is 1-based.
    pub fn send_polyphonic_aftertouch(channel: u8, key: u8, pressure: u8) {
        Self::send_channel_message(
            channel,
            MidiMessage::Aftertouch {
                key: key.into(),
                vel: pressure.into(),
            },
        );
    }

    /// Send a MIDI program change message.
    /// Parameter `channel` is 1-based.
    /// Parameter `program` is 0-based.
    #[allow(dead_code)]
    pub fn send_program_change(channel: u8, program: u8) {
        Self::send_channel_message(
            channel,
            MidiMessage::ProgramChange {
                program: program.into(),
            },
        );
    }

    /// Send Preset Loading Surface Processing global setting.
    /// Haken Editor's display of the setting may not be updated.
    pub fn send_surface_processing(on_preset_loading: PresetLoading) {
        let poke_id = on_preset_loading as u8;
        Self::send_matrix_poke(56, poke_id); // PreservSurf
        // The editor then does this. It seems not to make a difference here.
        // But let's do it anyway.
        // Write current global settings to flash
        Self::send_control_change(16, 109, 8); // curGloToFlash
    }

    pub fn start_instru_connection_monitor(&mut self) {
        // println!("Midi.start_instru_connection_monitor");
        let (stopper_sender, stopper_receiver) = mpsc::channel();
        self.connection_monitor_stopper_senders.push(stopper_sender);
        rayon::spawn(move || {
            Self::monitor_instru_connection(stopper_receiver);
        });
        self.is_connection_monitor_running = true;
    }

    pub fn stop_instru_connection_monitor(&mut self) {
        // println!("Midi.stop_instru_connection_monitor");
        if self.is_connection_monitor_running {
            // println!("Midi.stop_instru_connection_monitor: Already stopped.");
            return;
        }
        for stopper_sender in self.connection_monitor_stopper_senders.iter() {
            stopper_sender.send(()).unwrap_or_else(|_| {
                // println!("Midi.stop_instru_connection_monitor: Failed to send stop signal to instrument connection monitor");
            });
        }
        // println!("Midi.stop_instru_connection_monitor: Stopped monitor thread.");
        self.is_connection_monitor_running = false;
        // println!("Midi.stop_instru_connection_monitor: Done.");
    }

    pub fn on_updating_tuning() {
        IS_UPDATING_TUNING.store(true, Ordering::Relaxed);
    }

    /// Call the subscribed callback functions on a separate thread.
    fn call_back(callbacks: Arc<Mutex<Vec<Box<dyn Fn() + Send + Sync + 'static>>>>) {
        rayon::spawn(move || {
            let callbacks_guard = callbacks.lock().unwrap();
            for callback in callbacks_guard.iter() {
                callback();
            }
        });
    }

    fn connect_input_port(
        &mut self,
        index: usize,
        port_strategy: &dyn PortStrategy,
    ) -> Result<(), Box<dyn Error>> {
        // println!("Midi.connect_input_port: start");
        self.disconnect_input_port();
        let input: &mut Io<MidiInputPort> = &mut self.input;
        if let Some(port) = input.ports().get(index) {
            // println!("Midi.connect_input_port: found port");
            let device_name = port.device_name();
            let midi_port = port.midi_port();
            let midi_input = Self::create_midi_input();
            match midi_input.connect(
                midi_port,
                &device_name,
                move |_, message, _| Self::on_message_received(message),
                (),
            ) {
                Ok(connection) => {
                    input.set_port(port.clone());
                    let connection_option = Option::from(connection);
                    self.input_connection = connection_option;
                    // println!("Midi.connect_input_port: success");
                }
                Err(_) => {
                    // println!("Midi.connect_input_port: error");
                    // See comment in connect_output_port.
                    return Err(port_strategy.msg_cannot_connect(&device_name).into());
                }
            }
        }
        Ok(())
    }

    fn connect_output_port(
        &mut self,
        index: usize,
        port_strategy: &dyn PortStrategy,
    ) -> Result<(), Box<dyn Error>> {
        self.disconnect_output_port();
        let output: &mut Io<MidiOutputPort> = &mut self.output;
        if let Some(port) = output.ports().get(index) {
            let device_name = port.device_name();
            let midi_port = port.midi_port();
            let midi_output = Self::create_midi_output();
            match midi_output.connect(midi_port, &device_name) {
                Ok(connection) => {
                    *OUTPUT_CONNECTION.lock()? = Option::from(connection);
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
                {
                    return Err(port_strategy.msg_cannot_connect(&device_name).into());
                }
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
        let input_connection = self.input_connection.take();
        if let Some(connection) = input_connection {
            connection.close();
            let input = &mut self.input;
            input.set_port_to_none();
        }
    }

    fn disconnect_output_port(&mut self) {
        // println!("Midi.disconnect_output_port start");
        let output_connection = OUTPUT_CONNECTION.lock().unwrap().take();
        if let Some(connection) = output_connection {
            connection.close();
            let output = &mut self.output;
            output.set_port_to_none();
        }
    }

    fn log_message_received_time() {
        let now = Instant::now();
        *LAST_MESSAGE_RECEIVED_TIME.lock().unwrap() = Some(now);
        let has_instru_just_connected = !IS_INSTRU_CONNECTED.load(Ordering::Relaxed);
        IS_INSTRU_CONNECTED.store(true, Ordering::Relaxed);
        if has_instru_just_connected {
            Self::call_back(INSTRU_CONNECTED_CHANGED_CALLBACKS.clone());
            IS_AWAITING_EDITOR_DATA_DOWNLOAD_COMPLETED.store(true, Ordering::Relaxed);
        }
    }

    /// Monitor the connection status of the instrument.
    /// When the instrument has nothing else to send, it will send a sequence of heartbeat messages
    /// once a second. So, if we have not heard from the instrument for two seconds,
    /// we assume it has disconnected.
    fn monitor_instru_connection(stopper_receiver: mpsc::Receiver<()>) {
        let start_time = Instant::now();
        let mut has_initially_not_connected_callback_been_called = false;
        loop {
            if IS_INSTRU_CONNECTED.load(Ordering::Relaxed) {
                let now = Instant::now();
                let last_message_received_time =
                    *LAST_MESSAGE_RECEIVED_TIME.lock().unwrap();
                if let Some(last_message_received_time) = last_message_received_time {
                    let duration = now.duration_since(last_message_received_time);
                    let seconds = duration.as_secs();
                    if seconds > 2 {
                        // println!("midi.monitor_instru_connection: Instrument disconnected.");
                        IS_INSTRU_CONNECTED.store(false, Ordering::Relaxed);
                        Self::call_back(INSTRU_CONNECTED_CHANGED_CALLBACKS.clone());
                    }
                }
            } else if !has_initially_not_connected_callback_been_called {
                let now = Instant::now();
                let duration = now.duration_since(start_time);
                let seconds = duration.as_secs();
                // Give a chance for the instrument heartbeat messages to arrive.
                if seconds > 2 {
                    // println!("midi.monitor_instru_connection: Instrument not connected for 2 seconds on startup.");
                    // Not connected for 2 seconds after application start.
                    // So we can assume that the instrument is not yet connected.
                    // Provide an opportunity for a helpful message to be displayed.
                    Self::call_back(INSTRU_CONNECTED_CHANGED_CALLBACKS.clone());
                    has_initially_not_connected_callback_been_called = true;
                }
            }
            if let Ok(_) = stopper_receiver.recv_timeout(Duration::from_millis(500)) {
                // Sleep was interrupted
                return;
            }
            // Slept for 500ms, proceeding
        }
    }

    fn on_message_received(message: &[u8]) {
        Self::log_message_received_time();
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
                            // A pitch table has been loaded.
                            // This message is received as part of instrument config,
                            // and when a pitch table update sent to the instrument has been
                            // completed and loaded.
                            // println!("midi.on_message_received: Pitch table loaded");
                            if IS_UPDATING_TUNING.load(Ordering::Relaxed) {
                                println!("midi.on_message_received: Pitch table update confirmed");
                                IS_UPDATING_TUNING.store(false, Ordering::Relaxed);
                                Self::call_back(TUNING_UPDATED_CALLBACKS.clone());
                            }
                            return;
                        }
                        if controller == 109 {
                            if value == 54 {
                                *DOWNLOAD_STATUS.lock().unwrap() = DownloadStatus::BeginUserNames;
                                return;
                            }
                            if value == 55 {
                                *DOWNLOAD_STATUS.lock().unwrap() = DownloadStatus::EndUserNames;
                                return;
                            }
                        }
                        if controller == 56 && value == 20 {
                            // s_Mat_Poke: Start of Matrix stream
                            // println!("midi.on_message_received: Start of Matrix stream");
                            IS_MATRIX_STREAMING.store(true, Ordering::Relaxed);
                            let initial_surface_processing =
                                INITIAL_SURFACE_PROCESSING.lock().unwrap();
                            if initial_surface_processing.is_none() {
                                // println!(
                                //     "Midi.on_message_received: Start of initial matrix stream");
                                // This can happens twice before
                                // initial_surface_processing is stored.
                                // Probably when the editor is opened after this application,
                                // as the editor will also request the config on startup.
                                IS_INITIAL_MATRIX_STREAMING.store(true, Ordering::Relaxed);
                            }
                        }
                    }
                }
                MidiMessage::Aftertouch { key, vel } => {
                    let channel1 = u8::from(channel) + 1; // 1-based channel number.
                    if channel1 == 16 && key == 56 {
                        if IS_MATRIX_STREAMING.load(Ordering::Relaxed) {
                            // PreservSurf: Preset Loading Surface Processing (global)
                            // println!("Midi.on_message_received: Surface Processing");
                            if IS_INITIAL_MATRIX_STREAMING.load(Ordering::Relaxed) {
                                let mut initial_surface_processing =
                                    INITIAL_SURFACE_PROCESSING.lock().unwrap();
                                let preset_loading: PresetLoading = match u8::from(vel) {
                                    0 => PresetLoading::Replace,
                                    _ => PresetLoading::Preserve,
                                };
                                *initial_surface_processing = Option::from(preset_loading);
                                // We are not waiting for anything else from the matrix stream,
                                // and it does not have an end-of-stream message.
                                IS_INITIAL_MATRIX_STREAMING.store(false, Ordering::Relaxed);
                                // println!(
                                //     "Midi.on_message_received: initial_surface_processing = {:?}",
                                //     preset_loading);
                            }
                            // We are not waiting for anything else from the matrix stream,
                            // and it does not have an end-of-stream message.
                            IS_MATRIX_STREAMING.store(false, Ordering::Relaxed);
                        }
                    }
                }
                MidiMessage::ProgramChange { .. } => {
                    let channel1 = u8::from(channel) + 1; // 1-based channel number.
                    if channel1 == 16 {
                        let is_getting_config = IS_GETTING_CONFIG.load(Ordering::Relaxed);
                        if is_getting_config {
                            // This is the last item sent when config has been requested.
                            // println!("Midi.on_message_received: config received");
                            IS_GETTING_CONFIG.store(false, Ordering::Relaxed);
                            Self::call_back(CONFIG_RECEIVED_CALLBACKS.clone());
                            return;
                        }
                        let download_status = *DOWNLOAD_STATUS.lock().unwrap();
                        if download_status == DownloadStatus::EndUserNames {
                            *DOWNLOAD_STATUS.lock().unwrap() = DownloadStatus::None;
                        }
                    }
                }
                _ => {}
            },
            _ => {}
        }
    }

    fn refresh_input_devices(&mut self, input_device_name: &str) -> Result<(), Box<dyn Error>> {
        // println!("Midi.refresh_input_devices: start");
        self.disconnect_input_port();
        self.input.populate_devices(input_device_name)?;
        Ok(())
    }

    fn refresh_output_devices(&mut self, output_device_name: &str) -> Result<(), Box<dyn Error>> {
        // println!("Midi.refresh_output_devices: start");
        self.disconnect_output_port();
        self.output.populate_devices(output_device_name)?;
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
        // println!("Midi.send_message");
        let mut connection_option =
            OUTPUT_CONNECTION.lock().unwrap();
        // println!("Midi.send_message: Got connection");
        if let Some(connection) = connection_option.as_mut() {
            connection
                // unwrap_or_else sometimes freezes when showing the error message on close.
                // unwrap will panic on error, which may give use better diagnostics.
                .send(message).unwrap()
                // .send(message)
                // .unwrap_or_else(|_| println!("Error when sending message ..."));
        }
    }

    const INPUT_CLIENT_NAME: &str = "My MIDI Input";
    const OUTPUT_CLIENT_NAME: &str = "My MIDI Output";
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum DownloadStatus {
    None,
    BeginUserNames,
    EndUserNames,
    EndConfig,
}

lazy_static! {
    static ref CONFIG_RECEIVED_CALLBACKS:
        Arc<Mutex<Vec<Box<dyn Fn() + Send + Sync + 'static>>>> = Arc::new(Mutex::new(Vec::new()));
    static ref DOWNLOAD_STATUS: Arc<Mutex<DownloadStatus>> = Arc::new(Mutex::new(DownloadStatus::None));
    static ref EDITOR_DATA_DOWNLOAD_COMPLETED_CALLBACKS:
        Arc<Mutex<Vec<Box<dyn Fn() + Send + Sync + 'static>>>> = Arc::new(Mutex::new(Vec::new()));
    /// Initial Preset Loading Surface Processing global setting
    static ref INITIAL_SURFACE_PROCESSING:
        Arc<Mutex<Option<PresetLoading>>> = Arc::new(Mutex::new(None));
    static ref INSTRU_CONNECTED_CHANGED_CALLBACKS:
        Arc<Mutex<Vec<Box<dyn Fn() + Send + Sync + 'static>>>> = Arc::new(Mutex::new(Vec::new()));
    static ref IS_AWAITING_EDITOR_DATA_DOWNLOAD_COMPLETED: AtomicBool = AtomicBool::new(false);
    static ref IS_GETTING_CONFIG: AtomicBool = AtomicBool::new(false);
    static ref IS_INITIAL_MATRIX_STREAMING: AtomicBool = AtomicBool::new(false);
    static ref IS_INSTRU_CONNECTED: AtomicBool = AtomicBool::new(false);
    static ref IS_MATRIX_STREAMING: AtomicBool = AtomicBool::new(false);
    static ref IS_UPDATING_TUNING: AtomicBool = AtomicBool::new(false);
    static ref LAST_MESSAGE_RECEIVED_TIME: Mutex<Option<Instant>> = Mutex::new(None);
    static ref OUTPUT_CONNECTION: Mutex<Option<MidiOutputConnection>> = Mutex::new(None);
    static ref TUNING_UPDATED_CALLBACKS:
        Arc<Mutex<Vec<Box<dyn Fn() + Send + Sync + 'static>>>> = Arc::new(Mutex::new(Vec::new()));
}
