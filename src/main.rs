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

slint::include_modules!();

/// Giving separate names to this shared data struct and those in modules
/// and also to the static refs that use them works around a problem with RustRover where
/// Linter would sometimes falsely indicate compiler errors.
struct MainData {
    is_close_error_shown: Arc<AtomicBool>,
    main_window_weak: Option<Weak<MainWindow>>,
    midi: Option<SharedMidi>,
    osc: Osc,
}

lazy_static! {
    static ref MAIN_DATA: Mutex<MainData> = Mutex::new(MainData {
        is_close_error_shown: Arc::new(AtomicBool::new(false)),
        main_window_weak: None,
        midi: None,
        osc: Osc::new(),
    });
}

fn main() {
    let main_window = MainWindow::new().unwrap();
    main_window.set_window_title(APP_TITLE.into());
    let mut midi: SharedMidi = Arc::new(Mutex::new(Midi::new()));
    let mut settings: SharedSettings = Arc::new(Mutex::new(Settings::new()));
    init(&main_window, &mut midi, &mut settings);
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

fn connect_port(main_window_weak: Weak<MainWindow>, midi: &SharedMidi,
                settings: &SharedSettings, port_strategy: &(dyn PortStrategy + Send + Sync)) {
    let mut midi = Arc::clone(midi);
    let mut settings = Arc::clone(settings);
    let port_strategy = port_strategy.clone_box();
    with_main_window(main_window_weak, move |main_window| {
        connect_selected_port(main_window, &mut midi, &mut settings, &*port_strategy);
        if let Some(port) = midi.lock().unwrap().io(&*port_strategy).port() {
            let port_name: &str = &port.name();
            show_info(main_window, port_strategy.msg_connected(port_name));
        }
    });
}

fn connect_selected_port(main_window: &MainWindow, midi: &SharedMidi,
                         settings: &SharedSettings, port_strategy: &dyn PortStrategy) {
    // println!("main.connect_selected_port");
    let selected = port_strategy.get_selected_port_index(main_window);
    let index: usize = match usize::try_from(selected) {
        Ok(i) => i,
        Err(_) => {
            // A port has not been selected. That's impossible with the UI as it is.
            show_no_port_connected(main_window, settings, port_strategy);
            show_error(main_window, port_strategy.msg_not_selected());
            return;
        }
    };
    // Do all Midi borrowing/mutation inside a tight scope, then update UI after.
    let ui_action: Result<String, String> = {
        let mut midi1 = midi.lock().unwrap();
        let Some(name) = midi1.io(port_strategy).port_names().get(index).cloned()
        else {
            return;
        };
        match midi1.connect_port(index, port_strategy) {
            Ok(()) => Ok(name),
            Err(err) => Err(err.to_string()),
        }
    };
    match ui_action {
        Ok(name) => {
            show_connected_port_name(main_window, settings, &name, port_strategy);
        }
        Err(message) => {
            show_no_port_connected(main_window, settings, port_strategy);
            show_error(main_window, message);
        }
    }
}

fn handle_close_request(
        main_window_weak: Weak<MainWindow>, midi: &SharedMidi,
        settings: &SharedSettings) -> CloseRequestResponse {
    let response = Arc::new(Mutex::new(CloseRequestResponse::HideWindow));
    let mut data = MAIN_DATA.lock().unwrap();
    if data.is_close_error_shown.load(Ordering::Relaxed) {
        // If a close error message is already shown, allow the window to be closed.
        return *response.lock().unwrap()
    }
    Arc::clone(midi).lock().unwrap().close();
    let is_close_error_shown = Arc::clone(&data.is_close_error_shown);
    let response_clone = Arc::clone(&response);
    let settings1 = Arc::clone(settings);
    with_main_window(main_window_weak, move |main_window| {
        if let Err(err) = settings1.lock().unwrap().write_to_file() {
            *response_clone.lock().unwrap() = CloseRequestResponse::KeepWindowShown;
            show_error(main_window, err.to_string());
            is_close_error_shown.store(true, Ordering::Relaxed);
        }
    });
    data.osc.stop();
    *response.lock().unwrap()
}

fn init(main_window: &MainWindow, midi: &SharedMidi, settings: &SharedSettings) {
    println!("main.init");
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
        let mut midi1 = midi.lock().unwrap();
        if let Err(err) = midi1.init(
            &input_port_name, &output_port_name) {
            show_error(main_window, err.to_string());
            return;
        }
        midi1.add_preset_loaded_callback(Box::from(on_instru_preset_loaded));
        midi1.add_preset_loading_callback(Box::from(on_instru_preset_loading));
        // println!("main.init: Added tuning updated callback:");
        midi1.add_tuning_updated_callback(Box::from(on_tuning_updated));
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
    init_ui_handlers(&main_window, Arc::clone(&midi), Arc::clone(settings));
    let mut data = MAIN_DATA.lock().unwrap();
    data.main_window_weak = Some(main_window.as_weak().clone());
    data.midi = Some(midi.clone());
    data.osc.start(Arc::new(on_osc_tuning_received), Arc::new(on_osc_connected_changed));
}

fn init_ui_handlers(main_window: &MainWindow, midi: SharedMidi, settings: SharedSettings) {
    let window_weak = main_window.as_weak();
    {
        let mut midi: SharedMidi = Arc::clone(&midi);
        let mut settings: SharedSettings = Arc::clone(&settings);
        let window_weak = window_weak.clone();
        main_window.window().on_close_requested(move || {
            handle_close_request(window_weak.clone(), &mut midi, &mut settings)
        });
    }
    {
        let mut midi: SharedMidi = Arc::clone(&midi);
        let mut settings: SharedSettings = Arc::clone(&settings);
        let window_weak = window_weak.clone();
        main_window.on_connect_port(move |port_type: SlintPortType| {
            let port_strategy = create_port_strategy(port_type);
            connect_port(window_weak.clone(), &mut midi, &mut settings, &*port_strategy)
        });
    }
    {
        let mut midi: SharedMidi = Arc::clone(&midi);
        let mut settings: SharedSettings = Arc::clone(&settings);
        let window_weak = window_weak.clone();
        main_window.on_refresh_ports(move |port_type: SlintPortType| {
            let port_strategy = create_port_strategy(port_type);
            refresh_ports(window_weak.clone(), &mut midi, &mut settings, &*port_strategy)
        });
    }
    {
        let mut settings: SharedSettings = Arc::clone(&settings);
        main_window.on_selected_pitch_table_changed(move |index| {
            update_pitch_table_no(index as usize, &mut settings)
        });
    }
}

fn create_port_strategy(port_type: SlintPortType)
                        -> Box<dyn PortStrategy> {
    match port_type {
        SlintPortType::Input => InputStrategy::new().clone_box(),
        SlintPortType::Output => OutputStrategy::new().clone_box(),
    }
}

fn update_pitch_table_no(index: usize, settings: &SharedSettings) {
    let pitch_table_no = tuner::pitch_table_nos()[index];
    tuner::set_pitch_table_no(pitch_table_no);
    settings.lock().unwrap().pitch_table = pitch_table_no;
}

fn on_instru_preset_loaded() {
    let data = MAIN_DATA.lock().unwrap();
    if let Some(main_window_weak) = &data.main_window_weak {
        with_main_window(main_window_weak.clone(), move |main_window| {
            show_pitchgrid_status(main_window,
                "Instrument preset loaded: retuning...", MessageType::Info);
        });
    }
    tuner::retune();
}

fn on_instru_preset_loading() {
    let data = MAIN_DATA.lock().unwrap();
    if let Some(main_window_weak) = &data.main_window_weak {
        with_main_window(main_window_weak.clone(), move |main_window| {
            show_pitchgrid_status(main_window,
                "Instrument preset loading: retune pending...", MessageType::Info);
        });
    }
}

fn on_osc_connected_changed() {
    let data = MAIN_DATA.lock().unwrap();
    if let Some(main_window_weak) = &data.main_window_weak {
        // println!("main.on_osc_connected_changed: Found main_window_weak");
        let is_connected = data.osc.is_connected();
        // println!("main.on_osc_connected_changed: Connected = {}", is_connected);
        with_main_window(main_window_weak.clone(), move |main_window| {
            // println!("main.on_osc_connected_changed: Found main_window");
            if is_connected {
                show_pitchgrid_connected(main_window);
            } else {
                show_pitchgrid_disconnected(main_window);
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
    let data = MAIN_DATA.lock().unwrap();
    let midi = data.midi.clone().unwrap();
    let midi_guard = midi.lock().unwrap();
    let can_update_tuning =
        midi_guard.is_input_connected() && midi_guard.is_output_connected();
    if can_update_tuning {
        tuner::on_tuning_received(depth, mode, root_freq, stretch, skew, mode_offset, steps);
    }
    if let Some(main_window_weak) = &data.main_window_weak {
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
    let data = MAIN_DATA.lock().unwrap();
    if let Some(main_window_weak) = &data.main_window_weak {
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
        main_window_weak: Weak<MainWindow>, midi: &SharedMidi, settings: &SharedSettings,
        port_strategy: &dyn PortStrategy) {
    let midi = Arc::clone(midi);
    let mut settings = Arc::clone(settings);
    let port_strategy = port_strategy.clone_box();
    with_main_window(main_window_weak, move |main_window| {
        let port_name = port_strategy.port_setting(&settings.lock().unwrap()).to_string();
        if let Err(err) = midi.lock().unwrap().refresh_ports(
                &port_name, &*port_strategy) {
            show_error(main_window, err.to_string());
            return;
        }
        set_ports_model(&main_window, &midi, &*port_strategy);
        show_no_port_connected(main_window, &mut settings, &*port_strategy);
        show_warning(main_window, port_strategy.msg_refreshed_reconnect());
    });
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

fn set_pitch_tables_model(main_window: &MainWindow) {
    let pitch_table_items: Vec<ComboBoxItem> = tuner::pitch_table_nos()
        .iter()
        .map(|grid_no| ComboBoxItem { text: grid_no.to_string().into() })
        .collect();
    let model = Rc::new(TuningGridsModel(pitch_table_items));
    main_window.set_pitch_tables_model(slint::ModelRc::from(model));
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

fn show_pitchgrid_status(
    main_window: &MainWindow, message: impl Into<SharedString>, message_type: MessageType) {
    main_window.invoke_show_pitchgrid_status(message.into(), message_type);
}

fn show_pitchgrid_disconnected(main_window: &MainWindow) {
    show_pitchgrid_status(
        main_window, "Pitchgrid OSC is not connected. OSC must be enabled in Pitchgrid.", 
        MessageType::Error);
}

fn show_warning(main_window: &MainWindow, message: impl Into<SharedString>) {
    show_message(main_window, message, MessageType::Warning);
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
