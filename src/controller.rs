use std::error::Error;
use std::sync::{Arc, Mutex, OnceLock};
use crate::global::{MessageType, PortType, SharedMidi};
use crate::osc::{Osc, OscCallbacks};
use crate::port_strategy::{
    InputStrategy, OutputStrategy, PortStrategy};
use crate::settings::Settings;
use crate::{midi_static, tuner};
use crate::tuning_params::TuningParams;

/// This is the main controller in the Model-View-Controller (MVC) pattern.
/// PortStrategy contains both view and controller methods.
/// The Slint UI, main.rs and UiMethods are the remainder of the view.
/// Everything else is the model.
pub struct Controller {
    callbacks: Box<dyn ControllerCallbacks>,
    has_restart_been_requested: bool,
    osc: Osc,
    settings: Settings,
}

impl Controller {
    pub fn new(callbacks: Box<dyn ControllerCallbacks>) -> Self {
        Self {
            callbacks,
            has_restart_been_requested: false,
            osc: Osc::new(),
            settings: Settings::new(),
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
                main_window_x = self.settings.main_window_x;
                main_window_y = self.settings.main_window_y;
                input_device_name = self.settings.midi_input_device.clone();
                output_device_name = self.settings.midi_output_device.clone();
                osc_listening_port = {
                    if Osc::listening_ports().contains(&self.settings.osc_listening_port) {
                        self.settings.osc_listening_port
                    } else {
                        Osc::default_listening_port()
                    }
                };
                pitch_table = {
                    if tuner::pitch_tables().contains(&self.settings.pitch_table) {
                        self.settings.pitch_table
                    } else {
                        tuner::default_pitch_table()
                    }
                };
                override_rounding_initial = self.settings.override_rounding_initial;
                override_rounding_rate = self.settings.override_rounding_rate;
                rounding_rate = self.settings.rounding_rate;
            }
            Err(err) => {
                self.show_error(&err.to_string());
                return;
            }
        }
        // println!("Controller.init: Getting midi");
        self.callbacks.set_main_window_position(main_window_x, main_window_y);
        let midi = midi_static::midi_clone();
        let mut midi_guard = midi.lock().unwrap();
        if let Err(err) = midi_guard.init(
            &input_device_name, &output_device_name) {
            self.show_error(&err.to_string());
            return;
        }
        // println!("Controller.init: Adding download completed callback");
        midi_guard.add_init_download_completed_callback(Box::new(|| {
            Self::clone_controller().lock().unwrap().on_init_data_download_completed();
        }));
        midi_guard.add_init_download_started_callback(Box::new(|| {
            Self::clone_controller().lock().unwrap().on_init_data_download_started();
        }));
        midi_guard.add_ports_connected_changed_callback(Box::new(|| {
            Self::clone_controller().lock().unwrap().on_ports_connected_changed();
        }));
        // println!("Controller.init: Adding selected preset loaded callback");
        midi_guard.add_new_preset_selected_callback(Box::new(|| {
            Self::clone_controller().lock().unwrap().on_new_preset_selected();
        }));
        midi_guard.add_receiving_data_started_callback(Box::new(|| {
            Self::clone_controller().lock().unwrap().on_receiving_data_started_callback();
        }));
        midi_guard.add_receiving_data_stopped_callback(Box::new(|| {
            Self::clone_controller().lock().unwrap().on_receiving_data_stopped_callback();
        }));
        midi_guard.add_tuning_updated_callback(Box::new(|| {
            Self::clone_controller().lock().unwrap().on_tuning_updated();
        }));
        drop(midi_guard); // Release MIDI lock before calling device_names which needs to acquire it
        let input_strategy = InputStrategy::new();
        let output_strategy = OutputStrategy::new();
        // println!("Controller.init: Getting input port names");
        let input_device_names = self.device_names(&input_strategy);
        // println!("Controller.init: Got {} input port names", input_device_names.len());
        // println!("Controller.init: About to call callbacks.set_devices_model");
        self.callbacks.set_devices_model(&input_device_names, &input_strategy);
        // println!("Controller.init: Called callbacks.set_devices_model");
        // println!("Controller.init: Setting output ports model");
        self.callbacks.set_devices_model(&self.device_names(&output_strategy), &output_strategy);
        // println!("Controller.init: Connecting initial ports");
        self.connect_initial_port(&output_strategy);
        // Don't start listening to MIDI until we are able to send MIDI.
        if midi_static::is_output_port_connected() {
            // println!("Controller.init: Connecting input port");
            self.connect_initial_port(&input_strategy);
        }
        self.osc.set_listening_port(osc_listening_port);
        self.callbacks.set_selected_osc_listening_port_index(Osc::listening_port_index() as i32);
        // println!("Controller.init: Configuring tuner");
        tuner::init(pitch_table);
        self.callbacks.set_selected_pitch_table_index(tuner::pitch_table_index() as i32);
        tuner::set_override_rounding_initial(override_rounding_initial);
        tuner::set_override_rounding_rate(override_rounding_rate);
        tuner::set_rounding_rate(rounding_rate);
        self.callbacks.set_override_rounding_initial(override_rounding_initial);
        self.callbacks.set_override_rounding_rate(override_rounding_rate);
        self.callbacks.set_rounding_rate(rounding_rate);
        if midi_static::are_ports_connected() {
            // println!("Controller.init: Showing Checking instrument connection");
            self.show_info(CHECKING_INSTRUMENT_CONNECTION);
            // println!("Controller.init: Starting instrument connection monitor");
            midi_static::start_instru_connection_monitor();
        }
        // println!("Controller.init: Done");
    }

    #[allow(clippy::unwrap_used)]
    pub fn close(&mut self, main_window_x: i32, main_window_y: i32) -> Result<(), Box<dyn Error>> {
        midi_static::close();
        self.osc.stop();
        self.settings.main_window_x = main_window_x;
        self.settings.main_window_y = main_window_y;
        if let Err(err) = self.settings.write_to_file() {
            self.show_error(&err.to_string());
            return Err(err)
        };
        Ok(())
    }

    fn connect_initial_port(&mut self, port_strategy: &dyn PortStrategy) {
        // println!("Controller.connect_initial_port: {:?}", port_strategy.port_type());
        let midi = midi_static::midi_clone();
        let maybe_index = {
            let midi_guard = midi.lock().unwrap();
            midi_guard.io(port_strategy).device().as_ref()
                .map(|port| port.index())
        };
        if let Some(index) = maybe_index {
            // println!("Controller.connect_initial_port: Setting selected port index to {}", index);
            self.callbacks.set_selected_port_index(index, port_strategy);
            self.connect_selected_port(&midi, port_strategy);
        } else {
            self.show_no_port_connected(port_strategy);
            self.show_warning(port_strategy.msg_connect());
            self.callbacks.focus_port(port_strategy);
        }
    }

    pub fn connect_port(&mut self, port_strategy: &dyn PortStrategy) {
        // println!("Controller.connect_port");
        let midi = midi_static::midi_clone();
        let port_strategy = port_strategy.clone_box();
        // println!("Controller.connect_port: Stopping OSC and instrument connection monitor");
        self.stop_osc_and_instru_connection_monitor();
        // println!("Controller.connect_port: Showing PitchGrid disconnected");
        self.show_pitchgrid_disconnected();
        // println!("Controller.connect_port: Connecting selected port");
        self.connect_selected_port(&midi, &*port_strategy);
        // println!("Controller.connect_port: Getting port");
        let device_name_opt: Option<String> = midi.lock().unwrap()
            .io(&*port_strategy)
            .device()
            .map(|p| p.name().to_string());
        // println!("Controller.connect_port: Got port");
        if let Some(device_name) = device_name_opt {
            self.show_info(port_strategy.msg_connected(&device_name));
            if midi_static::are_ports_connected() {
                // println!("Controller.connect_port: Showing Restart this application");
                self.show_warning(RESTART_APPLICATION);
                self.has_restart_been_requested = true;
            } else {
                let other_port_strategy:Box<dyn PortStrategy> = match port_strategy.port_type() {
                    PortType::Input => Box::new(OutputStrategy::new()),
                    PortType::Output => Box::new(InputStrategy::new()),
                };
                self.show_warning(other_port_strategy.msg_connect());
            }
        }
        // println!("Controller.connect_port: Done");
    }

    fn connect_selected_port(&mut self, midi: &SharedMidi, port_strategy: &dyn PortStrategy) {
        // println!("Controller.connect_selected_port: {:?}", port_strategy.port_type());
        let selected = self.callbacks.get_selected_port_index(port_strategy);
        let index: usize = match usize::try_from(selected) {
            Ok(i) => i,
            Err(_) => {
                // A port has not been selected. That's impossible with the UI as it is.
                self.show_no_port_connected(port_strategy);
                self.show_error(port_strategy.msg_not_selected());
                return;
            }
        };
        // println!("Controller.connect_selected_port: Selected port index = {}", index);
        let ui_action: Result<String, String> = {
            // println!("Controller.connect_selected_port: Getting midi_guard.");
            let mut midi_guard = midi.lock().unwrap();
            // println!("Controller.connect_selected_port: Got midi_guard.");
            let Some(name) = midi_guard.io(port_strategy).device_names().get(index).cloned()
            else {
                return;
            };
            match midi_guard.connect_port(index, port_strategy) {
                Ok(()) => Ok(name),
                Err(err) => Err(err.to_string()),
            }
        };
        match ui_action {
            Ok(name) => {
                self.show_connected_device_name(&name, port_strategy);
            }
            Err(message) => {
                self.show_no_port_connected(port_strategy);
                self.show_error(&message);
            }
        }
    }

    fn device_names(&self, port_strategy: &dyn PortStrategy) -> Vec<String> {
        let midi = midi_static::midi_clone();
        // println!("Controller.device_names: Got midi");
        let midi_guard = midi.lock().unwrap();
        midi_guard.io(port_strategy).device_names()
    }

    pub fn refresh_devices(&mut self, port_strategy: &dyn PortStrategy) {
        let midi = midi_static::midi_clone();
        let port_strategy = port_strategy.clone_box();
        self.stop_osc_and_instru_connection_monitor();
        let device_name = port_strategy.port_setting(&self.settings).to_string();
        if let Err(err) = midi.lock().unwrap().refresh_devices(
            &device_name, &*port_strategy) {
            self.show_error(&err.to_string());
            return;
        }
        self.show_pitchgrid_disconnected();
        self.callbacks.set_devices_model(&self.device_names(&*port_strategy), &*port_strategy);
        self.show_no_port_connected(&*port_strategy);
        self.show_warning(port_strategy.msg_refreshed_reconnect());
    }

    /// Sets a thread-safe singleton Controller instance.
    pub fn set_controller(controller: SharedController) {
        CONTROLLER.set(controller).ok();
    }

    /// Returns a clone of the thread-safe singleton Controller instance.
    fn clone_controller() -> SharedController {
        let controller = CONTROLLER.get().unwrap();
        Arc::clone(controller)
    }

    /// Sets the root frequency override and sends it to the instrument, if connected.
    /// We probably don't need a setting for this.
    /// The player should have to choose an override, if required, on startup.
    pub fn set_root_freq_override(&mut self, index: usize) {
        let send_tuning = midi_static::is_receiving_data()
            && midi_static::are_ports_connected()
            && midi_static::has_downloaded_init_data()
            && self.osc.is_pitchgrid_connected();
        if send_tuning {
            self.callbacks.show_pitchgrid_status(
                UPDATING_ROOT_FREQ_OVERRIDE,
                MessageType::Info);
        }
        tuner::set_root_freq_override_note_no(index, send_tuning);
    }

    pub fn set_override_rounding_initial(&mut self, value: bool) {
        tuner::set_override_rounding_initial(value);
        self.settings.override_rounding_initial = value;
    }

    pub fn set_override_rounding_rate(&mut self, value: bool) {
        tuner::set_override_rounding_rate(value);
        self.settings.override_rounding_rate = value;
    }

    pub fn set_rounding_rate(&mut self, rate: u8) {
        tuner::set_rounding_rate(rate);
        self.settings.rounding_rate = rate;
    }

    pub fn set_osc_listening_port(&mut self, index: usize) {
        let osc_listening_port = Osc::listening_ports()[index];
        self.osc.set_listening_port(osc_listening_port);
        self.settings.osc_listening_port = osc_listening_port;
    }

    pub fn set_pitch_table(&mut self, index: usize) {
        let pitch_table = tuner::pitch_tables()[index];
        tuner::set_pitch_table(pitch_table);
        self.settings.pitch_table = pitch_table;
    }

    fn on_init_data_download_completed(&mut self) {
        // println!("Controller.on_init_data_download_completed");
        if midi_static::is_receiving_data()
                && midi_static::are_ports_connected()
                && !self.osc.is_running() {
            self.start_osc();
            self.show_info(OPENING_PITCHGRID_CONNECTION);
        }
    }

    fn on_init_data_download_started(&mut self) {
        // println!("Controller.on_init_data_download_started");
        self.show_info(AWAITING_DATA_DOWNLOAD_COMPLETION);
    }

    fn on_ports_connected_changed(&mut self) {
        // println!("Controller.on_ports_connected_changed");
        if midi_static::are_ports_connected() {
            return;
        }
        // Instrument is not connected. Stop OSC if running.
        // println!("Controller.on_instru_connected_changed: Instrument is not connected.");
        if self.osc.is_running() {
            // println!("Controller.on_instru_connected_changed: Stopping OSC");
            self.osc.stop();
            self.show_warning(INSTRUMENT_DISCONNECTED);
        }
        self.callbacks.show_pitchgrid_status(
            PITCHGRID_CONNECTION_CLOSED,
            MessageType::Warning);
    }

    fn on_new_preset_selected(&self) {
        // println!("Controller.on_new_preset_selected");
        if tuner::resend_tuning() && self.osc.is_pitchgrid_connected() {
            // println!("Controller.on_new_preset_selected: Resent");
            self.callbacks.show_pitchgrid_status(
                NEW_PRESET_SELECTED,
                MessageType::Info);
        }
    }

    fn on_receiving_data_started_callback(&mut self) {
        // The input port is connected, as we are receiving data from the instrument.
        // But the output port might not be, in which case we can't send data to the instrument
        // and should not overwrite the "Connect MIDI output port" warning message that should
        // already be displayed.
        if midi_static::are_ports_connected() {
            // println!("Controller.on_receiving_data_started_callback: Waiting for data download to complete.");
            self.show_info(WAITING_FOR_DATA_DOWNLOAD);
        }
    }

    fn on_receiving_data_stopped_callback(&mut self) {
        if self.osc.is_running() {
            self.stop_osc_and_show_message();
        }
        if midi_static::are_ports_connected() {
            self.show_warning(INSTRUMENT_NOT_CONNECTED);
        }
    }

    fn on_tuning_updated(&self) {
        // println!("Controller.on_tuning_updated: Showing tuning");
        self.callbacks.show_tuning(tuner::is_root_freq_overridden());
        // println!("Controller.on_tuning_updated: Showing Instrument tuning updated");
        self.callbacks.show_pitchgrid_status(INSTRUMENT_TUNING_UPDATED, MessageType::Info);
    }

    fn show_connected_device_name(
        &mut self, device_name: &str, port_strategy: &dyn PortStrategy) {
        let message_type = if device_name == PORT_NONE {
            MessageType::Warning
        } else {
            MessageType::Info
        };
        self.callbacks.show_connected_device_name(device_name, message_type, port_strategy);
        // Don't save the port setting if the port is not connected.
        // The device, if any, stored in the settings file is the one that was connected last time.
        // The persisted device may be temporarily unavailable for selection, for example if a
        // USB-MIDI cable is not plugged in.
        // So the player needs to be able to close the application and reopen it later when the
        // device is available again and still have the same device automatically selected and
        // connected on startup.
        if device_name != PORT_NONE {
            port_strategy.set_port_setting(&mut self.settings, device_name);
        }
    }

    fn show_error(&self, message: &str) {
        self.callbacks.show_message(message, MessageType::Error);
    }

    fn show_info(&self, message: &str) {
        self.callbacks.show_message(message, MessageType::Info);
    }

    fn show_no_port_connected(
        &mut self, port_strategy: &dyn PortStrategy) {
        self.show_connected_device_name(PORT_NONE, port_strategy);
    }
    
    fn show_pitchgrid_connected(&self) {
        // println!("Controller.show_pitchgrid_connected: Showing PitchGrid OSC is connected");
        self.callbacks.show_pitchgrid_status(
            PITCHGRID_OSC_CONNECTED,
            MessageType::Info);
    }

    fn show_pitchgrid_disconnected(&self) {
        self.callbacks.show_pitchgrid_status(
            DISCONNECTED_FROM_PITCHGRID,
            MessageType::Warning);
    }

    fn show_pitchgrid_not_connected(&self) {
        self.callbacks.show_pitchgrid_status(
            PITCHGRID_NOT_CONNECTED,
            MessageType::Error);
    }

    fn show_warning(&self, message: &str) {
        // println!("Controller.show_warning: {}", message);
        self.callbacks.show_message(message, MessageType::Warning);
    }

    fn start_osc(&mut self) {
        self.osc.start(Self::clone_controller());
    }

    fn stop_osc_and_instru_connection_monitor(&mut self) {
        midi_static::stop_instru_connection_monitor();
        self.osc.stop();
        // println!("Controller.stop_osc_and_instru_connection_monitor: Done");
    }

    /// Stops PitchGrid OSC and shows a message.
    /// When the application is reconnected to the instrument, OSC will be restarted, which will
    /// force PitchGrid to send the latest tuning.
    fn stop_osc_and_show_message(&self) {
        self.osc.stop();
        if !midi_static::are_ports_connected() {
            self.callbacks.show_pitchgrid_status(
                CANNOT_UPDATE_TUNING_CONNECT,
                MessageType::Error);
        } else if !midi_static::is_receiving_data() {
            self.callbacks.show_pitchgrid_status(
                CANNOT_UPDATE_TUNING_LOST,
                MessageType::Error);
        }
    }
}

