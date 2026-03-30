mod statics;
use statics::{Callbacks, DownloadStatus, PresetSelectStatus};
use statics::{download_completed_callbacks, download_monitor_stopper_sender, download_status,
              last_message_received_time, new_preset_selected_callbacks, output_connection,
              ports_connected_changed_callbacks, preset_select_status,
              receiving_data_started_callbacks, receiving_data_stopped_callbacks,
              tuning_updated_callbacks};
use midir::{
    MidiInput, MidiInputConnection, MidiInputPort, MidiOutput, MidiOutputPort,
};
use midly::{MidiMessage, live::LiveEvent};
use std::error::Error;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::time::{Duration, Instant};
use crate::global::{PortType};
use crate::midi_ports::{Io, IIo};
use crate::port_strategy::PortStrategy;
use crate::tuner;

pub struct Midi {
    connection_monitor_stopper_sender: Option<mpsc::Sender<()>>,
    input: Io<MidiInputPort>,
    input_connection: Option<MidiInputConnection<()>>,
    is_connection_monitor_running: bool,
    output: Io<MidiOutputPort>,
}

impl Midi {
    pub fn new() -> Self {
        Self {
            connection_monitor_stopper_sender: None,
            input: Io::<MidiInputPort>::new(Box::new(Self::create_midi_input())),
            input_connection: None,
            is_connection_monitor_running: false,
            output: Io::<MidiOutputPort>::new(Box::new(Self::create_midi_output())),
        }
    }

    pub fn add_init_download_completed_callback(
        &mut self,
        callback: Box<dyn Fn() + Send + Sync + 'static>,
    ) {
        // println!("Midi.add_init_download_completed_callback");
        download_completed_callbacks().lock().unwrap().push(callback);
    }

    pub fn add_ports_connected_changed_callback(
        &mut self,
        callback: Box<dyn Fn() + Send + Sync + 'static>,
    ) {
        // println!("Midi.add_tuning_updated_callback");
        ports_connected_changed_callbacks().lock().unwrap().push(callback);
    }

    pub fn add_new_preset_selected_callback(
        &mut self,
        callback: Box<dyn Fn() + Send + Sync + 'static>,
    ) {
        // println!("Midi.add_new_preset_selected_callback");
        new_preset_selected_callbacks().lock().unwrap().push(callback);
    }

    pub fn add_receiving_data_started_callback(
        &mut self,
        callback: Box<dyn Fn() + Send + Sync + 'static>,
    ) {
        // println!("Midi.add_receiving_data_started_callback");
        receiving_data_started_callbacks().lock().unwrap().push(callback);
    }

    pub fn add_receiving_data_stopped_callback(
        &mut self,
        callback: Box<dyn Fn() + Send + Sync + 'static>,
    ) {
        // println!("Midi.add_receiving_data_started_callback");
        receiving_data_stopped_callbacks().lock().unwrap().push(callback);
    }

    pub fn add_tuning_updated_callback(&mut self, callback: Box<dyn Fn() + Send + Sync + 'static>) {
        // println!("Midi.add_tuning_updated_callback");
        tuning_updated_callbacks().lock().unwrap().push(callback);
    }

    /// Return whether both input and output ports are connected.
    pub fn are_ports_connected(&self) -> bool {
        if self.input_connection.is_none() {
            return false;
        }
        output_connection().lock().unwrap().is_some()
    }

    pub fn close(&mut self) {
        // println!("Midi.close");
        self.disconnect_input_port();
        self.disconnect_output_port();
        self.stop_download_monitor();
        self.stop_instru_connection_monitor();
    }

