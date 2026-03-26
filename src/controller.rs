use std::cmp::max;
use std::error::Error;
use std::sync::{Arc, Mutex, OnceLock};
use crate::global::{MessageType, Rounding, SharedMidi,};
use crate::osc::{Osc, OscCallbacks};
use crate::port_strategy::{
    InputStrategy, OutputStrategy, PortStrategy};
use crate::settings::Settings;
use crate::{global, midi_static, tuner};

/// This is the main controller in the Model-View-Controller (MVC) pattern.
/// PortStrategy contains both view and controller methods.
/// The Slint UI, main.rs and UiMethods are the remainder of the view.
/// Everything else is the model.
pub struct Controller {
    callbacks: Box<dyn ControllerCallbacks>,
    osc: SharedOsc,
    settings: Settings,
}

impl Controller {
    pub fn new(callbacks: Box<dyn ControllerCallbacks>) -> Self {
        Self {
            callbacks,
            osc: Arc::new(Mutex::new(Osc::new())),
            settings: Settings::new(),
        }
    }

    pub fn init(&mut self) {
        // println!("Controller.init");
        let pitch_table_no: u8;
        let input_device_name: String;
        let output_device_name: String;
        let rounding: Rounding;
        // println!("Controller.init: Reading settings");
        match self.settings.read_from_file() {
            Ok(_) => {
                input_device_name = self.settings.midi_input_device.clone();
                output_device_name = self.settings.midi_output_device.clone();
                pitch_table_no = max(tuner::default_pitch_table_no(), self.settings.pitch_table);
                rounding = self.rounding_from_name(&self.settings.rounding);
            }
            Err(err) => {
                self.show_error(&err.to_string());
                return;
            }
        }
        // println!("Controller.init: Getting midi");
        let midi = midi_static::clone_midi();
        let mut midi_guard = midi.lock().unwrap();
        if let Err(err) = midi_guard.init(
            &input_device_name, &output_device_name) {
            self.show_error(&err.to_string());
            return;
        }
        // println!("Controller.init: Adding download completed callback");
        midi_guard.add_download_completed_callback(Box::new(|| {
            if let Some(controller) = CONTROLLER.get() {
                controller.lock().unwrap().on_instru_data_download_completed();
            }
        }));
        midi_guard.add_ports_connected_changed_callback(Box::new(|| {
            if let Some(controller) = CONTROLLER.get() {
                controller.lock().unwrap().on_ports_connected_changed();
            }
        }));
        // println!("Controller.init: Adding selected preset loaded callback");
        midi_guard.add_new_preset_selected_callback(Box::new(|| {
            if let Some(controller) = CONTROLLER.get() {
                controller.lock().unwrap().on_new_preset_selected();
            }
        }));
        midi_guard.add_receiving_data_started_callback(Box::new(|| {
            if let Some(controller) = CONTROLLER.get() {
                controller.lock().unwrap().on_receiving_instru_data_changed_callback();
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
        tuner::set_rounding(rounding);
        self.callbacks.set_selected_rounding_index(self.rounding_index(rounding) as i32);
        if midi_static::are_ports_connected() {
            let mut midi_guard = midi.lock().unwrap();
            // println!("Controller.init: Showing Checking instrument connection");
            self.show_info("Checking instrument connection...");
            midi_guard.start_instru_connection_monitor();
        }
        // println!("Controller.init: Done");
    }

    #[allow(clippy::unwrap_used)]
    pub fn close(&mut self) -> Result<(), Box<dyn Error>> {
        let midi = midi_static::clone_midi();
        // There is no way to avoid this unwrap, but there seems not to be a way of stopping
        // RustRover from suggesting it should be replaced with '?'.
        let mut midi_guard = midi.lock().unwrap();
        midi_guard.close();
        drop(midi_guard);
        drop(midi);

        let osc = self.osc.clone();
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
        let midi = midi_static::clone_midi();
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
        let midi = midi_static::clone_midi();
        //let osc = self.osc.clone();
        let port_strategy = port_strategy.clone_box();
        // println!("Controller.connect_port: Stopping OSC and instrument connection monitor");
        self.stop_osc_and_instru_connection_monitor(&midi);
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
        let midi = midi_static::clone_midi();
        // println!("Controller.device_names: Got midi");
        let midi_guard = midi.lock().unwrap();
        midi_guard.io(port_strategy).device_names()
    }

    pub fn refresh_devices(&mut self, port_strategy: &dyn PortStrategy) {
        let midi = midi_static::clone_midi();
        let port_strategy = port_strategy.clone_box();
        self.stop_osc_and_instru_connection_monitor(&midi);
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

    /// Sets the root frequency override and sends it to the instrument, if connected.
    /// We probably don't need a setting for this.
    /// The player should have to choose an override, if required, on startup.
    pub fn set_root_freq_override(&mut self, index: usize) {
        let send_tuning = midi_static::is_receiving_data();
        if send_tuning {
            self.callbacks.show_pitchgrid_status(
                "Updating root frequency override...",
                MessageType::Info);
        }
        tuner::set_root_freq_override(index, send_tuning);
    }

    /// Sets what type of rounding, if any, is required the next time tuning is sent.
    /// If rounding is not required, we don't want to disable rounding
    /// for presets that already have rounding enabled.
    /// So we don't to try to update the current preset till there is a tuning change,
    /// when rounding will be turned on if required, but not off if not required.
    pub fn set_rounding(&mut self, index: usize) {
        let rounding = self.rounding_from_index(index);
        tuner::set_rounding(rounding);
        self.settings.rounding = self.rounding_name(rounding);
    }

    fn rounding_from_index(&self, index: usize) -> Rounding {
        if index == 0 { Rounding::None }
        else if index == 1 { Rounding::Initial }
        else if index == 2 { Rounding::Max }
        else {
            println!("Invalid rounding index: {}", index);
            global::default_rounding() }
    }

    fn rounding_from_name(&self, name: &str) -> Rounding {
        if name == "None" { Rounding::None }
        else if name == "Initial" { Rounding::Initial }
        else if name == "Max" { Rounding::Max }
        else { global::default_rounding() } // When settings have just been initialised.
    }

    fn rounding_index(&self, rounding: Rounding) -> usize {
        match rounding {
            Rounding::None => 0,
            Rounding::Initial => 1,
            Rounding::Max => 2,
        }
    }

    pub fn rounding_name(&self, rounding: Rounding) -> String {
        match rounding {
            Rounding::None => "None".to_string(),
            Rounding::Initial => "Initial".to_string(),
            Rounding::Max => "Max".to_string(),
        }
    }

    pub fn set_pitch_table_no(&mut self, index: usize) {
        let pitch_table_no = tuner::pitch_table_nos()[index];
        tuner::set_pitch_table_no(pitch_table_no);
        self.settings.pitch_table = pitch_table_no;
    }

    fn start_osc(&self) {
        let osc = self.osc.clone();
        let mut osc_guard = osc.lock().unwrap();
        if let Some(controller) = CONTROLLER.get() {
            println!("Controller.start_osc:  Starting OSC");
            osc_guard.start(controller.clone());
        }
    }

    fn on_instru_data_download_completed(&self) {
        // println!("Controller.on_editor_data_download_completed");
        self.show_info("Opening PitchGrid connection...");
        self.start_osc();
    }

    fn on_ports_connected_changed(&self) {
        // println!("Controller.on_instru_connected_changed");
        if midi_static::is_receiving_data() {
            // println!("Controller.on_instru_connected_changed: Awaiting editor data download completion.");
            self.show_info("Awaiting completion of data download from instrument...");
            return;
        }
        // Instrument is not connected. Stop OSC if running.
        // println!("Controller.on_instru_connected_changed: Instrument is not connected.");
        let osc = self.osc.clone();
        let mut osc_guard = osc.lock().unwrap();
        if osc_guard.is_running() {
            // println!("Controller.on_instru_connected_changed: Stopping OSC");
            osc_guard.stop();
            self.show_warning(
                "Instrument is disconnected; closed PitchGrid connection.");
        } else if midi_static::are_ports_connected() {
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

    fn on_new_preset_selected(&self) {
        // println!("Controller.on_new_preset_selected");
        if tuner::resend_tuning() {
            // println!("Controller.on_new_preset_selected: Resent");
            self.callbacks.show_pitchgrid_status(
                "New instrument preset selected. Resent tuning...",
                MessageType::Info);
        }
    }

    fn on_receiving_instru_data_changed_callback(&self) {
        todo!()
    }

    fn on_tuning_updated(&self) {
        // println!("Controller.on_tuning_updated: Showing tuning");
        self.callbacks.show_tuning();
        // println!("Controller.on_tuning_updated: Showing Instrument tuning updated");
        self.callbacks.show_pitchgrid_status("Instrument tuning updated", MessageType::Info);
    }

    // fn osc.clone(&self) -> SharedOsc {
    //     let osc = OSC.get_or_init(|| Arc::new(Mutex::new(Osc::new())));
    //     Arc::clone(osc)
    // }

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
        // println!("Controller.show_pitchgrid_connected: Showing Pitchgrid OSC is connected");
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
    
    // fn stop_osc_and_instru_connection_monitor(&self, midi: &SharedMidi, osc: &SharedOsc) {
    fn stop_osc_and_instru_connection_monitor(&self, midi: &SharedMidi) {
        // println!("Controller.stop_osc_and_instru_connection_monitor");
        let mut midi_guard = midi.lock().unwrap();
        midi_guard.stop_instru_connection_monitor();
        self.osc.lock().unwrap().stop();
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
        // println!("Controller.on_osc_pitchgrid_connected_changed");
        let osc = self.osc.clone();
        let osc_guard = osc.lock().unwrap();
        if osc_guard.is_pitchgrid_connected() {
            // println!("Controller.on_osc_pitchgrid_connected_changed: Showing PitchGrid is connected");
            self.show_pitchgrid_connected();
            // println!("Controller.on_osc_pitchgrid_connected_changed: PitchGrid and instrument are connected");
            self.show_info("PitchGrid and instrument are connected");
        } else {
            // println!("Controller.on_osc_pitchgrid_connected_changed: PitchGrid is not connected");
            self.show_pitchgrid_not_connected();
            self.show_warning("Awaiting PitchGrid connection...");
        }
    }

    fn on_osc_tuning_received(&self, depth: i32, mode: i32, root_freq: f32, stretch: f32,
                              skew: f32, mode_offset: i32, steps: i32) {
        // println!("Controller.on_osc_tuning_received");
        // println!(
        //     "Controller.on_osc_tuning_received: depth = {}; mode = {}; root_freq = {}; stretch = {}; \
        //     skew = {}; mode_offset = {}; steps = {}",
        //     depth, mode, root_freq, stretch, skew, mode_offset, steps);
        if midi_static::are_ports_connected() {
            // println!("Controller.on_osc_tuning_received: Showing Updating instrument tuning");
            self.callbacks.show_pitchgrid_status("Updating instrument tuning", MessageType::Info);
            tuner::on_tuning_received(depth, mode, root_freq, stretch, skew, mode_offset, steps);
        } else {
            // println!("Controller.on_osc_tuning_received: Cannot updating tuning");
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
    fn set_selected_rounding_index(&self, index: i32);
    fn set_selected_pitch_table_index(&self, index: i32);
}

type SharedOsc = Arc<Mutex<Osc>>;
type SharedController = Arc<Mutex<Controller>>;

static CONTROLLER: OnceLock<SharedController> = OnceLock::new();
// static OSC: OnceLock<SharedOsc> = OnceLock::new();
