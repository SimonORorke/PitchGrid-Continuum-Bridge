// Prevent console window in addition to Slint window in Windows release builds when, e.g., starting the app via file manager. Ignored on other platforms.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod global;
mod midi;
mod midi_ports;
mod osc;
mod settings;
mod tuner;

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use lazy_static::lazy_static;
use round::round;
use slint::{CloseRequestResponse, SharedString, Weak};
use midi::{Midi, PortType};
use crate::midi_ports::MidiIo;
use crate::osc::Osc;

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
    main_window.set_window_title(global::APP_TITLE.into());
    let midi: SharedMidi = Rc::new(RefCell::new(Midi::new()));
    init(&main_window, &midi);
    main_window.run().unwrap();
}
fn connect_initial_port(main_window: &MainWindow, midi: &SharedMidi, port_type: &PortType) {
    let maybe_index = midi.borrow().io(port_type).port().as_ref()
        .map(|port| port.index());
    if let Some(index) = maybe_index {
        let index = index as i32;
        match port_type {
            PortType::Input => main_window.set_selected_input_port_index(index),
            PortType::Output => main_window.set_selected_output_port_index(index),
        }
        connect_selected_port(main_window, midi, port_type);
    } else {
        show_no_port_connected(main_window, port_type);
        let msg = match port_type {
            PortType::Input => MSG_CONNECT_INPUT,
            PortType::Output => MSG_CONNECT_OUTPUT,
        };
        show_warning(&main_window, msg);
    }
}

