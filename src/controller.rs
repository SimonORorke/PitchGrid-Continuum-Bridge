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

/// This is the controller in the Model-View-Controller (MVC) pattern.
/// The Slint UI and main are the view. Everything else is the model.
pub struct Controller {
    focus_port: FocusPortCallback,
    get_selected_port_index: GetSelectedPortIndexCallback,
    set_selected_port_index: SetSelectedPortIndexCallback,
    set_ports_model: SetPortsModelCallback,
    show_connected_port_name: ShowConnectedPortNameCallback,
    show_message: ShowMessageCallback,
    show_pitchgrid_status: ShowPitchgridStatusCallback,
    show_tuning: ShowTuningCallback,
    set_selected_pitch_table_index: SetSelectedPitchTableIndexCallback,
    settings: Settings,
}

impl Controller {
    pub fn new(
        focus_port: FocusPortCallback,
        get_selected_port_index: GetSelectedPortIndexCallback,
        set_selected_port_index: SetSelectedPortIndexCallback,
        set_ports_model: SetPortsModelCallback,
        show_connected_port_name: ShowConnectedPortNameCallback,
        show_message: ShowMessageCallback,
        show_pitchgrid_status: ShowPitchgridStatusCallback,
        show_tuning: ShowTuningCallback,
        set_selected_pitch_table_index: SetSelectedPitchTableIndexCallback,
    ) -> Self {
        Self {
            focus_port,
            get_selected_port_index,
            set_selected_port_index,
            set_ports_model,
            show_connected_port_name,
            show_message,
            show_pitchgrid_status,
            show_tuning,
            set_selected_pitch_table_index,
            settings: Settings::new(),
        }
    }

