// Prevent console window in addition to Slint window in Windows release builds when, e.g., starting the app via file manager. Ignored on other platforms.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod global;
mod midi;
mod midi_ports;
mod osc;
mod settings;
mod tuner;
mod port_strategy;

use std::cmp::max;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use lazy_static::lazy_static;
use slint::{CloseRequestResponse, SharedString, Weak};
use midi::{Midi, PortType};
use crate::global::{APP_TITLE, SharedMidi};
use crate::osc::Osc;
use crate::port_strategy::{
    InputStrategy, OutputStrategy, PortStrategy};
use crate::settings::Settings;

fn main() {
    let main_window = MainWindow::new().unwrap();
    main_window.set_window_title(APP_TITLE.into());
    let midi: SharedMidi = Arc::new(Mutex::new(Midi::new()));
    let settings: SharedSettings = Arc::new(Mutex::new(Settings::new()));
    init(&main_window, &midi, &settings);
    main_window.run().unwrap();
}

fn connect_initial_port(
    main_window: &MainWindow, midi: &SharedMidi,
    settings: &SharedSettings, port_strategy: &dyn PortStrategy) {
    // println!("main.connect_initial_port");
    // We have to limit the scope of the lock, as midi will have to be locked again in
    // connect_selected_port.
    let maybe_index = {
        let midi1 = midi.lock().unwrap();
        // println!("main.connect_initial_port: Locked midi");
        midi1.io(port_strategy).port().as_ref()
            .map(|port| port.index())
    }; // Lock is dropped here
    if let Some(index) = maybe_index {
        let index = index as i32;
        port_strategy.set_selected_port_index(main_window, index);
        connect_selected_port(main_window, midi, settings, port_strategy);
    } else {
        show_no_port_connected(main_window, settings, port_strategy);
        show_warning(&main_window, port_strategy.msg_connect());
        port_strategy.focus_port(main_window);
    }
}

fn connect_port(main_window_weak: Weak<MainWindow>, midi: &SharedMidi, osc: &SharedOsc,
                settings: &SharedSettings, port_strategy: &(dyn PortStrategy + Send + Sync)) {
    // TODO: Separate business logic and UI.
    println!("main.connect_port");
    let midi = Arc::clone(midi);
    let osc = Arc::clone(osc);
    let settings = Arc::clone(settings);
    let port_strategy = port_strategy.clone_box();
    println!("main.connect_port: Stopping OSC and instrument connection monitor");
    stop_osc_and_instru_connection_monitor(&midi, &osc);
    // TODO: Like in init_ui_handlers.
    let has_port_been_connected = connect_selected_port(main_window_weak, &midi, &settings, &*port_strategy);
    with_main_window(main_window_weak, move |main_window| {
        show_pitchgrid_disconnected(&main_window);
        println!("main.connect_port: Connecting port");
        //connect_selected_port(main_window, &midi, &settings, &*port_strategy);
        if let Some(port) = midi.lock().unwrap().io(&*port_strategy).port() {
            let port_name: &str = &port.name();
            show_info(main_window, port_strategy.msg_connected(port_name));
            println!("main.connect_port: Getting midi_clone");
            let midi_clone = Arc::clone(&midi);
            println!("main.connect_port: Got midi_clone");
            let midi_guard = midi_clone.lock().unwrap();
            println!("main.connect_port: Got midi_guard");
            if midi_guard.are_ports_connected() {
                println!("main.connect_port: Showing Restart this application");
                show_warning(
                    main_window, "Restart this application to connect to PitchGrid");
            }
        }
        println!("main.connect_port: with_main_window done");
    });
    println!("main.connect_port: Done");
}

