use std::error::Error;
use std::sync::{Arc, Mutex};
use std::sync::mpsc;
use std::time::Duration;
use crate::global::{MessageType};
use crate::i_midi::{MidiCallbacks, SharedMidi};
use crate::i_osc::{IOsc, OscCallbacks};
use crate::osc::Osc;
use crate::i_settings::ISettings;
use crate::device_strategy::{
    InputStrategy, OutputStrategy, DeviceStrategy};
use crate::settings::Settings;
use crate::midi::Midi;
use crate::i_ui_methods::IUiMethods;
use crate::i_tuner::SharedTuner;
use crate::tuner::Tuner;
use crate::tuning_params::TuningParams;

/// This is the main controller in the Model-View-Controller (MVC) pattern.
/// DeviceStrategy contains both view and controller methods.
/// The Slint UI, main.rs and UiMethods are the remainder of the view.
/// Everything else is the model.
pub struct Controller {
    await_tuning_updated_stopper_sender: Option<mpsc::Sender<()>>,
    ui_methods: Box<dyn IUiMethods>,
    is_awaiting_tuning_updated: bool,
    osc: Box<dyn IOsc>,
    settings: Box<dyn ISettings>,
    tuner: SharedTuner,
}

impl Controller {
    pub fn new(callbacks: Box<dyn IUiMethods>) -> Self {
        Self {
            await_tuning_updated_stopper_sender: None,
            ui_methods: callbacks,
            is_awaiting_tuning_updated: false,
            osc: Box::new(Osc::new()),
            settings: Box::new(Settings::new()),
            tuner: Arc::new(Tuner::new()),
        }
    }

    pub fn init(&mut self) {
        // println!("Controller.init");
        let main_window_x: i32;
        let main_window_y: i32;
        let osc_listening_port: u16;
        let pitch_table: u8;
        let input_device_name: String;
        let output_device_name: String;
        let override_rounding_initial: bool;
        let override_rounding_rate: bool;
        let rounding_rate: u8;
        // println!("Controller.init: Reading settings");
        match self.settings.read_from_file() {
            Ok(_) => {
                main_window_x = self.settings.main_window_x();
                main_window_y = self.settings.main_window_y();
                input_device_name = self.settings.midi_input_device().to_string();
                output_device_name = self.settings.midi_output_device().to_string();
                osc_listening_port = {
                    if Osc::listening_ports().contains(&self.settings.osc_listening_port()) {
                        self.settings.osc_listening_port()
                    } else {
                        Osc::default_listening_port()
                    }
                };
                pitch_table = {
                    if Tuner::pitch_tables().contains(&self.settings.pitch_table()) {
                        self.settings.pitch_table()
                    } else {
                        Tuner::default_pitch_table()
                    }
                };
                override_rounding_initial = self.settings.override_rounding_initial();
                override_rounding_rate = self.settings.override_rounding_rate();
                rounding_rate = self.settings.rounding_rate();
            }
            Err(err) => {
                self.show_error(&err.to_string());
                return;
            }
        }
        // println!("Controller.init: Getting midi");
        self.ui_methods.set_main_window_position(main_window_x, main_window_y);
        let mut midi = Midi::midi();
        midi.init(&input_device_name, &output_device_name, Self::clone_controller());
        drop(midi); // Release MIDI lock before calling device_names which needs to acquire it
        let input_strategy = InputStrategy::new();
        let output_strategy = OutputStrategy::new();
        // println!("Controller.init: Getting input device names");
        let input_device_names = self.device_names(&input_strategy);
        // println!("Controller.init: Got {} input device names", input_device_names.len());
        // println!("Controller.init: About to call callbacks.set_devices_model");
        self.ui_methods.set_devices_model(&input_device_names, &input_strategy);
        // println!("Controller.init: Called callbacks.set_devices_model");
        // println!("Controller.init: Setting output devices model");
        self.ui_methods.set_devices_model(&self.device_names(&output_strategy), &output_strategy);
        // println!("Controller.init: Connecting initial MIDI devices");
        self.connect_initial_device(&output_strategy);
        // Don't start listening to MIDI until we are able to send MIDI.
        if Midi::midi().is_output_device_connected() {
            // println!("Controller.init: Connecting input device");
            self.connect_initial_device(&input_strategy);
        }
        self.osc.set_listening_port(osc_listening_port);
        self.ui_methods.set_selected_osc_listening_port_index(Osc::listening_port_index() as i32);
        // println!("Controller.init: Configuring tuner");
        self.tuner.init(pitch_table);
        self.ui_methods.set_selected_pitch_table_index(self.tuner.pitch_table_index() as i32);
        self.tuner.set_override_rounding_initial(override_rounding_initial);
        self.tuner.set_override_rounding_rate(override_rounding_rate);
        self.tuner.set_rounding_rate(rounding_rate);
        self.ui_methods.set_override_rounding_initial(override_rounding_initial);
        self.ui_methods.set_override_rounding_rate(override_rounding_rate);
        self.ui_methods.set_rounding_rate(rounding_rate);
        if Midi::midi().are_devices_connected() {
            // println!("Controller.init: Starting instrument connection monitor");
            self.start_instrument_connection_monitor();
        }
        // println!("Controller.init: Done");
    }