    pub fn connect_port(
        &mut self,
        index: usize,
        port_strategy: &dyn PortStrategy,
    ) -> Result<(), Box<dyn Error>> {
        let were_ports_connected = self.are_ports_connected();
        self.stop_download_monitor();
        self.stop_instru_connection_monitor();
        match port_strategy.port_type() {
            PortType::Input => self.connect_input_port(index, port_strategy)?,
            PortType::Output => self.connect_output_port(index, port_strategy)?,
        }
        if !were_ports_connected {
            // The other port was already connected, so now they both are.
            if self.are_ports_connected() {
                Self::call_back(ports_connected_changed_callbacks().clone());
            }
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

    pub fn is_downloading_init_data(&self) -> bool {
        IS_DOWNLOADING_INIT_DATA.load(Ordering::Relaxed)
    }

    /// We should receive data from the instrument at least once per second, as it sends heartbeat
    /// messages at 1-second intervals when not otherwise busy.
    /// So, we can use this method to check if the instrument is still connected.
    pub fn is_receiving_data(&self) -> bool {
        IS_RECEIVING_DATA.load(Ordering::Relaxed)
    }

    pub fn output(&self) -> &Io<MidiOutputPort> {
        &self.output
    }

    pub fn refresh_devices(
        &mut self,
        device_name: &str,
        port_strategy: &dyn PortStrategy,
    ) -> Result<(), Box<dyn Error>> {
        let were_ports_connected = self.are_ports_connected();
        self.stop_download_monitor();
        self.stop_instru_connection_monitor();
        match port_strategy.port_type() {
            PortType::Input => self.refresh_input_devices(device_name)?,
            PortType::Output => self.refresh_output_devices(device_name)?,
        }
        if were_ports_connected {
            // We have just disconnected on of the ports.
            Self::call_back(ports_connected_changed_callbacks().clone());
        }
        Ok(())
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

    pub fn start_instru_connection_monitor(&mut self) {
        // println!("Midi.start_instru_connection_monitor");
        let (stopper_sender, stopper_receiver) = mpsc::channel();
        self.connection_monitor_stopper_sender = Some(stopper_sender);
        rayon::spawn(move || {
            Self::monitor_instru_connection(stopper_receiver);
        });
        self.is_connection_monitor_running = true;
    }

    pub fn stop_instru_connection_monitor(&mut self) {
        // println!("Midi.stop_instru_connection_monitor");
        if !self.is_connection_monitor_running {
            // println!("Midi.stop_instru_connection_monitor: Already stopped.");
            return;
        }
        let stopper_sender =
            self.connection_monitor_stopper_sender.take();
        if stopper_sender.is_none() { return; }
        stopper_sender.unwrap().send(()).unwrap_or_else(|_| {
            panic!("Midi.stop_instru_connection_monitor: Failed to send stop signal to connection monitor");
        });
        // println!("Midi.stop_instru_connection_monitor: Stopped monitor thread.");
        self.is_connection_monitor_running = false;
        IS_RECEIVING_DATA.store(false, Ordering::Relaxed);
        // println!("Midi.stop_instru_connection_monitor: Done.");
    }

    pub fn on_updating_tuning() {
        IS_UPDATING_TUNING.store(true, Ordering::Relaxed);
    }

    /// Call the subscribed callback functions on a separate thread.
    fn call_back(callbacks: Callbacks) {
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
                    *output_connection().lock()? = Option::from(connection);
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
        MidiInput::new(INPUT_CLIENT_NAME).unwrap()
    }

    fn create_midi_output() -> MidiOutput {
        MidiOutput::new(OUTPUT_CLIENT_NAME).unwrap()
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
        let connection_opt = output_connection().lock().unwrap().take();
        if let Some(connection) = connection_opt {
            connection.close();
            let output = &mut self.output;
            output.set_port_to_none();
        }
    }

    fn log_message_received_time() {
        let now = Instant::now();
        IS_RECEIVING_DATA.store(true, Ordering::Relaxed);
        let mut last_time =
            last_message_received_time().lock().unwrap();
        let prev_message_received_time =
            last_time.take();
        *last_time = Some(now);
        if prev_message_received_time.is_none() {
            HAS_JUST_STARTED_RECEIVING_DATA.store(true, Ordering::Relaxed);
            Self::call_back(receiving_data_started_callbacks().clone());
            return;
        }
        if HAS_JUST_STARTED_RECEIVING_DATA.load(Ordering::Relaxed) {
            HAS_JUST_STARTED_RECEIVING_DATA.store(false, Ordering::Relaxed);
            let duration = now.duration_since(prev_message_received_time.unwrap());
            let millis = duration.as_millis();
            // This is the second message we have received since the application started or
            // listening for messages was resumed.
            // The initial data download from the instrument to the editor consists of many
            // messages in quick succession.
            // So, if we have received the first two messages within 100ms of each other,
            // we assume an initial data download from the instrument to the editor or some other
            // burst of messages, such as the heartbeat cluster, is in progress.
            // Otherwise, we assume the initial data download occurred before we started listening,
            // which usually means before this application was launched.
            if millis < 100 {
                // We need to defer sending data until the download message burst
                // is complete. So start monitoring the download to ascertain when it finishes.
                // println!("Midi.log_message_received_time: Starting download monitor");
                Self::start_download_monitor();
            }
        }
    }

    /// Monitors the initial data download from the instrument to the editor to ascertain when
    /// it has finished, when monitoring will stop.
    fn monitor_data_download(stopper_receiver: mpsc::Receiver<()>) {
        // println!("Midi.monitor_editor_data_download");
        loop {
            if let Ok(_) = stopper_receiver.recv_timeout(Duration::from_millis(200)) {
                // Sleep was interrupted
                return;
            }
            // Slept for 200ms, proceeding
            let status = *download_status().lock().unwrap();
            if status == DownloadStatus::None {
                // The initial data download consists of many messages in quick succession.
                // Or this could be some other burst of messages, such as the heartbeat cluster.
                // Either way, as we have not received any more messages for 200 ms,
                // the burst of messages must have stopped.
                // println!("Midi.monitor_data_download: Download completed");
                IS_DOWNLOADING_INIT_DATA.store(false, Ordering::Relaxed);
                IS_DOWNLOAD_MONITOR_RUNNING.store(false, Ordering::Relaxed);
                Self::call_back(download_completed_callbacks().clone());
                return;
            }
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
            if IS_RECEIVING_DATA.load(Ordering::Relaxed) {
                let now = Instant::now();
                let last_time =
                    *last_message_received_time().lock().unwrap();
                if let Some(last_time) = last_time {
                    let duration = now.duration_since(last_time);
                    let seconds = duration.as_secs();
                    if seconds > 2 {
                        // println!("midi.monitor_instru_connection: Instrument disconnected.");
                        *last_message_received_time().lock().unwrap() = None;
                        IS_RECEIVING_DATA.store(false, Ordering::Relaxed);
                        Self::call_back(receiving_data_started_callbacks().clone());
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
                    Self::call_back(receiving_data_stopped_callbacks().clone());
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
                    if channel1 != 16 {
                        return;
                    }
                    // Channel 16: the instrument's control channel for most parameters.
                    // if controller != 82 && controller != 111 && controller != 114
                    //     && controller != 118 {  // Heartbeats ignored
                    //     println!("Midi.on_message_received: ch{} cc{} value {}",
                    //              channel1, controller, value);
                    // }
                    if controller == 0 // Bank MSB
                        // But if the editor were downloading the user preset list or the
                        // system preset list, this would be one of many.
                        && *download_status().lock().unwrap() == DownloadStatus::None {
                        // The user is selecting a preset;
                        // it's not part of the editor's initial download, after which we will
                        // have already sent a tuning.
                        // println!("midi.on_message_received: Preset selected, BankH");
                        *preset_select_status().lock().unwrap() = PresetSelectStatus::BankH;
                        return;
                    }
                    if controller == 51 { // Grid
                        // println!("midi.on_message_received: Pitch table loaded");
                        // A pitch table has been loaded to the instrument's current preset.
                        // This message is received as part of instrument config,
                        // and when a pitch table update sent to the instrument has been
                        // completed and loaded.
                        if IS_UPDATING_TUNING.load(Ordering::Relaxed) {
                            // Check that the value is the correct pitch table index
                            // for the tuning this application sent to the instrument.
                            // When there have been problems at the instrument end,
                            // it has sent back a ch16 cc51 messages, but with value 0.
                            if u8::from(value) == tuner::pitch_table_no() {
                                // println!("midi.on_message_received: Pitch table update confirmed");
                                IS_UPDATING_TUNING.store(false, Ordering::Relaxed);
                                Self::call_back(tuning_updated_callbacks().clone());
                            }
                        }
                        return;
                    }
                    if controller == 109 {
                        if value == 40 {
                            // println!("midi.on_message_received: EndSysNames");
                            *download_status().lock().unwrap() = DownloadStatus::EndSysNames;
                            return;
                        }
                        if value == 49 {
                            // println!("midi.on_message_received: BeginSysNames");
                            *download_status().lock().unwrap() = DownloadStatus::BeginSysNames;
                            return;
                        }
                        if value == 54 {
                            // println!("midi.on_message_received: BeginUserNames");
                            *download_status().lock().unwrap() = DownloadStatus::BeginUserNames;
                            return;
                        }
                        if value == 55 {
                            // println!("midi.on_message_received: EndUserNames");
                            *download_status().lock().unwrap() = DownloadStatus::EndUserNames;
                            return;
                        }
                    }
                }
                MidiMessage::ProgramChange { .. } => {
                    let channel1 = u8::from(channel) + 1; // 1-based channel number.
                    if channel1 == 16 {
                        // println!("midi.on_message_received: ProgramChange");
                        let dl_status = *download_status().lock().unwrap();
                        if dl_status == DownloadStatus::EndUserNames
                            || dl_status == DownloadStatus::EndSysNames {
                            // println!("Midi.on_message_received: End of download:");
                            *download_status().lock().unwrap() = DownloadStatus::None;
                            return;
                        }
                        let sel_status =
                            *preset_select_status().lock().unwrap();
                        match sel_status {
                            PresetSelectStatus::None => {}
                            PresetSelectStatus::BankH => {
                                // The user is selecting a preset. The editor sends the preset's
                                // zero-based program number after the bank.
                                // For unknown reason, this happens twice when a preset is loaded
                                // from disc.
                                // println!("midi.on_message_received: Preset selected, Program");
                                *preset_select_status().lock().unwrap() = PresetSelectStatus::None;
                                Self::call_back(new_preset_selected_callbacks().clone());
                                return;
                            }
                            // We seem not to get this message when the user has selected a preset.
                            // PresetSelectStatus::Program => {
                            //     // The second program change message when the user has selected a
                            //     // preset is the 1-based preset number that is the last item of
                            //     // preset data sent by the instrument when loading the preset.
                            //     // So the preset load is complete, and we now need to resend the
                            //     // tuning.
                            //     *PRESET_SELECT_STATUS.lock().unwrap() = PresetSelectStatus::None;
                            //     println!("midi.on_message_received: Preset selected, loaded");
                            //     Self::call_back(NEW_PRESET_SELECTED_CALLBACKS.clone());
                            //     return;
                            // }
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
        // println!("Midi.send_message: message={:?}", message);
        let mut connection_option =
            output_connection().lock().unwrap();
        // println!("Midi.send_message: Got connection");
        if let Some(connection) = connection_option.as_mut() {
            // We want a panic on send failure, for stack trace diagnostics.
            connection.send(message).unwrap_or_else(|_| {
                panic!("Error when sending MIDI message: {:?}", message);
            });
        }
    }

    fn start_download_monitor() {
        // println!("Midi.start_download_monitor");
        let (stopper_sender, stopper_receiver) = mpsc::channel();
        *download_monitor_stopper_sender().lock().unwrap() = Some(stopper_sender);
        IS_DOWNLOADING_INIT_DATA.store(true, Ordering::Relaxed);
        IS_DOWNLOAD_MONITOR_RUNNING.store(true, Ordering::Relaxed);
        rayon::spawn(move || {
            Self::monitor_data_download(stopper_receiver);
        });
    }

    fn stop_download_monitor(&mut self) {
        // println!("Midi.stop_download_monitor");
        if !IS_DOWNLOAD_MONITOR_RUNNING.load(Ordering::Relaxed) {
            // println!("Midi.stop_download_monitor: Already stopped.");
            return;
        }
        let stopper_sender =
            download_monitor_stopper_sender().lock().unwrap().take();
        if stopper_sender.is_none() { return; }
        stopper_sender.unwrap().send(()).unwrap_or_else(|_| {
            panic!("Midi.stop_download_monitor: Failed to send stop signal to download monitor");
        });
        // println!("Midi.stop_download_monitor: Stopped monitor thread.");
        IS_DOWNLOAD_MONITOR_RUNNING.store(false, Ordering::Relaxed);
        // println!("Midi.stop_download_monitor: Done.");
    }
}

const INPUT_CLIENT_NAME: &str = "My MIDI Input";
const OUTPUT_CLIENT_NAME: &str = "My MIDI Output";

static HAS_JUST_STARTED_RECEIVING_DATA: AtomicBool = AtomicBool::new(false);
static IS_DOWNLOAD_MONITOR_RUNNING: AtomicBool = AtomicBool::new(false);
static IS_DOWNLOADING_INIT_DATA: AtomicBool = AtomicBool::new(false);
static IS_RECEIVING_DATA: AtomicBool = AtomicBool::new(false);
static IS_UPDATING_TUNING: AtomicBool = AtomicBool::new(false);