fn connect_selected_port(main_window: &MainWindow, midi: &SharedMidi,
                         settings: &SharedSettings, port_strategy: &dyn PortStrategy) -> bool {
    println!("main.connect_selected_port");
    let selected = port_strategy.get_selected_port_index(main_window);
    let index: usize = match usize::try_from(selected) {
        Ok(i) => i,
        Err(_) => {
            // A port has not been selected. That's impossible with the UI as it is.
            show_no_port_connected(main_window, settings, port_strategy);
            show_error(main_window, port_strategy.msg_not_selected());
            return false;
        }
    };
    // Do all Midi borrowing/mutation inside a tight scope, then update UI after.
    let ui_action: Result<String, String> = {
        let mut midi_guard = midi.lock().unwrap();
        let Some(name) = midi_guard.io(port_strategy).port_names().get(index).cloned()
        else {
            return false;
        };
        match midi_guard.connect_port(index, port_strategy) {
            Ok(()) => Ok(name),
            Err(err) => Err(err.to_string()),
        }
    };
    match ui_action {
        Ok(name) => {
            show_connected_port_name(main_window, settings, &name, port_strategy);
            return true;
        }
        Err(message) => {
            show_no_port_connected(main_window, settings, port_strategy);
            show_error(main_window, message);
            return false;
        }
    }
    println!("main.connect_selected_port: Done");
}

fn handle_close_request(
    main_window_weak: Weak<MainWindow>, midi: &SharedMidi, osc: &SharedOsc,
    settings: &SharedSettings) -> CloseRequestResponse {
    println!("main.handle_close_request");
    let response =
        Arc::new(Mutex::new(CloseRequestResponse::HideWindow));
    if IS_CLOSE_ERROR_SHOWN.load(Ordering::Relaxed) {
        // If a close error message is already shown, allow the window to be closed.
        return *response.lock().unwrap()
    }
    // println!("main.handle_close_request: Cloning Midi");
    let midi_clone = Arc::clone(midi);
    // println!("main.handle_close_request: Getting Midi guard");
    let mut midi_guard = midi_clone.lock().unwrap();
    // println!("main.handle_close_request: Got Midi, closing");
    midi_guard.close();
    // println!("main.handle_close_request: Closed Midi.");
    let response_clone = Arc::clone(&response);
    let settings1 = Arc::clone(settings);
    with_main_window(main_window_weak, move |main_window| {
        if let Err(err) = settings1.lock().unwrap().write_to_file() {
            *response_clone.lock().unwrap() = CloseRequestResponse::KeepWindowShown;
            show_error(main_window, err.to_string());
            IS_CLOSE_ERROR_SHOWN.store(true, Ordering::Relaxed);
        }
    });
    // println!("main.handle_close_request: Stopping OSC");
    osc.lock().unwrap().stop();
    // println!("main.handle_close_request: Stopped OSC");
    *response.lock().unwrap()
}

fn init(main_window: &MainWindow, midi: &SharedMidi, settings: &SharedSettings) {
    // println!("main.init");
    let pitch_table_no: u8;
    {
        let mut settings1 = settings.lock().unwrap();
        let input_port_name: String;
        let output_port_name: String;
        match settings1.read_from_file() {
            Ok(_) => {
                input_port_name = settings1.midi_input_port.clone();
                output_port_name = settings1.midi_output_port.clone();
                pitch_table_no = max(tuner::default_pitch_table_no(), settings1.pitch_table);
            }
            Err(err) => {
                show_error(main_window, err.to_string());
                return;
            }
        }
        let mut midi_guard = midi.lock().unwrap();
        if let Err(err) = midi_guard.init(
            &input_port_name, &output_port_name) {
            show_error(main_window, err.to_string());
            return;
        }
        midi_guard.add_config_received_callback(Box::from(on_config_received));
        midi_guard.add_instru_connected_changed_callback(
            Box::from(on_instru_connected_changed));
        midi_guard.add_tuning_updated_callback(Box::from(on_tuning_updated));
    }
    let input_strategy = InputStrategy::new();
    let output_strategy = OutputStrategy::new();
    set_ports_model(&main_window, midi, &input_strategy);
    set_ports_model(&main_window, midi, &output_strategy);
    connect_initial_port(&main_window, midi, settings, &input_strategy);
    connect_initial_port(&main_window, midi, settings, &output_strategy);
    set_pitch_tables_model(&main_window);
    tuner::set_midi(midi.clone());
    tuner::set_pitch_table_no(pitch_table_no);
    main_window.set_selected_pitch_table_index(tuner::pitch_table_index() as i32);
    let osc: SharedOsc;
    {
        *MAIN_WINDOW_WEAK.lock().unwrap() = Some(main_window.as_weak().clone());
        println!("main.init: Creating MIDI");
        *MIDI.lock().unwrap() = Some(midi.clone());
        osc = OSC.clone();
    }
    init_ui_handlers(&main_window, Arc::clone(&midi), osc, Arc::clone(settings));
    println!("main.init: Checking if MIDI is connected");
    let mut midi_guard = midi.lock().unwrap();
    let is_midi_connected = midi_guard.are_ports_connected();
    if is_midi_connected {
        // println!("main.init: Showing Checking instrument connection message");
        show_info(main_window, "Checking instrument connection...");
        println!("main.init: Starting instrument connection monitor");
        midi_guard.start_instru_connection_monitor();
    }
}