    pub fn init(&mut self) {
        let pitch_table_no: u8;
        let input_port_name: String;
        let output_port_name: String;
        match self.settings.read_from_file() {
            Ok(_) => {
                input_port_name = self.settings.midi_input_port.clone();
                output_port_name = self.settings.midi_output_port.clone();
                pitch_table_no = max(tuner::default_pitch_table_no(), self.settings.pitch_table);
            }
            Err(err) => {
                self.show_error(&err.to_string());
                return;
            }
        }
        let midi = self.midi_static_clone();
        let mut midi_guard = midi.lock().unwrap();
        if let Err(err) = midi_guard.init(
            &input_port_name, &output_port_name) {
            self.show_error(&err.to_string());
            return;
        }
        midi_guard.add_config_received_callback(Box::new(|| {
            if let Some(controller) = CONTROLLER.get() {
                controller.lock().unwrap().on_config_received();
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
        let input_strategy = InputStrategy::new();
        let output_strategy = OutputStrategy::new();
        (self.set_ports_model)(self, &input_strategy);
        (self.set_ports_model)(self, &output_strategy);
        self.connect_initial_port(&input_strategy);
        self.connect_initial_port(&output_strategy);
        tuner::set_midi(midi.clone());
        tuner::set_pitch_table_no(pitch_table_no);
        (self.set_selected_pitch_table_index)(tuner::pitch_table_index() as i32);
        if midi_guard.are_ports_connected() {
            self.show_info("Checking instrument connection...");
            midi_guard.start_instru_connection_monitor();
        }
    }

    #[allow(clippy::unwrap_used)]
    pub fn close(&mut self) -> Result<(), Box<dyn Error>> {
        let midi = self.midi_static_clone();
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
        let midi = self.midi_static_clone();
        let maybe_index = {
            let midi_guard = midi.lock().unwrap();
            midi_guard.io(port_strategy).port().as_ref()
                .map(|port| port.index())
        };
        if let Some(index) = maybe_index {
            (self.set_selected_port_index)(index, port_strategy);
            self.connect_selected_port(&midi, port_strategy);
        } else {
            self.show_no_port_connected(port_strategy);
            self.show_warning(port_strategy.msg_connect());
            (self.focus_port)(port_strategy);
        }
    }

    pub fn connect_port(&mut self, port_strategy: &dyn PortStrategy) {
        let midi = self.midi_static_clone();
        let osc = self.osc_static_clone();
        let port_strategy = port_strategy.clone_box();
        self.stop_osc_and_instru_connection_monitor(&midi, &osc);
        self.show_pitchgrid_disconnected();
        self.connect_selected_port(&midi, &*port_strategy);
        if let Some(port) = midi.lock().unwrap().io(&*port_strategy).port() {
            let port_name: &str = &port.name();
            self.show_info(port_strategy.msg_connected(port_name));
            let midi_guard = midi.lock().unwrap();
            if midi_guard.are_ports_connected() {
                self.show_warning("Restart this application to connect to PitchGrid");
            }
        }
    }

    fn connect_selected_port(&mut self, midi: &SharedMidi, port_strategy: &dyn PortStrategy) {
        let selected = (self.get_selected_port_index)(port_strategy);
        let index: usize = match usize::try_from(selected) {
            Ok(i) => i,
            Err(_) => {
                // A port has not been selected. That's impossible with the UI as it is.
                self.show_no_port_connected(port_strategy);
                self.show_error(port_strategy.msg_not_selected());
                return;
            }
        };
        let ui_action: Result<String, String> = {
            let mut midi_guard = midi.lock().unwrap();
            let Some(name) = midi_guard.io(port_strategy).port_names().get(index).cloned()
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
                self.show_connected_port_name(&name, port_strategy);
            }
            Err(message) => {
                self.show_no_port_connected(port_strategy);
                self.show_error(&message);
            }
        }
    }

    pub fn port_names(&self, port_strategy: &dyn PortStrategy) -> Vec<String> {
        let midi = self.midi_static_clone();
        println!("main.on_instru_connected_changed: Got midi");
        let midi_guard = midi.lock().unwrap();
        midi_guard.io(port_strategy).port_names()
    }

    pub fn refresh_ports(&mut self, port_strategy: &dyn PortStrategy) {
        let midi = self.midi_static_clone();
        let osc = self.osc_static_clone();
        let port_strategy = port_strategy.clone_box();
        self.stop_osc_and_instru_connection_monitor(&midi, &osc);
        let port_name = port_strategy.port_setting(&self.settings).to_string();
        if let Err(err) = midi.lock().unwrap().refresh_ports(
            &port_name, &*port_strategy) {
            self.show_error(&err.to_string());
            return;
        }
        self.show_pitchgrid_disconnected();
        (self.set_ports_model)(&self, &*port_strategy);
        self.show_no_port_connected(&*port_strategy);
        self.show_warning(port_strategy.msg_refreshed_reconnect());
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
        self.show_info("Got instrument config. Opening PitchGrid connection.");
        let osc = self.osc_static_clone();
        let mut osc_guard = osc.lock().unwrap();
        if let Some(controller) = CONTROLLER.get() {
            osc_guard.start(controller.clone());
        }
    }

    fn on_instru_connected_changed(&self) {
        let midi = self.midi_static_clone();
        let midi_guard = midi.lock().unwrap();
        if midi_guard.is_instru_connected() {
            self.show_info("Instrument is connected. Getting instrument config...");
            midi_guard.request_config();
            return;
        }
        // Instrument is not connected. Stop OSC.
        let osc = self.osc_static_clone();
        let mut osc_guard = osc.lock().unwrap();
        if osc_guard.is_connected() {
            println!("main.on_instru_connected_changed: Stopping OSC");
            osc_guard.stop();
            self.show_warning(
                "Instrument is disconnected; closed PitchGrid connection.");
        } else if midi_guard.are_ports_connected() {
            // This probably means the instrument is not connected on application start.
            // So show a helpful message.
            self.show_warning(
                "The instrument is not connected. Waiting for the editor to be \
                        opened with this application and the instrument connected to it...");
        }
        (self.show_pitchgrid_status)(
            "PitchGrid connection closed while instrument disconnected",
            MessageType::Warning);
    }

    fn on_tuning_updated(&self) {
        (self.show_tuning)();
        (self.show_pitchgrid_status)("Instrument tuning updated", MessageType::Info);
    }

    fn osc_static_clone(&self) -> SharedOsc {
        Arc::clone(&OSC)
    }

    fn show_connected_port_name(
        &mut self, port_name: &str, port_strategy: &dyn PortStrategy) {
        let message_type = if port_name == PORT_NONE {
            MessageType::Warning
        } else {
            MessageType::Info
        };
        let port_setting = if port_name == PORT_NONE {
            ""
        } else {
            port_name
        };
        port_strategy.set_port_setting(&mut self.settings, port_setting);
        (self.show_connected_port_name)(port_name, message_type, port_strategy);
    }

    fn show_error(&self, message: &str) {
        (self.show_message)(message, MessageType::Error);
    }

    fn show_info(&self, message: &str) {
        (self.show_message)(message, MessageType::Info);
    }

    fn show_no_port_connected(
        &mut self, port_strategy: &dyn PortStrategy) {
        self.show_connected_port_name(PORT_NONE, port_strategy);
    }
    
    fn show_pitchgrid_connected(&self) {
        (self.show_pitchgrid_status)(
            "Pitchgrid OSC is connected",
            MessageType::Info);
    }

    fn show_pitchgrid_disconnected(&self) {
        (self.show_pitchgrid_status)(
            "Disconnected from PitchGrid because MIDI is not connected",
            MessageType::Warning);
    }

    fn show_pitchgrid_not_connected(&self) {
        (self.show_pitchgrid_status)(
            "PitchGrid is not connected. OSC must be enabled in Pitchgrid.",
            MessageType::Error);
    }

    fn show_warning(&self, message: &str) {
        (self.show_message)(message, MessageType::Warning);
    }
    
    fn stop_osc_and_instru_connection_monitor(&self, midi: &SharedMidi, osc: &SharedOsc) {
        // println!("controller.stop_osc_and_instru_connection_monitor");
        let mut midi_guard = midi.lock().unwrap();
        midi_guard.stop_instru_connection_monitor();
        osc.lock().unwrap().stop();
    }
}

impl OscCallbacks for Mutex<Controller> {
    fn on_osc_connected_changed(&self) {
        let controller = self.lock().unwrap();
        controller.on_osc_connected_changed();
    }

    fn on_osc_tuning_received(&self, depth: i32, mode: i32, root_freq: f32, stretch: f32,
                              skew: f32, mode_offset: i32, steps: i32) {
        let controller = self.lock().unwrap();
        controller.on_osc_tuning_received(depth, mode, root_freq, stretch, skew, mode_offset, steps);
    }
}

impl OscCallbacks for Controller {
    fn on_osc_connected_changed(&self) {
        let osc = self.osc_static_clone();
        let osc_guard = osc.lock().unwrap();
        if osc_guard.is_connected() {
            self.show_pitchgrid_connected();
            self.show_info("PitchGrid and instrument are connected.");
        } else {
            self.show_pitchgrid_not_connected();
        }
    }

    fn on_osc_tuning_received(&self, depth: i32, mode: i32, root_freq: f32, stretch: f32,
                              skew: f32, mode_offset: i32, steps: i32) {
        // println!(
        //     "controller.on_osc_tuning_received: depth = {}; mode = {}; root_freq = {}; stretch = {}; \
        //     skew = {}; mode_offset = {}; steps = {}",
        //     depth, mode, root_freq, stretch, skew, mode_offset, steps);
        let midi = self.midi_static_clone();
        let midi_guard = midi.lock().unwrap();
        let can_update_tuning = midi_guard.are_ports_connected();
        if can_update_tuning {
            (self.show_pitchgrid_status)("Updating instrument tuning", MessageType::Info);
            tuner::on_tuning_received(depth, mode, root_freq, stretch, skew, mode_offset, steps);
        } else {
            (self.show_pitchgrid_status)(
                "Cannot updating tuning. Connect instrument input/output.",
                MessageType::Error);
        }
    }
}

const PORT_NONE: &str = "[None]";

pub type FocusPortCallback = Box<dyn Fn(&dyn PortStrategy) + Send + Sync + 'static>;
pub type GetSelectedPortIndexCallback = Box<dyn Fn(&dyn PortStrategy) -> usize + Send + Sync + 'static>;
pub type SetSelectedPortIndexCallback = Box<dyn Fn(usize, &dyn PortStrategy) + Send + Sync + 'static>;
pub type SetPortsModelCallback = Box<dyn Fn(&Controller, &dyn PortStrategy) + Send + Sync + 'static>;
pub type ShowConnectedPortNameCallback = Box<dyn Fn(&str, MessageType, &dyn PortStrategy) + Send + Sync + 'static>;
pub type ShowMessageCallback = Box<dyn Fn(&str, MessageType) + Send + Sync + 'static>;
pub type ShowPitchgridStatusCallback = Box<dyn Fn(&str, MessageType) + Send + Sync + 'static>;
pub type ShowTuningCallback = Box<dyn Fn() + Send + Sync + 'static>;
pub type SetSelectedPitchTableIndexCallback = Box<dyn Fn(i32) + Send + Sync + 'static>;
type SharedOsc = Arc<Mutex<Osc>>;
type SharedController = Arc<Mutex<Controller>>;

static CONTROLLER: OnceLock<SharedController> = OnceLock::new();

pub fn set_controller(controller: SharedController) {
    CONTROLLER.set(controller).ok();
}

lazy_static! {
    static ref MIDI: SharedMidi = Arc::new(Mutex::new(Midi::new()));
    static ref OSC: SharedOsc = Arc::new(Mutex::new(Osc::new()));
}
