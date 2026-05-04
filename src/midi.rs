mod midi_refs;
use midi_refs::{Callbacks, DownloadStatus, TuningStatus};
use midi_refs::{download_completed_callbacks, download_started_callbacks,
                download_status,
                download_wait_start_time,
                last_message_received_time, new_preset_selected_callbacks, output_connection,
                ports_connected_changed_callbacks,
                receiving_data_started_callbacks, receiving_data_stopped_callbacks,
                tuning_status, tuning_updated_callbacks, updating_tuning_callbacks};
use midir::{
    MidiInput, MidiInputConnection, MidiInputPort, MidiOutput, MidiOutputPort,
};
use midly::{MidiMessage, live::LiveEvent};
use std::error::Error;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::time::{Duration, Instant};
use std::thread::sleep;
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

    pub fn add_init_download_started_callback(
        &mut self,
        callback: Box<dyn Fn() + Send + Sync + 'static>,
    ) {
        // println!("Midi.add_init_download_started_callback");
        download_started_callbacks().lock().unwrap().push(callback);
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
        // println!("Midi.add_receiving_data_stopped_callback");
        receiving_data_stopped_callbacks().lock().unwrap().push(callback);
    }

    pub fn add_tuning_updated_callback(&mut self, callback: Box<dyn Fn() + Send + Sync + 'static>) {
        // println!("Midi.add_tuning_updated_callback");
        tuning_updated_callbacks().lock().unwrap().push(callback);
    }

    pub fn add_updating_tuning_callback(&mut self, callback: Box<dyn Fn() + Send + Sync + 'static>) {
        // println!("Midi.add_updating_tuning_callback");
        updating_tuning_callbacks().lock().unwrap().push(callback);
    }

    /// Return whether both input and output ports are connected.
    pub fn are_ports_connected(&self) -> bool {
        if self.input_connection.is_none() {
            return false;
        }
        self.is_output_port_connected()
    }

    pub fn close(&mut self) {
        // println!("Midi.close");
        self.disconnect_input_port();
        self.disconnect_output_port();
        // self.stop_download_monitor();
        self.stop_instrument_connection_monitor();
    }

    pub fn connect_port(
        &mut self,
        index: usize,
        port_strategy: &dyn PortStrategy,
    ) -> Result<(), Box<dyn Error>> {
        let were_ports_connected = self.are_ports_connected();
        // self.stop_download_monitor();
        self.stop_instrument_connection_monitor();
        match port_strategy.port_type() {
            PortType::Input => self.connect_input_port(index, port_strategy)?,
            PortType::Output => self.connect_output_port(index, port_strategy)?,
        }
        if !were_ports_connected {
            // The other port was already connected, so now they both are.
            if self.are_ports_connected() {
                // println!("Midi.connect_port {:?}: Calling ports_connected_changed_callbacks() \
                // because both ports are now connected", port_strategy.port_type());
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

    pub fn has_downloaded_init_data(&self) -> bool {
        *download_status().lock().unwrap() == DownloadStatus::Complete
    }

    pub fn is_output_port_connected(&self) -> bool {
        output_connection().lock().unwrap().is_some()
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
        // self.stop_download_monitor();
        self.stop_instrument_connection_monitor();
        match port_strategy.port_type() {
            PortType::Input => self.refresh_input_devices(device_name)?,
            PortType::Output => self.refresh_output_devices(device_name)?,
        }
        if were_ports_connected {
            // We have just disconnected one of the ports.
            // println!("Midi.refresh_devices: Calling ports_connected_changed_callbacks() because we have just disconnected one of the ports");
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

    pub fn start_instrument_connection_monitor(&mut self) {
        // println!("Midi.start_instrument_connection_monitor");
        *last_message_received_time().lock().unwrap() = None;
        let (stopper_sender, stopper_receiver) = mpsc::channel();
        self.connection_monitor_stopper_sender = Some(stopper_sender);
        rayon::spawn(move || {
            Self::monitor_instrument_connection(stopper_receiver);
        });
        self.is_connection_monitor_running = true;
    }

    pub fn stop_instrument_connection_monitor(&mut self) {
        // println!("Midi.stop_instrument_connection_monitor");
        if !self.is_connection_monitor_running {
            // println!("Midi.stop_instrument_connection_monitor: Already stopped.");
            return;
        }
        let stopper_sender =
            self.connection_monitor_stopper_sender.take();
        if stopper_sender.is_none() { return; }
        stopper_sender.unwrap().send(()).unwrap_or_else(|_| {
            panic!("Midi.stop_instrument_connection_monitor: Failed to send stop signal to connection monitor");
        });
        // println!("Midi.stop_instrument_connection_monitor: Stopped monitor thread.");
        self.is_connection_monitor_running = false;
        IS_RECEIVING_DATA.store(false, Ordering::Relaxed);
        // println!("Midi.stop_instrument_connection_monitor: Done.");
    }

    pub fn on_updating_tuning() {
        println!("Midi.on_updating_tuning");
        *tuning_status().lock().unwrap() = TuningStatus::Tuning;
        Self::call_back(updating_tuning_callbacks().clone());
    }

    /// Call the subscribed callback functions on a separate thread.
    fn call_back(shared_callbacks: Callbacks) {
        rayon::spawn(move || {
            let callbacks = shared_callbacks.lock().unwrap();
            for callback in callbacks.iter() {
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
        // If we have not yet received any messages from the instrument after 2 seconds,
        // show a message to the user.
        // Is this redundant?
        rayon::spawn(move || {
            sleep(Duration::from_secs(MIDI_WAIT_SECS));
            if !IS_RECEIVING_DATA.load(Ordering::Relaxed) {
                // println!k("Midi.connect_input_port: Stopped receiving data");
                Self::call_back(receiving_data_stopped_callbacks().clone());
            }
        });
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
        // println!("Midi.log_message_received_time: Start");
        IS_RECEIVING_DATA.store(true, Ordering::Relaxed);
        let mut last_time =
            last_message_received_time().lock().unwrap();
        let prev_message_received_time =
            last_time.take();
        *last_time = Some(now);
        if prev_message_received_time.is_none() {
            // This is the first message we have received since monitoring for messages started.
            // We need to wait for the initial data download to the editor to complete, if
            // it did not already happen before we started listening.
            // However, we will be judging the download to be complete either when we receive
            // the last download message or if there's been no data received for 0.2 seconds.
            // And, on my Continuum at least, the initial data download to the editor starts
            // 3 to 4 seconds after turning the instrument on. So, to be safe, we will wait 6
            // seconds to give the download a chance to start, if it is going to,
            // before we start monitoring for download completion.
            *download_status().lock().unwrap() = DownloadStatus::Waiting;
            // println!("Midi.log_message_received_time: Setting download wait start time");
            *download_wait_start_time().lock().unwrap() = Some(now);
            // println!("Midi.log_message_received_time: receiving_data_started_callbacks");
            Self::call_back(receiving_data_started_callbacks().clone());
            return;
        }
        if IS_MONITORING_DOWNLOAD.load(Ordering::Relaxed) {
            let millis_since_prev =
                now.duration_since(prev_message_received_time.unwrap()).as_millis();
            if millis_since_prev >= 200 {
                // The initial data download consists of many messages in quick succession.
                // Or this could be some other burst of messages, such as the heartbeat cluster.
                // Either way, as we have not received any more messages for 200 ms,
                // the burst of messages must have stopped.
                Self::on_init_data_download_completed();
            }
            return;
        }
        // Check whether it is time to start monitoring the initial data download.
        // If we have not started monitoring and the download has not completed in the meantime,
        // the download status will either be Checking or, if the download has already started,
        // something else other than Complete.
        if *download_status().lock().unwrap() != DownloadStatus::Complete {
            let wait_duration = now.duration_since(
                *download_wait_start_time().lock().unwrap().as_ref().unwrap());
            let wait_secs = wait_duration.as_secs();
            if wait_secs < 6 {
                // println!("Midi.log_message_received_time: Waited {} seconds so far to start download monitor", wait_secs);
                return;
            }
            // println!("Midi.log_message_received_time: Six seconds is up");
            // We have waited 6 seconds, and the download has either not started or is in progress.
            // So, we can start monitoring the download.
            // println!("Midi.log_message_received_time: Starting download monitor");
            IS_MONITORING_DOWNLOAD.store(true, Ordering::Relaxed);
            return;
        }
    }

    fn on_init_data_download_completed() {
        // println!("Midi.on_init_data_download_completed: Stopping download monitor");
        IS_MONITORING_DOWNLOAD.store(false, Ordering::Relaxed);
        IS_DOWNLOADING_INIT_DATA.store(false, Ordering::Relaxed);
        *download_status().lock().unwrap() = DownloadStatus::Complete;
        Self::call_back(download_completed_callbacks().clone());
    }

    /// Monitor the connection status of the instrument.
    /// When the instrument has nothing else to send, it will send a sequence of heartbeat messages
    /// once a second and the editor will send back a sequence of heartbeat messages less than half
    /// a second later. This application will receive both sets of heartbeat messages. To be safe,
    /// if we have not had any data for 2 seconds, we assume
    /// the editor or instrument has disconnected.
    fn monitor_instrument_connection(stopper_receiver: mpsc::Receiver<()>) {
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
                    if seconds > MIDI_WAIT_SECS {
                        // println!("midi.monitor_instrument_connection: Instrument disconnected.");
                        *last_message_received_time().lock().unwrap() = None;
                        IS_RECEIVING_DATA.store(false, Ordering::Relaxed);
                        Self::call_back(receiving_data_stopped_callbacks().clone());
                    }
                }
            } else if !has_initially_not_connected_callback_been_called {
                let now = Instant::now();
                let duration = now.duration_since(start_time);
                let seconds = duration.as_secs();
                // Give a chance for the instrument heartbeat messages to arrive.
                if seconds > MIDI_WAIT_SECS {
                    // println!("midi.monitor_instrument_connection: Instrument not connected for 2 seconds on startup.");
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

    fn  on_message_received(message: &[u8]) {
        // println!("Midi.on_message_received: message={:?}", message);
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
                    if controller == 51 { // Grid
                        let pitch_table = u8::from(value);
                        // println!("midi.on_message_received: Pitch table {}", pitch_table);
                        // A pitch table has been loaded to the instrument's current preset.
                        // This message is received as part of instrument config,
                        // and when a pitch table update sent to the instrument has been
                        // completed and loaded.
                        let status = *tuning_status().lock().unwrap();
                        // Workaround for firmware 10.73 Beta not sending update confirmation
                        // for some presets.
                        if status == TuningStatus::Tuning {
                            // Check that the value is the correct pitch table index
                            // for the tuning this application sent to the instrument:
                            // when a preset is loaded, there will be a Grid message
                            // for the preset's initial tuning table, which will be zero.
                            if pitch_table == tuner::pitch_table() {
                                println!("midi.on_message_received: Preset's pitch table \
                                            update requested, pitch table no: {}", pitch_table);
                                *tuning_status().lock().unwrap() = TuningStatus::None;
                                Self::call_back(tuning_updated_callbacks().clone());
                            }
                        }
                        // When the firmware bug is fixed, remove the above workaround
                        // and restore the tuning update confirmation check below.
                        // This will fix the problem described in a comment in
                        // Controller.await_tuning_updated.
                        // match status {
                        //     TuningStatus::None => {}
                        //     TuningStatus::Tuning => {
                        //         // Check that the value is the correct pitch table index
                        //         // for the tuning this application sent to the instrument.
                        //         // When there have been problems at the instrument end,
                        //         // it has sent back a ch16 cc51 messages, but with value 0.
                        //         if pitch_table == tuner::pitch_table() {
                        //             // The editor sends us back what we send to the instrument,
                        //             // as well as what the instrument sends back to us.
                        //             // So we have just requested that the current preset be updated
                        //             // with the new pitch table.
                        //             println!("midi.on_message_received: Preset's pitch table \
                        //                 update requested, pitch table no: {}", pitch_table);
                        //             *tuning_status().lock().unwrap() =
                        //                 TuningStatus::RequestedPresetUpdate;
                        //         }
                        //     }
                        //     TuningStatus::RequestedPresetUpdate => {
                        //         // The instrument has confirmed that the current preset has been
                        //         // updated with the new pitch table.
                        //         // As at firmware 10.73, there is a firmware bug where, for
                        //         // specific presets, the instrument will send back a cc51 message
                        //         // with value 0 instead of the pitch table no we requested.
                        //         // Haken Audio Incident 2335
                        //         // https://github.com/SimonORorke/PitchGrid-Continuum-Bridge/issues/5
                        //         // So we can omit checking the pitch table no here.
                        //         println!("midi.on_message_received: Preset's pitch table \
                        //                 update confirmed, pitch table no: {}", pitch_table);
                        //         *tuning_status().lock().unwrap() = TuningStatus::None;
                        //         Self::call_back(tuning_updated_callbacks().clone());
                        //     }
                        // }
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
                #[allow(unused_variables)]
                MidiMessage::ProgramChange { program } => {
                    let channel1 = u8::from(channel) + 1; // 1-based channel number.
                    if channel1 == 16 {
                        // When the editor requests a preset load, which can be seen in the
                        // editor's console log but not here, the program number is zero-based.
                        // When the instrument confirms that the preset has been loaded,
                        // which we see here, the program number is one-based.
                        // println!("midi.on_message_received: ProgramChange ch16 program {}", program);
                        let download_status = *download_status().lock().unwrap();
                        // I don't think this will work if system presets are downloaded.
                        // But it's a rare occurrence; and the user will be able to work around it.
                        if download_status == DownloadStatus::EndUserNames
                            || download_status == DownloadStatus::EndSysNames {
                            // println!("Midi.on_message_received: End of download");
                            Self::on_init_data_download_completed();
                            return;
                        }
                        if download_status == DownloadStatus::Complete {
                            // The user is selecting a preset. The editor sends the preset's
                            // zero-based program number after the bank.
                            // For unknown reason, this happens twice when a preset is loaded
                            // from disc.
                            // println!("midi.on_message_received: Program, preset selected");
                            // *preset_select_status().lock().unwrap() = PresetSelectStatus::None;
                            Self::call_back(new_preset_selected_callbacks().clone());
                            return;
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
            connection.send(message).unwrap_or_else(|_| {
                println!("Error when sending MIDI message: {:?}", message);
                // Panic for stack trace diagnostics.
                // panic!("Error when sending MIDI message: {:?}", message);
            });
        }
    }
}

const INPUT_CLIENT_NAME: &str = "My MIDI Input";
const MIDI_WAIT_SECS: u64 = 2;
const OUTPUT_CLIENT_NAME: &str = "My MIDI Output";

static IS_DOWNLOADING_INIT_DATA: AtomicBool = AtomicBool::new(false);
static IS_MONITORING_DOWNLOAD: AtomicBool = AtomicBool::new(false);
static IS_RECEIVING_DATA: AtomicBool = AtomicBool::new(false);