fn init_ui_handlers(main_window: &MainWindow, midi: SharedMidi, osc: SharedOsc,
                    settings: SharedSettings) {
    // println!("main.init_ui_handlers");
    let window_weak = main_window.as_weak();
    {
        let midi: SharedMidi = Arc::clone(&midi);
        let osc: SharedOsc = Arc::clone(&osc);
        let settings: SharedSettings = Arc::clone(&settings);
        let window_weak = window_weak.clone();
        main_window.window().on_close_requested(move || {
            handle_close_request(window_weak.clone(), &midi, &osc, &settings)
        });
    }
    {
        let midi: SharedMidi = Arc::clone(&midi);
        let osc: SharedOsc = Arc::clone(&osc);
        let settings: SharedSettings = Arc::clone(&settings);
        let window_weak = window_weak.clone();
        main_window.on_connect_port(move |port_type: SlintPortType| {
            let port_strategy = create_port_strategy(port_type);
            connect_port(window_weak.clone(), &midi, &osc, &settings, &*port_strategy)
        });
    }
    {
        let midi: SharedMidi = Arc::clone(&midi);
        let osc: SharedOsc = Arc::clone(&osc);
        let settings: SharedSettings = Arc::clone(&settings);
        let window_weak = window_weak.clone();
        main_window.on_refresh_ports(move |port_type: SlintPortType| {
            let port_strategy = create_port_strategy(port_type);
            refresh_ports(window_weak.clone(), &midi, &osc, &settings, &*port_strategy)
        });
    }
    {
        let settings: SharedSettings = Arc::clone(&settings);
        main_window.on_selected_pitch_table_changed(move |index| {
            set_pitch_table_no(index as usize, &settings)
        });
    }
    // println!("main.init_ui_handlers: Done");
}

fn create_port_strategy(port_type: SlintPortType)
                        -> Box<dyn PortStrategy> {
    match port_type {
        SlintPortType::Input => InputStrategy::new().clone_box(),
        SlintPortType::Output => OutputStrategy::new().clone_box(),
    }
}

fn main_window_weak_static() -> Option<Weak<MainWindow>> {
    MAIN_WINDOW_WEAK.lock().unwrap().clone()
}

fn midi_static() -> SharedMidi {
    MIDI.lock().unwrap().clone().unwrap()
}

fn on_config_received() {
    println!("main.on_config_received");
    if let Some(main_window_weak) = main_window_weak_static() {
        with_main_window(main_window_weak.clone(), move |main_window| {
            // Remove Getting instrument config... message.
            show_info(
                main_window, "Got instrument config. Opening PitchGrid connection.");
        });
    }
    OSC.lock().unwrap().start(
        Arc::new(on_osc_tuning_received), Arc::new(on_osc_connected_changed));
}