impl OscCallbacks for Mutex<Controller> {
    fn on_osc_pitchgrid_connected_changed(&self) {
        // println!("OscCallbacks for Mutex<Controller>.on_osc_pitchgrid_connected_changed");
        let controller = self.lock().unwrap();
        controller.on_osc_pitchgrid_connected_changed();
    }

    fn on_osc_tuning_received(&self, tuning_params: TuningParams) {
        // println!("OscCallbacks for Mutex<Controller>.on_osc_tuning_received");
        let controller = self.lock().unwrap();
        controller.on_osc_tuning_received(tuning_params);
    }
}

impl OscCallbacks for Controller {
    fn on_osc_pitchgrid_connected_changed(&self) {
        // println!("Controller.on_osc_pitchgrid_connected_changed");
        if self.osc.is_pitchgrid_connected() {
            // println!("Controller.on_osc_pitchgrid_connected_changed: Showing PitchGrid is connected");
            self.show_pitchgrid_connected();
            // println!("Controller.on_osc_pitchgrid_connected_changed: PitchGrid and instrument are connected");
            self.show_info(PITCHGRID_AND_INSTRUMENT_CONNECTED);
        } else {
            // println!("Controller.on_osc_pitchgrid_connected_changed: PitchGrid is not connected");
            self.show_pitchgrid_not_connected();
            self.show_warning(AWAITING_PITCHGRID_CONNECTION);
        }
    }