    fn start_instrument_connection_monitor(&mut self) {
        // println!("Controller.start_instrument_connection_monitor");
        self.show_info(CHECKING_INSTRUMENT_CONNECTION);
        Midi::midi().start_instrument_connection_monitor();
        // println!("Controller.start_instrument_connection_monitor: Instrument connection monitor started");
    }

    #[allow(clippy::unwrap_used)]
    pub fn close(&mut self, main_window_x: i32, main_window_y: i32) -> Result<(), Box<dyn Error>> {
        Midi::midi().close();
        self.osc.stop();
        self.settings.set_main_window_x(main_window_x);
        self.settings.set_main_window_y(main_window_y);
        if let Err(err) = self.settings.write_to_file() {
            self.show_error(&err.to_string());
            return Err(err)
        };
        Ok(())
    }

    fn connect_initial_device(&mut self, device_strategy: &dyn DeviceStrategy) {
        // println!("Controller.connect_initial_port: {:?}", device_strategy.device_type());
        let shared_midi = Midi::midi_clone();
        let maybe_index = {
            let midi = shared_midi.lock().unwrap();
            midi.io(device_strategy).device().as_ref()
                .map(|device| device.index())
        };
        if let Some(index) = maybe_index {
            // println!("Controller.connect_initial_port: Setting selected port index to {}", index);
            self.ui_methods.set_selected_device_index(index, device_strategy);
            self.connect_selected_device(&shared_midi, device_strategy);
        } else {
            self.show_no_port_connected(device_strategy);
            self.show_warning(device_strategy.msg_connect());
            self.ui_methods.focus_device(device_strategy);
        }
    }

    pub fn connect_device(&mut self, device_strategy: &dyn DeviceStrategy) {
        // println!("Controller.connect_device");
        let shared_midi = Midi::midi_clone();
        let device_strategy = device_strategy.clone_box();
        // println!("Controller.connect_device: Stopping OSC and instrument connection monitor");
        self.stop_osc_and_instrument_connection_monitor();
        // println!("Controller.connect_device: Showing PitchGrid disconnected");
        self.show_pitchgrid_disconnected();
        // println!("Controller.connect_device: Connecting selected port");
        self.connect_selected_device(&shared_midi, &*device_strategy);
        // println!("Controller.connect_device: Getting port");
        let device_name_opt: Option<String> = shared_midi.lock().unwrap()
            .io(&*device_strategy)
            .device()
            .map(|p| p.name().to_string());
        // println!("Controller.connect_device: Got port");
        if let Some(device_name) = device_name_opt {
            self.show_info(device_strategy.msg_connected(&device_name));
            if Midi::midi().are_devices_connected() {
                // println!("Controller.connect_device: Starting instrument connection monitor");
                self.start_instrument_connection_monitor();
            } else {
                let other_device_strategy =
                    device_strategy.other_device_strategy();
                self.show_warning(other_device_strategy.msg_connect());
            }
        }
        // println!("Controller.connect_device: Done");
    }