fn on_instru_connected_changed() {
    println!("main.on_instru_connected_changed");
    let midi = midi_static();
    println!("main.on_instru_connected_changed: Got midi");
    let midi_guard = midi.lock().unwrap();
    println!("main.on_instru_connected_changed: Got midi_guard");
    let are_ports_connected = midi_guard.are_ports_connected();
    let is_instru_connected = midi_guard.is_instru_connected();
    if is_instru_connected {
        midi_guard.request_config()
    }
    println!("main.on_instru_connected_changed: is_instru_connected = {}", is_instru_connected);
    if let Some(main_window_weak) = main_window_weak_static() {
        with_main_window(main_window_weak, move |main_window| {
            if is_instru_connected {
                println!(
                    "main.on_instru_connected_changed: Showing Getting instrument config");
                show_info(main_window,
                          "Instrument is connected. Getting instrument config...");
                println!("main.on_instru_connected_changed: Requesting config");
            } else {
                let mut osc = OSC.lock().unwrap();
                if osc.is_connected() {
                    println!("main.on_instru_connected_changed: Stopping OSC");
                    osc.stop();
                    show_warning(
                        main_window,
                        "Instrument is disconnected; closed PitchGrid connection.");
                } else if are_ports_connected {
                    // This probably means the instrument is not connected on application start.
                    // So show a helpful message.
                    println!(
                        "main.on_instru_connected_changed: Showing Instrument is disconnected");
                    show_warning(
                        main_window,
                        "The instrument is not connected. Waiting for the editor to be \
                        opened with this application and the instrument connected to it...");
                }
                show_pitchgrid_status(
                    main_window,
                    "PitchGrid connection closed while instrument disconnected",
                    MessageType::Warning);
            }
        });
    }
}

fn on_osc_connected_changed() {
    if let Some(main_window_weak) = main_window_weak_static() {
        // println!("main.on_osc_connected_changed: Found main_window_weak");
        let is_connected = OSC.lock().unwrap().is_connected();
        // println!("main.on_osc_connected_changed: Connected = {}", is_connected);
        with_main_window(main_window_weak.clone(), move |main_window| {
            // println!("main.on_osc_connected_changed: Found main_window");
            if is_connected {
                show_pitchgrid_connected(main_window);
                show_info(main_window, "PitchGrid and instrument are connected.");
            } else {
                show_pitchgrid_not_connected(main_window);
            }
        });
    }
}

fn on_osc_tuning_received(depth: i32, mode: i32, root_freq: f32, stretch: f32,
                          skew: f32, mode_offset: i32, steps: i32) {
    // println!(
    //     "main.on_osc_tuning_received: depth = {}; mode = {}; root_freq = {}; stretch = {}; \
    //     skew = {}; mode_offset = {}; steps = {}",
    //     depth, mode, root_freq, stretch, skew, mode_offset, steps);
    let midi = midi_static();
    let midi_guard = midi.lock().unwrap();
    let can_update_tuning = midi_guard.are_ports_connected();
    if can_update_tuning {
        tuner::on_tuning_received(depth, mode, root_freq, stretch, skew, mode_offset, steps);
    }
    if let Some(main_window_weak) = main_window_weak_static() {
        with_main_window(main_window_weak.clone(), move |main_window| {
            if can_update_tuning {
                show_pitchgrid_status(
                    main_window, "Updating instrument tuning", MessageType::Info);
            } else {
                show_pitchgrid_status(
                    main_window,
                    "Cannot updating tuning. Connect instrument input/output.",
                    MessageType::Error);
            }
        });
    }
}

