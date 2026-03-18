use std::cmp::max;
use std::error::Error;
use std::sync::{Arc, Mutex, OnceLock};
use lazy_static::lazy_static;
use crate::midi::Midi;
use crate::global::{SharedMidi, MessageType};
use crate::osc::{Osc, OscCallbacks};
use crate::port_strategy::{
    InputStrategy, OutputStrategy, PortStrategy};
use crate::settings::Settings;
use crate::tuner;

/// This is the main controller in the Model-View-Controller (MVC) pattern.
/// PortStrategy contains both view and controller methods.
/// The Slint UI, main.rs and UiMethods are the remainder of the view.
/// Everything else is the model.
pub struct Controller {
    callbacks: Box<dyn ControllerCallbacks>,
    settings: Settings,
}

impl Controller {
    pub fn new(callbacks: Box<dyn ControllerCallbacks>) -> Self {
        Self {
            callbacks,
            settings: Settings::new(),
        }
    }

    pub fn init(&mut self) {
        // println!("Controller.init");
        let pitch_table_no: u8;
        let input_device_name: String;
        let output_device_name: String;
        // println!("Controller.init: Reading settings");
        match self.settings.read_from_file() {
            Ok(_) => {
                input_device_name = self.settings.midi_input_device.clone();
                output_device_name = self.settings.midi_output_device.clone();
                pitch_table_no = max(tuner::default_pitch_table_no(), self.settings.pitch_table);
            }
            Err(err) => {
                self.show_error(&err.to_string());
                return;
            }
        }
        // println!("Controller.init: Getting midi");
        let midi = self.midi_static_clone();
        let mut midi_guard = midi.lock().unwrap();
        if let Err(err) = midi_guard.init(
            &input_device_name, &output_device_name) {
            self.show_error(&err.to_string());
            return;
        }
        // println!("Controller.init: Adding callbacks");
        midi_guard.add_config_received_callback(Box::new(|| {
            if let Some(controller) = CONTROLLER.get() {
                controller.lock().unwrap().on_config_received();
            }
        }));
        println!("Controller.init: Adding download completed callback");
        midi_guard.add_editor_data_download_completed_callback(Box::new(|| {
            if let Some(controller) = CONTROLLER.get() {
                controller.lock().unwrap().on_editor_data_download_completed();
            }
        }));
        midi_guard.add_instru_connected_changed_callback(Box::new(|| {
            if let Some(controller) = CONTROLLER.get() {
                controller.lock().unwrap().on_instru_connected_changed();
            }
        }));
        midi_guard.add_tuning_updated_callback(Box::new(|| {
            if let Some(controller) = CONTROLLER.get() {
                controller.lock().unwrap().on_tuning_updated();
            }
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
        self.connect_initial_port(&input_strategy);
        self.connect_initial_port(&output_strategy);
        // println!("Controller.init: Configuring tuner");
        tuner::set_midi(midi.clone());
        tuner::set_pitch_table_no(pitch_table_no);
        self.callbacks.set_selected_pitch_table_index(tuner::pitch_table_index() as i32);
        let mut midi_guard = midi.lock().unwrap();
        if midi_guard.are_ports_connected() {
            // println!("Controller.init: Showing Checking instrument connection");
            self.show_info("Checking instrument connection...");
            midi_guard.start_instru_connection_monitor();
        }
        // println!("Controller.init: Done");
    }

    #[allow(clippy::unwrap_used)]
    pub fn close(&mut self) -> Result<(), Box<dyn Error>> {
        let midi = self.midi_static_clone();
        // There is no way to avoid this unwrap, but there seems not to be a way of stopping
        // RustRover from suggesting it should be replaced with '?'.
        let mut midi_guard = midi.lock().unwrap();
        midi_guard.close();
        drop(midi_guard);
        drop(midi);

        let osc = self.osc_static_clone();
        let mut osc_guard = osc.lock().unwrap();
        osc_guard.stop();
        drop(osc_guard);
        drop(osc);

        if let Err(err) = self.settings.write_to_file() {
            self.show_error(&err.to_string());
            return Err(err)
        };
        Ok(())
    }

    fn connect_initial_port(&mut self, port_strategy: &dyn PortStrategy) {
        // println!("Controller.connect_initial_port: {:?}", port_strategy.port_type());
        let midi = self.midi_static_clone();
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
        let midi = self.midi_static_clone();
        let osc = self.osc_static_clone();
        let port_strategy = port_strategy.clone_box();
        // println!("Controller.connect_port: Stopping OSC and instrument connection monitor");
        self.stop_osc_and_instru_connection_monitor(&midi, &osc);
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
            // println!("Controller.connect_port: Getting midi_guard");
            let midi_guard = midi.lock().unwrap();
            // println!("Controller.connect_port: Got midi_guard");
            if midi_guard.are_ports_connected() {
                self.show_warning("Restart this application to connect to PitchGrid");
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
        let midi = self.midi_static_clone();
        // println!("Controller.device_names: Got midi");
        let midi_guard = midi.lock().unwrap();
        midi_guard.io(port_strategy).device_names()
    }

    pub fn refresh_devices(&mut self, port_strategy: &dyn PortStrategy) {
        let midi = self.midi_static_clone();
        let osc = self.osc_static_clone();
        let port_strategy = port_strategy.clone_box();
        self.stop_osc_and_instru_connection_monitor(&midi, &osc);
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

    /// Sets a thread-safe singleton controller instance.
    pub fn set_controller(controller: SharedController) {
        CONTROLLER.set(controller).ok();
    }

    pub fn set_root_freq_override(&mut self, index: usize) {
        tuner::set_root_freq_override(index);
        // We probably don't need a setting for this.
        // The player should have to choose an override, if required, on startup.
    }

    pub fn set_pitch_table_no(&mut self, index: usize) {
        let pitch_table_no = tuner::pitch_table_nos()[index];
        tuner::set_pitch_table_no(pitch_table_no);
        self.settings.pitch_table = pitch_table_no;
    }

    fn midi_static_clone(&self) -> SharedMidi {
        Arc::clone(&MIDI)
    }

    fn on_config_received(&self) {
        println!("Controller.on_config_received");
        self.show_info("Got instrument config. Opening PitchGrid connection.");
        let osc = self.osc_static_clone();
        let mut osc_guard = osc.lock().unwrap();
        if let Some(controller) = CONTROLLER.get() {
            osc_guard.start(controller.clone());
        }
    }

    fn on_editor_data_download_completed(&self) {
        println!("Controller.on_editor_data_download_completed");
        let midi = self.midi_static_clone();
        let midi_guard = midi.lock().unwrap();
        self.show_info("Getting instrument config...");
        midi_guard.request_config();
    }

    fn on_instru_connected_changed(&self) {
        // println!("Controller.on_instru_connected_changed");
        let midi = self.midi_static_clone();
        let midi_guard = midi.lock().unwrap();
        if midi_guard.is_instru_connected() {
            println!("Controller.on_instru_connected_changed: Awaiting editor data download completion.");
            self.show_info("Awaiting editor data download completion...");
            return;
        }
        // Instrument is not connected. Stop OSC if running.
        // println!("Controller.on_instru_connected_changed: Instrument is not connected.");
        let osc = self.osc_static_clone();
        let mut osc_guard = osc.lock().unwrap();
        if osc_guard.is_running() {
            // println!("Controller.on_instru_connected_changed: Stopping OSC");
            osc_guard.stop();
            self.show_warning(
                "Instrument is disconnected; closed PitchGrid connection.");
        } else if midi_guard.are_ports_connected() {
            // println!("Controller.on_instru_connected_changed: Showing The instrument is not connected");
            // This probably means the instrument is not connected on application start.
            // So show a helpful message.
            self.show_warning(
                "The instrument is not connected. Waiting for the editor to be \
                        opened with this application and the instrument connected to it...");
        }
        self.callbacks.show_pitchgrid_status(
            "PitchGrid connection closed while instrument disconnected",
            MessageType::Warning);
    }

    fn on_tuning_updated(&self) {
        println!("Controller.on_tuning_updated: Showing tuning");
        self.callbacks.show_tuning();
        println!("Controller.on_tuning_updated: Showing Instrument tuning updated");
        self.callbacks.show_pitchgrid_status("Instrument tuning updated", MessageType::Info);
    }

    fn osc_static_clone(&self) -> SharedOsc {
        Arc::clone(&OSC)
    }

    fn show_connected_device_name(
        &mut self, device_name: &str, port_strategy: &dyn PortStrategy) {
        let message_type = if device_name == PORT_NONE {
            MessageType::Warning
        } else {
            MessageType::Info
        };
        let port_setting = if device_name == PORT_NONE {
            ""
        } else {
            device_name
        };
        port_strategy.set_port_setting(&mut self.settings, port_setting);
        self.callbacks.show_connected_device_name(device_name, message_type, port_strategy);
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
        println!("Controller.show_pitchgrid_connected: Showing Pitchgrid OSC is connected");
        self.callbacks.show_pitchgrid_status(
            "Pitchgrid OSC is connected",
            MessageType::Info);
    }

    fn show_pitchgrid_disconnected(&self) {
        self.callbacks.show_pitchgrid_status(
            "Disconnected from PitchGrid because MIDI is not connected",
            MessageType::Warning);
    }

    fn show_pitchgrid_not_connected(&self) {
        self.callbacks.show_pitchgrid_status(
            "PitchGrid is not connected. OSC must be enabled in Pitchgrid.",
            MessageType::Error);
    }

    fn show_warning(&self, message: &str) {
        // println!("Controller.show_warning: {}", message);
        self.callbacks.show_message(message, MessageType::Warning);
    }
    
    fn stop_osc_and_instru_connection_monitor(&self, midi: &SharedMidi, osc: &SharedOsc) {
        // println!("Controller.stop_osc_and_instru_connection_monitor");
        let mut midi_guard = midi.lock().unwrap();
        midi_guard.stop_instru_connection_monitor();
        osc.lock().unwrap().stop();
        // println!("Controller.stop_osc_and_instru_connection_monitor: Done");
    }
}

impl OscCallbacks for Mutex<Controller> {
    fn on_osc_pitchgrid_connected_changed(&self) {
        // println!("OscCallbacks for Mutex<Controller>.on_osc_pitchgrid_connected_changed");
        let controller = self.lock().unwrap();
        controller.on_osc_pitchgrid_connected_changed();
    }

    fn on_osc_tuning_received(&self, depth: i32, mode: i32, root_freq: f32, stretch: f32,
                              skew: f32, mode_offset: i32, steps: i32) {
        // println!("OscCallbacks for Mutex<Controller>.on_osc_tuning_received");
        let controller = self.lock().unwrap();
        controller.on_osc_tuning_received(depth, mode, root_freq, stretch, skew, mode_offset, steps);
    }
}

impl OscCallbacks for Controller {
    fn on_osc_pitchgrid_connected_changed(&self) {
        println!("Controller.on_osc_pitchgrid_connected_changed");
        let osc = self.osc_static_clone();
        let osc_guard = osc.lock().unwrap();
        if osc_guard.is_pitchgrid_connected() {
            println!("Controller.on_osc_pitchgrid_connected_changed: Showing PitchGrid is connected");
            self.show_pitchgrid_connected();
            println!("Controller.on_osc_pitchgrid_connected_changed: PitchGrid and instrument are connected");
            self.show_info("PitchGrid and instrument are connected");
        } else {
            println!("Controller.on_osc_pitchgrid_connected_changed: PitchGrid is not connected");
            self.show_pitchgrid_not_connected();
        }
    }

    fn on_osc_tuning_received(&self, depth: i32, mode: i32, root_freq: f32, stretch: f32,
                              skew: f32, mode_offset: i32, steps: i32) {
        println!("Controller.on_osc_tuning_received");
        // println!(
        //     "Controller.on_osc_tuning_received: depth = {}; mode = {}; root_freq = {}; stretch = {}; \
        //     skew = {}; mode_offset = {}; steps = {}",
        //     depth, mode, root_freq, stretch, skew, mode_offset, steps);
        let midi = self.midi_static_clone();
        let midi_guard = midi.lock().unwrap();
        let can_update_tuning = midi_guard.are_ports_connected();
        if can_update_tuning {
            println!("Controller.on_osc_tuning_received: Showing Updating instrument tuning");
            self.callbacks.show_pitchgrid_status("Updating instrument tuning", MessageType::Info);
            tuner::on_tuning_received(depth, mode, root_freq, stretch, skew, mode_offset, steps);
        } else {
            println!("Controller.on_osc_tuning_received: Cannot updating tuning");
            self.callbacks.show_pitchgrid_status(
                "Cannot updating tuning. Connect instrument input/output.",
                MessageType::Error);
        }
    }
}

const PORT_NONE: &str = "[None]";

pub trait ControllerCallbacks: Send + Sync {
    fn focus_port(&self, port_strategy: &dyn PortStrategy);
    fn get_selected_port_index(&self, port_strategy: &dyn PortStrategy) -> usize;
    fn set_selected_port_index(&self, index: usize, port_strategy: &dyn PortStrategy);
    fn set_devices_model(&self, device_names: &Vec<String>, port_strategy: &dyn PortStrategy);
    fn show_connected_device_name(&self, name: &str, msg_type: MessageType, port_strategy: &dyn PortStrategy);
    fn show_message(&self, msg: &str, msg_type: MessageType);
    fn show_pitchgrid_status(&self, status: &str, msg_type: MessageType);
    fn show_tuning(&self);
    fn set_selected_pitch_table_index(&self, index: i32);
}

type SharedOsc = Arc<Mutex<Osc>>;
type SharedController = Arc<Mutex<Controller>>;

static CONTROLLER: OnceLock<SharedController> = OnceLock::new();

lazy_static! {
    static ref MIDI: SharedMidi = Arc::new(Mutex::new(Midi::new()));
    static ref OSC: SharedOsc = Arc::new(Mutex::new(Osc::new()));
}