    fn connect_selected_device(&mut self, shared_midi: &SharedMidi,
                               device_strategy: &dyn DeviceStrategy) {
        // println!("Controller.connect_selected_device: {:?}", device_strategy.device_type());
        let selected = self.ui_methods.get_selected_device_index(device_strategy);
        let index: usize = match usize::try_from(selected) {
            Ok(i) => i,
            Err(_) => {
                // A port has not been selected. That's impossible with the UI as it is.
                self.show_no_port_connected(device_strategy);
                self.show_error(device_strategy.msg_not_selected());
                return;
            }
        };
        // println!("Controller.connect_selected_device: Selected port index = {}", index);
        let ui_action: Result<String, String> = {
            // println!("Controller.connect_selected_device: Getting midi.");
            let mut midi = shared_midi.lock().unwrap();
            // println!("Controller.connect_selected_device: Got midi.");
            let Some(name) = midi.io(device_strategy).device_names().get(index).cloned()
            else {
                return;
            };
            match midi.connect_device(index, device_strategy) {
                Ok(()) => Ok(name),
                Err(err) => Err(err.to_string()),
            }
        };
        match ui_action {
            Ok(name) => {
                self.show_connected_device_name(&name, device_strategy);
            }
            Err(message) => {
                self.show_no_port_connected(device_strategy);
                self.show_error(&message);
            }
        }
    }

    fn device_names(&self, device_strategy: &dyn DeviceStrategy) -> Vec<String> {
        Midi::midi().io(device_strategy).device_names()
    }

    pub fn refresh_devices(&mut self, device_strategy: &dyn DeviceStrategy) {
        let device_strategy = device_strategy.clone_box();
        self.stop_osc_and_instrument_connection_monitor();
        let device_name = device_strategy.device_setting(&*self.settings).to_string();
        Midi::midi().refresh_devices(&device_name, &*device_strategy);
        self.show_pitchgrid_disconnected();
        self.ui_methods.set_devices_model(&self.device_names(&*device_strategy), &*device_strategy);
        self.show_no_port_connected(&*device_strategy);
        self.show_warning(device_strategy.msg_refreshed_reconnect());
    }

    /// Replaces the default Osc instance for testing.
    pub fn set_osc(&mut self, osc: Box<dyn IOsc>) { self.osc = osc; }

    /// Replaces the default Settings instance for testing.
    pub fn set_settings(&mut self, settings: Box<dyn ISettings>) { self.settings = settings; }

    /// Replaces the default Tuner instance for testing.
    pub fn set_tuner(&mut self, tuner: SharedTuner) {
        self.tuner = tuner;
    }

    /// Sets a thread-safe singleton Controller instance.
    pub fn set_controller(controller: SharedController) {
        *CONTROLLER.lock().unwrap() = Some(controller);
    }

    /// Returns a clone of the thread-safe singleton Controller instance.
    fn clone_controller() -> SharedController {
        Arc::clone(CONTROLLER.lock().unwrap().as_ref().unwrap())
    }

    /// Sets the root frequency override and sends it to the instrument,
    /// if the instrument and PitchGrid are both connected.
    /// We probably don't need a setting for this.
    /// The player should have to choose an override, if required, on startup.
    pub fn set_root_freq_override(&mut self, index: usize) {
        let send_tuning = Midi::midi().is_receiving_data()
            && Midi::midi().are_devices_connected()
            && Midi::midi().has_downloaded_init_data()
            && self.osc.is_pitchgrid_connected();
        if send_tuning {
            self.ui_methods.show_pitchgrid_status(
                UPDATING_ROOT_FREQ_OVERRIDE,
                MessageType::Info);
        }
        self.tuner.set_root_freq_override_note_no(index, send_tuning);
    }

    pub fn set_override_rounding_initial(&mut self, value: bool) {
        self.tuner.set_override_rounding_initial(value);
        self.settings.set_override_rounding_initial(value);
    }

    pub fn set_override_rounding_rate(&mut self, value: bool) {
        self.tuner.set_override_rounding_rate(value);
        self.settings.set_override_rounding_rate(value);
    }

    pub fn set_rounding_rate(&mut self, rate: u8) {
        self.tuner.set_rounding_rate(rate);
        self.settings.set_rounding_rate(rate);
    }

    pub fn set_osc_listening_port(&mut self, index: usize) {
        let osc_listening_port = Osc::listening_ports()[index];
        self.osc.set_listening_port(osc_listening_port);
        self.settings.set_osc_listening_port(osc_listening_port);
    }