fn on_tuning_updated() {
    // println!("main.on_tuning_updated");
    if let Some(main_window_weak) = main_window_weak_static() {
        with_main_window(main_window_weak.clone(), move |main_window| {
            let params = tuner::formatted_tuning_params();
            main_window.set_depth(params.depth.into());
            main_window.set_root_freq(params.root_freq.into());
            main_window.set_stretch(params.stretch.into());
            main_window.set_skew(params.skew.into());
            main_window.set_mode_offset(params.mode_offset.into());
            main_window.set_steps(params.steps.into());
            show_pitchgrid_status(main_window,
                                  "Instrument tuning updated", MessageType::Info);
        });
    }
}

fn refresh_ports(
        main_window_weak: Weak<MainWindow>, midi: &SharedMidi,
        osc: &SharedOsc, settings: &SharedSettings, port_strategy: &dyn PortStrategy) {
    let midi = Arc::clone(midi);
    let osc = Arc::clone(osc);
    let settings = Arc::clone(settings);
    let port_strategy = port_strategy.clone_box();
    stop_osc_and_instru_connection_monitor(&midi, &osc);
    with_main_window(main_window_weak, move |main_window| {
        let port_name = port_strategy.port_setting(&settings.lock().unwrap()).to_string();
        if let Err(err) = midi.lock().unwrap().refresh_ports(
                &port_name, &*port_strategy) {
            show_error(main_window, err.to_string());
            return;
        }
        show_pitchgrid_disconnected(&main_window);
        set_ports_model(&main_window, &midi, &*port_strategy);
        show_no_port_connected(main_window, &settings, &*port_strategy);
        show_warning(main_window, port_strategy.msg_refreshed_reconnect());
    });
}

fn set_pitch_table_no(index: usize, settings: &SharedSettings) {
    let pitch_table_no = tuner::pitch_table_nos()[index];
    tuner::set_pitch_table_no(pitch_table_no);
    settings.lock().unwrap().pitch_table = pitch_table_no;
}

fn set_pitch_tables_model(main_window: &MainWindow) {
    let pitch_table_items: Vec<ComboBoxItem> = tuner::pitch_table_nos()
        .iter()
        .map(|grid_no| ComboBoxItem { text: grid_no.to_string().into() })
        .collect();
    let model = Rc::new(TuningGridsModel(pitch_table_items));
    main_window.set_pitch_tables_model(slint::ModelRc::from(model));
}

fn set_ports_model(main_window: &MainWindow, midi: &SharedMidi, port_strategy: &dyn PortStrategy) {
    let port_items: Vec<ComboBoxItem> =
        midi.lock().unwrap().io(port_strategy).port_names()
        .iter()
        .map(|text| ComboBoxItem { text: text.into() })
        .collect();
        let model =
            match port_strategy.port_type() {
                PortType::Input => {
                    let input_model = Rc::new(InputPortsModel(port_items));
                    slint::ModelRc::from(input_model)
                },
                PortType::Output => {
                    let output_model = Rc::new(OutputPortsModel(port_items));
                    slint::ModelRc::from(output_model)
                },
            };
        port_strategy.set_ports_model(main_window, model);
}

fn show_connected_port_name(main_window: &MainWindow, settings: &SharedSettings,
                            port_name: &str, port_strategy: &dyn PortStrategy) {
    let message_type = if port_name == PORT_NONE {
        MessageType::Warning }
    else {
        MessageType::Info
    };
    port_strategy.show_connected_port_name(main_window, port_name, message_type);
    let mut settings1 = settings.lock().unwrap();
    port_strategy.set_port_setting(&mut settings1, port_name);
}

fn show_error(main_window: &MainWindow, message: impl Into<SharedString>) {
    show_message(main_window, message, MessageType::Error);
}

fn show_info(main_window: &MainWindow, message: impl Into<SharedString>) {
    show_message(main_window, message, MessageType::Info);
}

fn show_message(main_window: &MainWindow, message: impl Into<SharedString>, message_type: MessageType) {
    main_window.invoke_show_message(message.into(), message_type);
}

fn show_no_port_connected(main_window: &MainWindow, settings: &SharedSettings,
                          port_strategy: &dyn PortStrategy) {
    show_connected_port_name(main_window, settings, PORT_NONE, port_strategy);
}

