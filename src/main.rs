// Prevent console window in addition to Slint window in Windows release builds when, e.g., starting the app via file manager. Ignored on other platforms.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod global;
mod midi;
mod midi_ports;
mod osc;
mod settings;
mod tuner;

use std::cmp::max;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use lazy_static::lazy_static;
use round::round;
use slint::{CloseRequestResponse, SharedString, Weak};
use midi::{Midi, PortType};
use crate::global::APP_TITLE;
use crate::midi_ports::MidiIo;
use crate::osc::Osc;
use crate::settings::Settings;

slint::include_modules!();

struct Data {
    pub is_close_error_shown: Arc<AtomicBool>,
    pub main_window_weak: Option<Weak<MainWindow>>,
    pub osc: Osc,
}

lazy_static! {
    static ref DATA: Mutex<Data> = Mutex::new(Data {
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

fn connect_initial_port(main_window: &MainWindow, midi: &mut SharedMidi, settings: &mut SharedSettings, port_type: &PortType) {
    // println!("main.connect_initial_port");
    // We have to limit the scope of the lock, as midi will have to be locked again in
    // connect_selected_port.
    let maybe_index = {
        let midi1 = midi.lock().unwrap();
        // println!("main.connect_initial_port: Locked midi");
        midi1.io(port_type).port().as_ref()
            .map(|port| port.index())
    }; // Lock is dropped here

    if let Some(index) = maybe_index {
        let index = index as i32;
        match port_type {
            PortType::Input => main_window.set_selected_input_port_index(index),
            PortType::Output => main_window.set_selected_output_port_index(index),
        }
        connect_selected_port(main_window, midi, settings, port_type);
    } else {
        show_no_port_connected(main_window, settings, port_type);
        let msg = match port_type {
            PortType::Input => MSG_CONNECT_INPUT,
            PortType::Output => MSG_CONNECT_OUTPUT,
        };
        show_warning(&main_window, msg);
    }
}

fn connect_port(main_window_weak: Weak<MainWindow>, midi: &mut SharedMidi,
                settings: &mut SharedSettings, port_type: &PortType) {
    let port_type = *port_type;
    let mut midi = Arc::clone(midi);
    let mut settings = Arc::clone(settings);
    with_main_window(main_window_weak, move |main_window| {
        connect_selected_port(main_window, &mut midi, &mut settings, &port_type);
        let port_type_name = match port_type {
            PortType::Input => "input",
            PortType::Output => "output",
        };
        if let Some(port) = midi.lock().unwrap().io(&port_type).port() {
            show_info(main_window, format!("Connected MIDI {} port {}",
                                           port_type_name, port.name()));
        }
    });
}

fn connect_selected_port(main_window: &MainWindow, midi: &mut SharedMidi, 
                         settings: &mut SharedSettings, port_type: &PortType) {
    // println!("main.connect_selected_port");
    let selected = match port_type {
        PortType::Input => main_window.get_selected_input_port_index(),
        PortType::Output => main_window.get_selected_output_port_index(),
    };
    let index: usize = match usize::try_from(selected) {
        Ok(i) => i,
        Err(_) => {
            show_no_port_connected(main_window, settings, port_type);
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
        let Some(name) = midi1.io(port_type).port_names().get(index).cloned()
        else {
            return;
        };
        match midi1.connect_port(port_type, index) {
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
    let mut data = DATA.lock().unwrap();
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
    let tuning_grid_no: i32;
    {
        let mut settings1 = settings.lock().unwrap();
        let input_port_name: String;
        let output_port_name: String;
        match settings1.read_from_file() {
            Ok(_) => {
                input_port_name = settings1.midi_input_port.clone();
                output_port_name = settings1.midi_output_port.clone();
                tuning_grid_no = max(tuner::default_tuning_grid_no(), settings1.tuning_grid);
            }
            Err(err) => {
                show_error(main_window, err.to_string());
                return;
            }
        }
        let mut midi1 = midi.lock().unwrap();
        if let Err(err) = midi1.init(&input_port_name, &output_port_name) {
            show_error(main_window, err.to_string());
            return;
        }
    }
    set_ports_model(&main_window, midi, &PortType::Input);
    set_ports_model(&main_window, midi, &PortType::Output);
    connect_initial_port(&main_window, midi, settings, &PortType::Input);
    connect_initial_port(&main_window, midi, settings, &PortType::Output);
    set_tuning_grids_model(&main_window);
    tuner::set_tuning_grid_no(tuning_grid_no);
    main_window.set_selected_tuning_grid_index(tuner::tuning_grid_index() as i32);
    {
        // println!("main.init: Showing warning if no MIDI ports are connected.");
        let midi1 = midi.lock().unwrap();
        if midi1.output().port().is_none() {
            if midi1.input().port().is_none() {
                show_warning(&main_window, MSG_CONNECT_BOTH);
            } else {
                main_window.invoke_focus_output_port();
            }
        }
    }
    init_ui_handlers(&main_window, Arc::clone(&midi), Arc::clone(settings));
    let mut data = DATA.lock().unwrap();
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
    {
        let mut settings: SharedSettings = Arc::clone(&settings);
        main_window.on_selected_tuning_grid_changed(move |index| {
            update_tuning_grid_no(index as usize, &mut settings)
        });
    }
}

fn update_tuning_grid_no(index: usize, settings: &mut SharedSettings) {
    let tuning_grid_no = tuner::tuning_grid_nos()[index];
    tuner::set_tuning_grid_no(tuning_grid_no);
    settings.lock().unwrap().tuning_grid = tuning_grid_no;
}

fn on_osc_connected_changed() {
    let data = DATA.lock().unwrap();
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
    tuner::on_tuning_received(depth, mode, root_freq, stretch, skew, mode_offset, steps);
    // println!("main.on_osc_tuning_received");
    let data = DATA.lock().unwrap();
    if let Some(main_window_weak) = &data.main_window_weak {
        with_main_window(main_window_weak.clone(), move |main_window| {
            main_window.set_depth(format!("{depth}").into());
            main_window.set_root_freq(format!("{} Hz", round(root_freq as f64, 2)).into());
            // The stretch parameter is in octaves, so we need to multiply by 1200 to get the
            // number of cents to display.
            main_window.set_stretch(format!("{} ct", (stretch * 1200.0).round()).into());
            main_window.set_skew(format!("{}", round(skew as f64, 2)).into());
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
        PortType::Input => settings.lock().unwrap().midi_input_port.clone(),
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

fn set_ports_model(main_window: &MainWindow, midi: &SharedMidi, port_type: &PortType) {
    let port_items: Vec<ComboBoxItem> = midi.lock().unwrap().io(port_type).port_names()
        .iter()
        .map(|text| ComboBoxItem { text: text.into() })
        .collect();
    match port_type {
        PortType::Input => {
            let model = Rc::new(InputPortsModel(port_items));
            main_window.set_input_ports_model(slint::ModelRc::from(model));
        },
        PortType::Output => {
            let model = Rc::new(OutputPortsModel(port_items));
            main_window.set_output_ports_model(slint::ModelRc::from(model));
        },
    }
}

fn set_tuning_grids_model(main_window: &MainWindow) {
    let tuning_grid_items: Vec<ComboBoxItem> = tuner::tuning_grid_nos()
        .iter()
        .map(|grid_no| ComboBoxItem { text: grid_no.to_string().into() })
        .collect();
    let model = Rc::new(TuningGridsModel(tuning_grid_items));
    main_window.set_tuning_grids_model(slint::ModelRc::from(model));
}

fn show_connected_port_name(main_window: &MainWindow, settings: &mut SharedSettings, 
                            port_name: &str, port_type: &PortType) {
    let message_type = if port_name == PORT_NONE {
        MessageType::Warning }
    else {
        MessageType::Info
    };
    let mut settings1 = settings.lock().unwrap();
    match port_type {
        PortType::Input => {
            main_window.invoke_show_connected_input_port_name(port_name.into(), message_type);
            settings1.midi_input_port = port_name.into();
        }
        PortType::Output => {
            main_window.invoke_show_connected_output_port_name(port_name.into(), message_type);
            settings1.midi_output_port = port_name.into();
        }
    }
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
                          port_type: &PortType) {
    show_connected_port_name(main_window, settings, PORT_NONE, port_type);
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

const MSG_CONNECT_BOTH: &str = "Connect MIDI input and output ports.";
const MSG_CONNECT_INPUT: &str = "Connect a MIDI input port.";
const MSG_NO_INPUT_SELECTED: &str = "No MIDI input port selected.";
const MSG_REFRESHED_INPUTS_RECONNECT: &str = "Refreshed MIDI input ports. You must (re)connect.";
const MSG_CONNECT_OUTPUT: &str = "Connect a MIDI output port.";
const MSG_NO_OUTPUT_SELECTED: &str = "No MIDI output port selected.";
const MSG_REFRESHED_OUTPUTS_RECONNECT: &str = "Refreshed MIDI output ports. You must (re)connect.";
const PORT_NONE: &str = "[None]";