    pub fn set_pitch_table(&mut self, index: usize) {
        let pitch_table = Tuner::pitch_tables()[index];
        self.tuner.set_pitch_table(pitch_table);
        self.settings.set_pitch_table(pitch_table);
    }

    fn on_data_download_completed(&mut self) {
        println!("Controller.on_data_download_completed");
        if Midi::midi().is_receiving_data()
                && Midi::midi().are_devices_connected()
                && !self.osc.is_running() {
            println!("Controller.on_data_download_completed: Starting OSC");
            self.start_osc();
            self.show_info(OPENING_PITCHGRID_CONNECTION);
        }
    }

    fn on_data_download_started(&mut self) {
        // println!("Controller.on_data_download_started");
        self.show_info(AWAITING_DATA_DOWNLOAD_COMPLETION);
    }

    fn on_devices_connected_changed(&mut self) {
        // println!("Controller.on_ports_connected_changed");
        if Midi::midi().are_devices_connected() {
            return;
        }
        // At least one MIDI port is not connected. Stop OSC if running.
        // println!("Controller.on_ports_connected_changed: Ports are not connected.");
        if self.osc.is_running() {
            // println!("Controller.on_ports_connected_changed: Stopping OSC");
            self.stop_osc_and_show_pitchgrid_status();
            self.show_warning(INSTRUMENT_DISCONNECTED);
        }
        self.ui_methods.show_pitchgrid_status(
            PITCHGRID_CONNECTION_CLOSED,
            MessageType::Warning);
    }

    fn on_new_preset_selected(&self) {
        println!("Controller.on_new_preset_selected");
        if self.tuner.send_current_preset_update() {
            println!("Controller.on_new_preset_selected: Updated");
            self.ui_methods.show_pitchgrid_status(
                NEW_PRESET_SELECTED,
                MessageType::Info);
        }
    }

    /// Started receiving data from the instrument.
    fn on_receiving_data_started(&mut self) {
        // println!("Controller.on_receiving_data_started");
        // The input device is connected, as we are receiving data from the instrument.
        // But the output device might not be, in which case we can't send data to the instrument
        // and should not overwrite the "Connect MIDI output device" warning message that should
        // already be displayed.
        if Midi::midi().are_devices_connected() {
            // println!("Controller.on_receiving_data_started: Waiting for data download to complete.");
            self.show_info(WAITING_FOR_DATA_DOWNLOAD);
        }
    }

    /// Stopped receiving data from the instrument.
    fn on_receiving_data_stopped(&mut self) {
        // println!("Controller.on_receiving_data_stopped");
        if self.osc.is_running() {
            println!("Controller.on_receiving_data_stopped: Stopping OSC");
            self.stop_osc_and_show_pitchgrid_status();
        }
        if Midi::midi().are_devices_connected() {
            // println!("Controller.on_receiving_data_stopped: Showing instrument not connected warning");
            self.show_warning(INSTRUMENT_NOT_CONNECTED);
        }
    }

    fn on_tuning_updated(&mut self) {
        println!("Controller.on_tuning_updated");
        // Stop waiting for tuning update to be confirmed.
        if self.is_awaiting_tuning_updated {
            let stopper_sender =
                self.await_tuning_updated_stopper_sender.take();
            if stopper_sender.is_some() {
                stopper_sender.unwrap().send(()).unwrap_or_else(|_| {
                    panic!("Controller.on_tuning_updated: Failed to send stop signal to \
                    await_tuning_updated");
                });
            }
            self.is_awaiting_tuning_updated = false;
        }
        self.tuner.on_tuning_updated();
        // If there's no tuning data, the displayed tuning data will be blanked.
        self.ui_methods.show_tuning(self.tuner.formatted_tuning_params(), self.tuner.is_root_freq_overridden());
        if !self.tuner.has_data() {
            // Could be tuning updated when an instrument preset is loaded
            // while PitchGrid is not connected.
            return;
        }
        println!("Controller.on_tuning_updated: Showing Instrument tuning updated");
        self.ui_methods.show_pitchgrid_status(INSTRUMENT_TUNING_UPDATED, MessageType::Info);
    }