fn show_pitchgrid_connected(main_window: &MainWindow) {
    show_pitchgrid_status(
        main_window, "Pitchgrid OSC is connected", MessageType::Info);
}

fn show_pitchgrid_disconnected(main_window: &MainWindow) {
    println!("show_pitchgrid_disconnected");
    show_pitchgrid_status(
        main_window, "Disconnected from PitchGrid because MIDI is not connected",
        MessageType::Warning);
}

fn show_pitchgrid_not_connected(main_window: &MainWindow) {
    show_pitchgrid_status(
        main_window, "PitchGrid is not connected. OSC must be enabled in Pitchgrid.",
        MessageType::Error);
}

fn show_pitchgrid_status(
    main_window: &MainWindow, message: impl Into<SharedString>, message_type: MessageType) {
    main_window.invoke_show_pitchgrid_status(message.into(), message_type);
}

fn show_warning(main_window: &MainWindow, message: impl Into<SharedString>) {
    show_message(main_window, message, MessageType::Warning);
}

fn stop_osc_and_instru_connection_monitor(midi: &SharedMidi, osc: &SharedOsc) {
    println!("main.stop_osc_and_instru_connection_monitor");
    let mut midi_guard = midi.lock().unwrap();
    midi_guard.stop_instru_connection_monitor();
    osc.lock().unwrap().stop();
    println!("main.stop_osc: Got osc, stopped it");
}

/// Upgrades a weak reference to a `MainWindow` to a strong reference, then calls the closure with
/// the strong reference. If called from a non-UI thread, the closure will be executed in the UI
/// event loop.
fn with_main_window(main_window_weak: Weak<MainWindow>,
                    f: impl FnOnce(&MainWindow) + Send + 'static) {
    main_window_weak.upgrade_in_event_loop(move |main_window| {
        f(&main_window);
    }).ok();
}

type SharedOsc = Arc<Mutex<Osc>>;

type SharedSettings = Arc<Mutex<Settings>>;

struct InputPortsModel(Vec<ComboBoxItem>);

impl slint::Model for InputPortsModel {
    type Data = ComboBoxItem;
    fn row_count(&self) -> usize {
        self.0.len()
    }
    fn row_data(&self, row: usize) ->
        Option<Self::Data> {
        self.0.get(row).map(|x| x.clone())
    }
    fn model_tracker(&self) -> &dyn slint::ModelTracker {
        &()
    }
}

struct OutputPortsModel(Vec<ComboBoxItem>);

impl slint::Model for OutputPortsModel {
    type Data = ComboBoxItem;
    fn row_count(&self) -> usize {
        self.0.len()
    }
    fn row_data(&self, row: usize) ->
        Option<Self::Data> {
        self.0.get(row).map(|x| x.clone())
    }
    fn model_tracker(&self) -> &dyn slint::ModelTracker {
        &()
    }
}

struct TuningGridsModel(Vec<ComboBoxItem>);

impl slint::Model for TuningGridsModel {
    type Data = ComboBoxItem;
    fn row_count(&self) -> usize {
        self.0.len()
    }
    fn row_data(&self, row: usize) ->
        Option<Self::Data> {
        self.0.get(row).map(|x| x.clone())
    }
    fn model_tracker(&self) -> &dyn slint::ModelTracker {
        &()
    }
}

const PORT_NONE: &str = "[None]";

slint::include_modules!();

lazy_static! {
    static ref IS_CLOSE_ERROR_SHOWN: AtomicBool = AtomicBool::new(false);
    static ref MAIN_WINDOW_WEAK: Mutex<Option<Weak<MainWindow>>> = Mutex::new(None);
    static ref MIDI: Mutex<Option<SharedMidi>> = Mutex::new(None);
    static ref OSC: SharedOsc = Arc::new(Mutex::new(Osc::new()));
}