fn connect_port(main_window_weak: Weak<MainWindow>, midi: &SharedMidi, port_type: &PortType) {
    with_main_window(main_window_weak, |main_window| {
        connect_selected_port(main_window, midi, port_type);
        let port_type_name = match port_type {
            PortType::Input => "input",
            PortType::Output => "output",
        };
        if let Some(port) = midi.borrow().io(port_type).port() {
            show_info(main_window, format!("Connected MIDI {} port {}",
                                           port_type_name, port.name()));
        }
    });
}
fn connect_selected_port(main_window: &MainWindow, midi: &SharedMidi, port_type: &PortType) {
    let selected = match port_type {
        PortType::Input => main_window.get_selected_input_port_index(),
        PortType::Output => main_window.get_selected_output_port_index(),
    };
    let index: usize = match usize::try_from(selected) {
        Ok(i) => i,
        Err(_) => {
            show_no_port_connected(main_window, port_type);
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
        let mut midi_mut = midi.borrow_mut();
        let Some(name) = midi_mut.io(port_type).port_names().get(index).cloned()
        else {
            return;
        };
        match midi_mut.connect_port(port_type, index) {
            Ok(()) => Ok(name),
            Err(err) => Err(err.to_string()),
        }
    };
    match ui_action {
        Ok(name) => {
            show_connected_port_name(main_window, &name, port_type);
        }
        Err(message) => {
            show_no_port_connected(main_window, port_type);
            show_error(main_window, message);
        }
    }
}

fn handle_close_request(main_window_weak: Weak<MainWindow>, midi: &SharedMidi) -> CloseRequestResponse {
    let mut response = CloseRequestResponse::HideWindow;
    let mut data = DATA.lock().unwrap();
    if data.is_close_error_shown.load(Ordering::Relaxed) {
        // If a close error message is already shown, allow the window to be closed.
        return response
    }
    with_main_window(main_window_weak, |main_window| {
        if let Err(err) = midi.borrow_mut().close() {
            response = CloseRequestResponse::KeepWindowShown;
            show_error(main_window, err.to_string());
            data.is_close_error_shown.store(true, Ordering::Relaxed);
        }
    });
    data.osc.stop();
    response
}

fn init(main_window: &MainWindow, midi: &SharedMidi) {
    if let Err(err) = midi.borrow_mut().init() {
        show_error(main_window, err.to_string());
        return;
    }
    set_ports_model(&main_window, midi, &PortType::Input);
    set_ports_model(&main_window, midi, &PortType::Output);
    connect_initial_port(&main_window, midi, &PortType::Input);
    connect_initial_port(&main_window, midi, &PortType::Output);
    let midi2 = midi.borrow();
    if midi2.output().port().is_none() {
        if midi2.input().port().is_none() {
            show_warning(&main_window, MSG_CONNECT_BOTH);
        } else {
            main_window.invoke_focus_output_port();
        }
    }
    init_ui_handlers(&main_window, Rc::clone(&midi));
    show_pitchgrid_disconnected(&main_window);
    let mut data = DATA.lock().unwrap();
    data.main_window_weak = Some(main_window.as_weak().clone());
    data.osc.start(Arc::new(on_osc_tuning_received), Arc::new(on_osc_connected_changed));
}

fn init_ui_handlers(main_window: &MainWindow, midi: SharedMidi) {
    let window_weak = main_window.as_weak();
    {
        let mut midi: SharedMidi = Rc::clone(&midi);
        let window_weak = window_weak.clone();
        main_window.window().on_close_requested(move || {
            handle_close_request(window_weak.clone(), &mut midi)
        });
    }
    {
        let midi: SharedMidi = Rc::clone(&midi);
        let window_weak = window_weak.clone();
        main_window.on_connect_input_port(move || {
            connect_port(window_weak.clone(), &midi, &PortType::Input)
        });
    }
    {
        let mut midi: SharedMidi = Rc::clone(&midi);
        let window_weak = window_weak.clone();
        main_window.on_refresh_input_ports(move || {
            refresh_ports(window_weak.clone(), &mut midi, &PortType::Input)
        });
    }
    {
        let midi: SharedMidi = Rc::clone(&midi);
        let window_weak = window_weak.clone();
        main_window.on_connect_output_port(move || {
            connect_port(window_weak.clone(), &midi, &PortType::Output)
        });
    }
    {
        let mut midi: SharedMidi = Rc::clone(&midi);
        let window_weak = window_weak.clone();
        main_window.on_refresh_output_ports(move || {
            refresh_ports(window_weak.clone(), &mut midi, &PortType::Output)
        });
    }
}

fn on_osc_connected_changed() {
    let data = DATA.lock().unwrap();
    println!("main.on_osc_connected_changed: Connected = {}", data.osc.is_connected());
    if let Some(main_window_weak) = &data.main_window_weak {
        println!("main.on_osc_connected_changed: Found main_window_weak");
        with_main_window(main_window_weak.clone(), |main_window| {
            println!("main.on_osc_connected_changed: Found main_window");
            if data.osc.is_connected() {
                show_pitchgrid_connected(main_window);
            } else {
                show_pitchgrid_disconnected(main_window);
            }
        });
    }
}

fn on_osc_tuning_received(depth: i32, mode: i32, root_freq: f32, stretch: f32,
                          skew: f32, mode_offset: i32, steps: i32) {
    tuner::on_tuning_changed(depth, mode, root_freq, stretch, skew, mode_offset, steps);
    println!("main.on_osc_tuning_received");
    let data = DATA.lock().unwrap();
    if let Some(main_window_weak) = &data.main_window_weak {
        with_main_window(main_window_weak.clone(), |main_window| {
            main_window.set_depth(format!("{depth}").into());
            main_window.set_root_freq(format!("{} Hz", round(root_freq as f64, 2)).into());
            main_window.set_stretch(format!("{} ct", stretch.round()).into());
            main_window.set_skew(format!("{}", round(skew as f64, 2)).into());
            main_window.set_mode_offset(format!("{mode_offset}").into());
            main_window.set_steps(format!("{steps}").into());
        });
    }
}

fn refresh_ports(
    main_window_weak: Weak<MainWindow>, midi: &SharedMidi, port_type: &PortType) {
    with_main_window(main_window_weak, |main_window| {
        if let Err(err) = midi.borrow_mut().refresh_ports(port_type) {
            show_error(main_window, err.to_string());
            return;
        }
        set_ports_model(&main_window, midi, port_type);
        show_no_port_connected(main_window, port_type);
        let msg = match port_type {
            PortType::Input => MSG_REFRESHED_INPUTS_RECONNECT,
            PortType::Output => MSG_REFRESHED_OUTPUTS_RECONNECT,
        };
        show_warning(main_window, msg);
    });
}

fn set_ports_model(main_window: &MainWindow, midi: &SharedMidi, port_type: &PortType) {
    let port_items: Vec<ComboBoxItem> = midi.borrow().io(port_type).port_names()
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

fn show_connected_port_name(main_window: &MainWindow, port_name: &str, port_type: &PortType) {
    let message_type = if port_name == PORT_NONE {
        MessageType::Warning }
    else {
        MessageType::Info
    };
    match port_type {
        PortType::Input => 
            main_window.invoke_show_connected_input_port_name(port_name.into(), message_type),
        PortType::Output => 
            main_window.invoke_show_connected_output_port_name(port_name.into(), message_type),
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

fn show_no_port_connected(main_window: &MainWindow, port_type: &PortType) {
    show_connected_port_name(main_window, PORT_NONE, port_type);
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

fn with_main_window(main_window_weak: Weak<MainWindow>, f: impl FnOnce(&MainWindow)) {
    println!("main.with_main_window start");
    if let Some(main_window) = main_window_weak.upgrade() {
        println!("main.with_main_window: Found main_window");
        f(&main_window);
    }
}

/// 'Rc<RefCell<MidiManager>>' gives **shared ownership** ('Rc')
/// + **interior mutability** ('RefCell'), so multiple closures can mutate
/// the same manager safely (single-threaded UI context).
/// If you later move MIDI work off the UI thread, you’ll want 'Arc<Mutex<_>>' instead.
/// But for Slint’s typical single-threaded event loop, 'Rc<RefCell<_>>' is the right fix.
type SharedMidi = Rc<RefCell<Midi>>;

struct InputPortsModel(Vec<ComboBoxItem>);

impl slint::Model for InputPortsModel {
    type Data = ComboBoxItem;
    fn row_count(&self) -> usize {
        self.0.len()
    }
    fn row_data(&self, row: usize) -> Option<Self::Data> {
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
    fn row_data(&self, row: usize) -> Option<Self::Data> {
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