    fn on_updating_tuning(&mut self) {
        println!("Controller.on_updating_tuning");
        let (stopper_sender, stopper_receiver) = mpsc::channel();
        self.await_tuning_updated_stopper_sender = Some(stopper_sender);
        self.is_awaiting_tuning_updated = true;
        rayon::spawn(move || {
            Self::await_tuning_updated(stopper_receiver);
        });
    }

    /// Shows an error message if tuning update is not confirmed within 2 seconds.
    /// The probable cause of the error is that MIDI output does not connect to one the editor's
    /// Ext All Data MIDI inputs.
    fn await_tuning_updated(stopper_receiver: mpsc::Receiver<()>) {
        // There's one scenario where this check is known not to behave as expected.
        // Editor MIDI:
        //     Input  LB1 (A)
        //     Output LB2 (A)
        // As we are using loopback endpoints, the following are the correct MIDI connections in
        // this application:
        //     Input  LB2 (B)
        //     Output LB1 (B)
        // But try the following MIDI connections in this application:
        //     Input  LB2 (B)
        //     Output LB2 (A)
        // In this scenario, this application's MIDI input is correct, but the incorrect output is
        // the same as the editor's output.
        // Expected behaviour:
        //     As our output is incorrect, the instrument tuning and the tuning shown in the
        //     editor should not be updated.
        //     We should not receive confirmation that the tuning has been updated.
        // Actual behavour:
        //     As with the expected behaviour, the instrument tuning and the tuning shown in the
        //     editor are not updated.
        //     However, we receive Grid message ch16 cc51 g, where g is our seleted pitch table
        //     number. We interpret this as confirmation that the tuning has been updated.
        //
        // Explanation
        //
        // Something like the following must be happening.
        // As Windows MIDI devices are currently shared with no way to make them exclusive,
        // there's nothing to stop us sending MIDI direct to the instrument, bypassing the editor.
        // But from the instrument's perspective, our tuning data looks like invalid data from
        // the editor, rather than a valid request from an external software component.
        // So the firmware does not implement the request.
        // As we request the current preset to be updated with the tuning with the same cc51 Grid
        // message, what we currently interpret as update confirmation is just our
        // request echoed back, which is expected. There is currently a firmware bug where,
        // for some presets, the confirmation message is not sent when the preset's tuning has been
        // updated. Our temporary workaround is to treat the echoed back request as confirmation.
        //
        // Pending fix
        //
        // Once the firmware bug is fixed, in Midi.on_message_received we can remove the workaround
        // and revert to interpreting not the first cc51 Grid message received, our request, but
        // the second as confirmation.
        // That should make the problem go away. I've tested it with a preset that still sends
        // the confirmation message even with the firmware bug.
        if let Ok(_) = stopper_receiver.recv_timeout(Duration::from_secs(2)) {
            // Sleep was interrupted: tuning has been updated.
            println!("Controller.await_tuning_updated: Tuning updated");
            return;
        }
        println!("Controller.await_tuning_updated: Tuning update not confirmed");
        let shared_controller = Self::clone_controller();
        let mut controller = shared_controller.lock().unwrap();
        controller.show_error(INSTRUMENT_TUNING_UPDATE_NOT_CONFIRMED);
        controller.is_awaiting_tuning_updated = false;
    }

    fn show_connected_device_name(
        &mut self, device_name: &str, device_strategy: &dyn DeviceStrategy) {
        let message_type = if device_name == DEVICE_NONE {
            MessageType::Warning
        } else {
            MessageType::Info
        };
        self.ui_methods.show_connected_device_name(device_name, message_type, device_strategy);
        // Don't save the port setting if the port is not connected.
        // The device, if any, stored in the settings file is the one that was connected last time.
        // The persisted device may be temporarily unavailable for selection, for example if a
        // USB-MIDI cable is not plugged in.
        // So the player needs to be able to close the application and reopen it later when the
        // device is available again and still have the same device automatically selected and
        // connected on startup.
        if device_name != DEVICE_NONE {
            device_strategy.set_device_setting(&mut *self.settings, device_name);
        }
    }

    fn show_error(&self, message: &str) {
        self.ui_methods.show_message(message, MessageType::Error);
    }

    fn show_info(&self, message: &str) {
        self.ui_methods.show_message(message, MessageType::Info);
    }