    fn on_osc_tuning_received(&self, tuning_params: TuningParams) {
        // println!("Controller.on_osc_tuning_received");
        // println!(
        //     "Controller.on_osc_tuning_received: depth = {}; mode = {}; root_freq = {}; stretch = {}; \
        //     skew = {}; mode_offset = {}; steps = {}",
        //     depth, mode, root_freq, stretch, skew, mode_offset, steps);
        if midi_static::are_ports_connected() && midi_static::is_receiving_data() {
            // println!("Controller.on_osc_tuning_received: Showing Updating instrument tuning");
            self.callbacks.show_pitchgrid_status(UPDATING_INSTRUMENT_TUNING, MessageType::Info);
            tuner::on_tuning_received(tuning_params);
        } else {
            self.stop_osc_and_show_message();
        }
    }
}

const AWAITING_DATA_DOWNLOAD_COMPLETION: &str = "Awaiting completion of data download from instrument...";
const AWAITING_PITCHGRID_CONNECTION: &str = "Awaiting PitchGrid connection...";
const CANNOT_UPDATE_TUNING_CONNECT: &str = "Cannot updating tuning. Connect instrument input/output.";
const CANNOT_UPDATE_TUNING_LOST: &str = "Cannot update tuning. Instrument connection lost.";
const CHECKING_INSTRUMENT_CONNECTION: &str = "Checking instrument connection...";
const DISCONNECTED_FROM_PITCHGRID: &str = "Disconnected from PitchGrid because MIDI is not connected";
const INSTRUMENT_DISCONNECTED: &str = "Instrument is disconnected; closed PitchGrid connection.";
const INSTRUMENT_NOT_CONNECTED: &str = "The instrument is not connected. Waiting for the editor to be \
        opened with this application and the instrument connected to it...";
