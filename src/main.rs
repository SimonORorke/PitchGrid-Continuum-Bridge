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
use round::round;
use slint::{CloseRequestResponse, SharedString, Weak};
use midi::{ConnectionTo, Midi, PortType};
use crate::global::APP_TITLE;
use crate::midi_ports::MidiIo;
use crate::osc::Osc;
use crate::port_strategy::{
    EditorInputStrategy, EditorOutputStrategy, 
    InstrumentInputStrategy, InstrumentOutputStrategy, PortStrategy};
use crate::settings::Settings;

slint::include_modules!();

/// Giving separate names to this shared data struct and those in modules
/// and also to the static refs that use them works around a problem with RustRover where
/// Linter would sometimes falsely indicate compiler errors.
struct MainData {
    pub is_close_error_shown: Arc<AtomicBool>,
    pub main_window_weak: Option<Weak<MainWindow>>,
    pub osc: Osc,
}

lazy_static! {
    static ref MAIN_DATA: Mutex<MainData> = Mutex::new(MainData {
        is_close_error_shown: Arc::new(AtomicBool::new(false)),
        main_window_weak: None,
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
    main_window: &MainWindow, midi: &mut SharedMidi,
    settings: &mut SharedSettings, port_strategy: &dyn PortStrategy) {
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
    }
}

fn connect_port(main_window_weak: Weak<MainWindow>, midi: &mut SharedMidi,
                settings: &mut SharedSettings, port_strategy: &(dyn PortStrategy + Send + Sync)) {
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

fn connect_selected_port(main_window: &MainWindow, midi: &mut SharedMidi,
                         settings: &mut SharedSettings, port_strategy: &dyn PortStrategy) {
    // println!("main.connect_selected_port");
    let selected = port_strategy.get_selected_port_index(main_window);
    let index: usize = match usize::try_from(selected) {
        Ok(i) => i,
        Err(_) => {
            show_no_port_connected(main_window, settings, port_type);
            // let msg = match connection_to {
            //
            // };
            let msg = match port_type {
                PortType::Input => MSG_NO_INPUT_SELECTED,
                PortType::Output => MSG_NO_OUTPUT_SELECTED,
            };
            show_error(main_window, msg);
            return;
        }
    };
    // Do all Midi borrowing/mutation inside a tight scope, then update UI after.
    let ui_action: Result<String, String> = {
        let mut midi1 = midi.lock().unwrap();
        let Some(name) = midi1.io(connection_to, port_type).port_names().get(index).cloned()
        else {
            return;
        };
        match midi1.connect_port(index, connection_to, port_type) {
            Ok(()) => Ok(name),
            Err(err) => Err(err.to_string()),
        }
    };
    match ui_action {
        Ok(name) => {
            show_connected_port_name(main_window, settings, &name, port_type);
        }
        Err(message) => {
            show_no_port_connected(main_window, settings, port_type);
            show_error(main_window, message);
        }
    }
}

fn handle_close_request(
        main_window_weak: Weak<MainWindow>, midi: &SharedMidi,
        settings: &mut SharedSettings) -> CloseRequestResponse {
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

fn init(main_window: &MainWindow, midi: &mut SharedMidi, settings: &mut SharedSettings) {
    // println!("main.init");
    let pitch_table_no: u8;
    {
        let mut settings1 = settings.lock().unwrap();
        let editor_input_port_name: String;
        let editor_output_port_name: String;
        let instru_input_port_name: String;
        let instr_output_port_name: String;
        match settings1.read_from_file() {
            Ok(_) => {
                editor_input_port_name = settings1.editor_midi_input_port.clone();
                editor_output_port_name = settings1.editor_midi_output_port.clone();
                instru_input_port_name = settings1.instrument_midi_input_port.clone();
                instr_output_port_name = settings1.instrument_midi_output_port.clone();
                pitch_table_no = max(tuner::default_pitch_table_no(), settings1.pitch_table);
            }
            Err(err) => {
                show_error(main_window, err.to_string());
                return;
            }
        }
        let mut midi1 = midi.lock().unwrap();
        if let Err(err) = midi1.init(
            &editor_input_port_name, &editor_output_port_name,
            &instru_input_port_name, &instr_output_port_name) {
            show_error(main_window, err.to_string());
            return;
        }
    }
    let editor_input_strategy = EditorInputStrategy::new();
    let editor_output_strategy = EditorOutputStrategy::new();
    let instru_input_strategy = InstrumentInputStrategy::new();
    let instru_output_strategy = InstrumentOutputStrategy::new();
    set_ports_model(&main_window, midi, &editor_input_strategy);
    set_ports_model(&main_window, midi, &editor_output_strategy);
    set_ports_model(&main_window, midi, &instru_input_strategy);
    set_ports_model(&main_window, midi, &instru_output_strategy);
    connect_initial_port(&main_window, midi, settings, &editor_input_strategy);
    connect_initial_port(&main_window, midi, settings, &editor_output_strategy);
    connect_initial_port(&main_window, midi, settings, &instru_input_strategy);
    connect_initial_port(&main_window, midi, settings, &instru_output_strategy);
    set_pitch_tables_model(&main_window);
    tuner::set_pitch_table_no(pitch_table_no);
    main_window.set_selected_pitch_table_index(tuner::pitch_table_index() as i32);
    {
        // println!("main.init: Showing warning if no MIDI ports are connected.");
        let midi1 = midi.lock().unwrap();
        if midi1.output().port().is_none() {
            if midi1.input().port().is_none() {
                show_warning(&main_window, MSG_CONNECT_BOTH);
            } else {
                main_window.invoke_focus_output_port();
            }
        } else if midi1.input().port().is_some() {
            main_window.invoke_focus_update_tuning_button();
        }
    }
    init_ui_handlers(&main_window, Arc::clone(&midi), Arc::clone(settings));
    let mut data = MAIN_DATA.lock().unwrap();
    data.main_window_weak = Some(main_window.as_weak().clone());
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
        main_window.on_connect_input_port(move || {
            connect_port(window_weak.clone(), &mut midi, &mut settings, &PortType::Input)
        });
    }
    {
        let mut midi: SharedMidi = Arc::clone(&midi);
        let mut settings: SharedSettings = Arc::clone(&settings);
        let window_weak = window_weak.clone();
        main_window.on_refresh_input_ports(move || {
            refresh_ports(window_weak.clone(), &mut midi, &mut settings, &PortType::Input)
        });
    }
    {
        let mut midi: SharedMidi = Arc::clone(&midi);
        let mut settings: SharedSettings = Arc::clone(&settings);
        let window_weak = window_weak.clone();
        main_window.on_connect_output_port(move || {
            connect_port(window_weak.clone(), &mut midi, &mut settings, &PortType::Output)
        });
    }
    {
        let mut midi: SharedMidi = Arc::clone(&midi);
        let mut settings: SharedSettings = Arc::clone(&settings);
        let window_weak = window_weak.clone();
        main_window.on_refresh_output_ports(move || {
            refresh_ports(window_weak.clone(), &mut midi, &mut settings, &PortType::Output)
        });
    }
    main_window.on_update_tuning(move || {
        tuner::update_tuning()
    });
    {
        let mut settings: SharedSettings = Arc::clone(&settings);
        main_window.on_selected_pitch_table_changed(move |index| {
            update_pitch_table_no(index as usize, &mut settings)
        });
    }
}

fn update_pitch_table_no(index: usize, settings: &mut SharedSettings) {
    let pitch_table_no = tuner::pitch_table_nos()[index];
    tuner::set_pitch_table_no(pitch_table_no);
    settings.lock().unwrap().pitch_table = pitch_table_no;
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
    tuner::on_tuning_received(depth, mode, root_freq, stretch, skew, mode_offset, steps);
    let data = MAIN_DATA.lock().unwrap();
    if let Some(main_window_weak) = &data.main_window_weak {
        with_main_window(main_window_weak.clone(), move |main_window| {
            main_window.set_depth(format!("{depth}").into());
            main_window.set_root_freq(format!("{} Hz", round(root_freq as f64, 3)).into());
            // The stretch parameter is in octaves, so we need to multiply by 1200 to get the
            // number of cents to display.
            main_window.set_stretch(format!("{} ct", (stretch * 1200.0).round()).into());
            main_window.set_skew(format!("{}", round(skew as f64, 5)).into());
            main_window.set_mode_offset(format!("{mode_offset}").into());
            main_window.set_steps(format!("{steps}").into());
        });
    }
}

fn refresh_ports(
        main_window_weak: Weak<MainWindow>, midi: &SharedMidi, settings: &mut SharedSettings,
        port_type: &PortType) {
    let midi = Arc::clone(midi);
    let mut settings = Arc::clone(settings);
    let port_type = *port_type;
    let port_name = match port_type {
        PortType::Input => settings.lock().unwrap().instrument_midi_input_port.clone(),
        PortType::Output => settings.lock().unwrap().midi_output_port.clone(),
    };
    with_main_window(main_window_weak, move |main_window| {
        if let Err(err) = midi.lock().unwrap().refresh_ports(&port_name, &port_type) {
            show_error(main_window, err.to_string());
            return;
        }
        set_ports_model(&main_window, &midi, &port_type);
        show_no_port_connected(main_window, &mut settings, &port_type);
        let msg = match port_type {
            PortType::Input => MSG_REFRESHED_INPUTS_RECONNECT,
            PortType::Output => MSG_REFRESHED_OUTPUTS_RECONNECT,
        };
        show_warning(main_window, msg);
    });
}

fn set_ports_model(main_window: &MainWindow, midi: &SharedMidi,
                   port_strategy: &dyn PortStrategy) {
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

fn show_connected_port_name(main_window: &MainWindow, settings: &mut SharedSettings,
                            port_name: &str, port_strategy: &dyn PortStrategy) {
    let message_type = if port_name == PORT_NONE {
        MessageType::Warning }
    else {
        MessageType::Info
    };
    port_strategy.invoke_show_connected_port_name(main_window, port_name, message_type);
    let mut settings1 = settings.lock().unwrap();
    port_strategy.update_port_setting(&mut settings1, port_name);
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

fn show_no_port_connected(main_window: &MainWindow, settings: &mut SharedSettings,
                          port_strategy: &dyn PortStrategy) {
    show_connected_port_name(main_window, settings, PORT_NONE, port_strategy);
}

fn show_pitchgrid_connected(main_window: &MainWindow) {
    show_pitchgrid_connection_status(
        main_window, "Pitchgrid OSC is connected", MessageType::Info);
}

fn show_pitchgrid_connection_status(
    main_window: &MainWindow, message: impl Into<SharedString>, message_type: MessageType) {
    main_window.invoke_show_pitchgrid_connection_status(message.into(), message_type);
}

fn show_pitchgrid_disconnected(main_window: &MainWindow) {
    show_pitchgrid_connection_status(
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

type SharedMidi = Arc<Mutex<Midi>>;
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