    fn show_no_port_connected(
        &mut self, device_strategy: &dyn DeviceStrategy) {
        self.show_connected_device_name(DEVICE_NONE, device_strategy);
    }
    
    fn show_pitchgrid_connected(&self) {
        // println!("Controller.show_pitchgrid_connected: Showing PitchGrid OSC is connected");
        self.ui_methods.show_pitchgrid_status(
            PITCHGRID_OSC_CONNECTED,
            MessageType::Info);
    }

    fn show_pitchgrid_disconnected(&self) {
        self.ui_methods.show_pitchgrid_status(
            DISCONNECTED_FROM_PITCHGRID,
            MessageType::Warning);
    }

    fn show_pitchgrid_not_connected(&self) {
        self.ui_methods.show_pitchgrid_status(
            PITCHGRID_NOT_CONNECTED,
            MessageType::Error);
    }

    fn show_warning(&self, message: &str) {
        // println!("Controller.show_warning: {}", message);
        self.ui_methods.show_message(message, MessageType::Warning);
    }

    fn start_osc(&mut self) {
        self.osc.start(Self::clone_controller());
    }

    fn stop_osc_and_instrument_connection_monitor(&mut self) {
        // println!("Controller.stop_osc_and_instrument_connection_monitor");
        Midi::midi().stop_instrument_connection_monitor();
        self.stop_osc_and_show_pitchgrid_status();
        // println!("Controller.stop_osc_and_instrument_connection_monitor: Done");
    }

    /// Stops PitchGrid OSC, blanks the displayed tuning and shows a PitchGrid status message.
    /// When the application is reconnected to the instrument, OSC will be restarted, which will
    /// force PitchGrid to send the latest tuning.
    fn stop_osc_and_show_pitchgrid_status(&self) {
        println!("Controller.stop_osc_and_show_pitchgrid_status");
        self.osc.stop();
        self.remove_data();
        if !Midi::midi().are_devices_connected() {
            self.ui_methods.show_pitchgrid_status(
                CANNOT_UPDATE_TUNING_CONNECT,
                MessageType::Error);
        } else if !Midi::midi().is_receiving_data() {
            self.ui_methods.show_pitchgrid_status(
                CANNOT_UPDATE_TUNING_LOST,
                MessageType::Error);
        }
    }

    /// Blanks the displayed tuning and removes tuning data from the tuner.
    fn remove_data(&self) {
        println!("Controller.remove_data");
        self.tuner.remove_data();
        // As we've removed tuning data, the displayed tuning data will be blanked.
        self.ui_methods.show_tuning(self.tuner.formatted_tuning_params(), self.tuner.is_root_freq_overridden());
    }
}

impl MidiCallbacks for Mutex<Controller> {
    fn on_download_completed(&self) {
        self.lock().unwrap().on_data_download_completed();
    }

    fn on_download_started(&self) {
        self.lock().unwrap().on_data_download_started();
    }

    fn on_new_preset_selected(&self) {
        self.lock().unwrap().on_new_preset_selected();
    }

    fn on_devices_connected_changed(&self) {
        self.lock().unwrap().on_devices_connected_changed();
    }

    fn on_receiving_data_started(&self) {
        self.lock().unwrap().on_receiving_data_started();
    }

    fn on_receiving_data_stopped(&self) {
        self.lock().unwrap().on_receiving_data_stopped();
    }

    fn on_tuning_updated(&self) {
        self.lock().unwrap().on_tuning_updated();
    }

    fn on_updating_tuning(&self) {
        self.lock().unwrap().on_updating_tuning();
    }
}

impl OscCallbacks for Mutex<Controller> {
    fn on_pitchgrid_connected_changed(&self) {
        // println!("OscCallbacks for Mutex<Controller>.on_pitchgrid_connected_changed");
        let controller = self.lock().unwrap();
        controller.on_pitchgrid_connected_changed();
    }

    fn on_tuning_received(&self, tuning_params: TuningParams) {
        // println!("OscCallbacks for Mutex<Controller>.on_tuning_received");
        let controller = self.lock().unwrap();
        controller.on_tuning_received(tuning_params);
    }
}