const INSTRUMENT_TUNING_UPDATED: &str = "Instrument tuning updated";
const NEW_PRESET_SELECTED: &str = "New instrument preset selected. Resent tuning...";
const OPENING_PITCHGRID_CONNECTION: &str = "Opening PitchGrid connection...";
const PITCHGRID_AND_INSTRUMENT_CONNECTED: &str = "PitchGrid and instrument are connected";
const PITCHGRID_CONNECTION_CLOSED: &str = "PitchGrid connection closed while instrument disconnected";
const PITCHGRID_NOT_CONNECTED: &str = "PitchGrid is not connected. OSC must be enabled in Pitchgrid.";
const PITCHGRID_OSC_CONNECTED: &str = "PitchGrid OSC is connected";
const PORT_NONE: &str = "[None]";
const RESTART_APPLICATION: &str = "Restart this application to connect to PitchGrid";
const UPDATING_INSTRUMENT_TUNING: &str = "Updating instrument tuning";
const UPDATING_ROOT_FREQ_OVERRIDE: &str = "Updating root frequency override...";
const WAITING_FOR_DATA_DOWNLOAD: &str =
    "Waiting (maximum 6 seconds) for possible initial data download from instrument...";

pub trait ControllerCallbacks: Send + Sync {
    fn focus_port(&self, port_strategy: &dyn PortStrategy);
    fn get_selected_port_index(&self, port_strategy: &dyn PortStrategy) -> usize;
    fn set_selected_port_index(&self, index: usize, port_strategy: &dyn PortStrategy);
    fn set_devices_model(&self, device_names: &Vec<String>, port_strategy: &dyn PortStrategy);
    fn show_connected_device_name(&self, name: &str, msg_type: MessageType, port_strategy: &dyn PortStrategy);
    fn show_message(&self, msg: &str, msg_type: MessageType);
    fn show_pitchgrid_status(&self, status: &str, msg_type: MessageType);
    fn show_tuning(&self, is_overriding_root_freq: bool);
    fn set_main_window_position(&self, x: i32, y: i32);
    fn set_override_rounding_initial(&self, value: bool);
    fn set_override_rounding_rate(&self, value: bool);
    fn set_rounding_rate(&self, rate: u8);
    fn set_selected_osc_listening_port_index(&self, index: i32);
    fn set_selected_pitch_table_index(&self, index: i32);
}

type SharedController = Arc<Mutex<Controller>>;

static CONTROLLER: OnceLock<SharedController> = OnceLock::new();