impl OscCallbacks for Controller {
    fn on_pitchgrid_connected_changed(&self) {
        // println!("Controller.on_pitchgrid_connected_changed");
        if self.osc.is_pitchgrid_connected() {
            println!("Controller.on_pitchgrid_connected_changed: Showing tuning");
            self.ui_methods.show_tuning(self.tuner.formatted_tuning_params(), self.tuner.is_root_freq_overridden());
            // println!("Controller.on_pitchgrid_connected_changed: Showing PitchGrid is connected");
            self.show_pitchgrid_connected();
            // println!("Controller.on_pitchgrid_connected_changed: PitchGrid and instrument are connected");
            self.show_info(PITCHGRID_AND_INSTRUMENT_CONNECTED);
        } else {
            println!("Controller.on_pitchgrid_connected_changed: PitchGrid is not connected");
            self.remove_data();
            self.show_pitchgrid_not_connected();
            self.show_warning(AWAITING_PITCHGRID_CONNECTION);
        }
    }

    fn on_tuning_received(&self, tuning_params: TuningParams) {
        // println!("Controller.on_tuning_received");
        // println!(
        //     "Controller.on_tuning_received: depth = {}; mode = {}; root_freq = {}; stretch = {}; \
        //     skew = {}; mode_offset = {}; steps = {}",
        //     depth, mode, root_freq, stretch, skew, mode_offset, steps);
        if Midi::midi().are_devices_connected() && Midi::midi().is_receiving_data() {
            println!("Controller.on_tuning_received: Showing Updating instrument tuning");
            self.ui_methods.show_pitchgrid_status(UPDATING_INSTRUMENT_TUNING, MessageType::Info);
            println!("Controller.on_tuning_received: Updating instrument tuning");
            self.tuner.on_tuning_received(tuning_params);
        } else {
            self.stop_osc_and_show_pitchgrid_status();
        }
    }
}

pub const AWAITING_DATA_DOWNLOAD_COMPLETION: &str = "Awaiting completion of data download from instrument...";
pub const AWAITING_PITCHGRID_CONNECTION: &str = "Awaiting PitchGrid connection...";
pub const CANNOT_UPDATE_TUNING_CONNECT: &str = "Cannot update tuning. Connect instrument input/output.";
pub const CANNOT_UPDATE_TUNING_LOST: &str = "Cannot update tuning. Instrument connection lost.";
pub const CHECKING_INSTRUMENT_CONNECTION: &str = "Checking instrument connection...";
pub const DEVICE_NONE: &str = "[None]";
pub const DISCONNECTED_FROM_PITCHGRID: &str = "Disconnected from PitchGrid because MIDI is not connected";
pub const INSTRUMENT_DISCONNECTED: &str = "Instrument is disconnected; closed PitchGrid connection.";
pub const INSTRUMENT_NOT_CONNECTED: &str = "The instrument is not connected. Waiting for the editor to be \
        opened with this application and the instrument connected to it...";
pub const INSTRUMENT_TUNING_UPDATE_NOT_CONFIRMED: &str = "Instrument tuning update has not been \
    confirmed. Ensure that MIDI output is connected to the editor.";
pub const INSTRUMENT_TUNING_UPDATED: &str = "Instrument tuning updated";
pub const NEW_PRESET_SELECTED: &str = "New instrument preset selected. Resent tuning...";
pub const OPENING_PITCHGRID_CONNECTION: &str = "Opening PitchGrid connection...";
pub const PITCHGRID_AND_INSTRUMENT_CONNECTED: &str = "PitchGrid and instrument are connected";
pub const PITCHGRID_CONNECTION_CLOSED: &str = "PitchGrid connection closed while instrument disconnected";
pub const PITCHGRID_NOT_CONNECTED: &str = "PitchGrid is not connected. OSC must be enabled in PitchGrid.";
pub const PITCHGRID_OSC_CONNECTED: &str = "PitchGrid OSC is connected";
pub const UPDATING_INSTRUMENT_TUNING: &str = "Updating instrument tuning";
pub const UPDATING_ROOT_FREQ_OVERRIDE: &str = "Updating root frequency override...";
pub const WAITING_FOR_DATA_DOWNLOAD: &str =
    "Waiting (maximum 6 seconds) for possible initial data download from instrument...";

type SharedController = Arc<Mutex<Controller>>;

static CONTROLLER: Mutex<Option<SharedController>> = Mutex::new(None);
