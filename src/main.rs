// Prevent console window in addition to Slint window in Windows release builds when, e.g., starting the app via file manager. Ignored on other platforms.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod global;
mod midi;
mod midi_data;
mod midi_data_wrong;
mod settings;

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Mutex;
use lazy_static::lazy_static;
use slint::{CloseRequestResponse, SharedString, Weak};
use midi::Midi;
// use midi::{InputPort, Midi, OutputPort};

slint::include_modules!();

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

lazy_static! {
    static ref IS_CLOSE_ERROR_SHOWN: Mutex<bool> = Mutex::new(false);
}

fn main() {
    let main_window = MainWindow::new().unwrap();
    main_window.set_window_title(global::APP_TITLE.into());
    let midi: SharedMidi = Rc::new(RefCell::new(Midi::new()));
    init(&main_window, &midi);
    main_window.run().unwrap();
}

fn connect_initial_input_port(main_window: &MainWindow, midi: &SharedMidi) {
    println!("main.connect_initial_input_port: start");
    let maybe_index = midi.borrow().input_port().as_ref().map(|p| p.index());
    if let Some(index) = maybe_index {
        main_window.set_selected_input_port_index(index as i32);
        connect_selected_input_port(main_window, midi);
    } else {
        show_no_input_port_connected(main_window);
        show_warning(&main_window, MSG_CONNECT_INPUT);
    }
}

fn connect_initial_output_port(main_window: &MainWindow, midi: &SharedMidi) {
    let maybe_index =
        midi.borrow().output_port().as_ref().map(|p| p.index());
    if let Some(index) = maybe_index {
        main_window.set_selected_output_port_index(index as i32);
        connect_selected_output_port(main_window, midi);
    } else {
        show_no_output_port_connected(main_window);
        show_warning(&main_window, MSG_CONNECT_OUTPUT);
    }
}

fn connect_input_port(main_window_weak: Weak<MainWindow>, midi: &SharedMidi) {
    // println!("main.connect_input_port start");
    with_main_window(main_window_weak, |main_window| {
        connect_selected_input_port(main_window, midi);
        // println!("main.connect_input_port: after connect_selected_input_port");
        if let Some(port) = midi.borrow().input_port() {
            show_info(main_window, format!("Connected MIDI input port {}", port.name()));
            // println!("main.connect_input_port: showed 'Connected MIDI input port...'");
        }
    });
}

fn connect_output_port(main_window_weak: Weak<MainWindow>, midi: &SharedMidi) {
    with_main_window(main_window_weak, |main_window| {
        connect_selected_output_port(main_window, midi);
        if let Some(port) = midi.borrow().output_port() {
            show_info(main_window, format!("Connected MIDI output port {}", port.name()));
        }
    });
}

fn connect_selected_input_port(main_window: &MainWindow, midi: &SharedMidi) {
    // println!("main.connect_selected_input_port: start");
    let selected = main_window.get_selected_input_port_index();
    let index: usize = match usize::try_from(selected) {
        Ok(i) => i,
        Err(_) => {
            show_no_input_port_connected(main_window);
            show_error(main_window, MSG_NO_INPUT_SELECTED);
            return;
        }
    };
    // Do all Midi borrowing/mutation inside a tight scope, then update UI after.
    let ui_action: Result<String, String> = {
        let mut midi_mut = midi.borrow_mut();
        let Some(name) = midi_mut.input_port_names().get(index).cloned()
        else {
            return;
        };
        match midi_mut.connect_input_port(index) {
            Ok(()) => Ok(name),
            Err(err) => Err(err.to_string()),
        }
    };
    match ui_action {
        Ok(name) => {
            show_connected_input_port_name(main_window, &name);
        }
        Err(message) => {
            show_no_input_port_connected(main_window);
            show_error(main_window, message);
        }
    }
}

fn connect_selected_output_port(main_window: &MainWindow, midi: &SharedMidi) -> bool {
    let selected = main_window.get_selected_output_port_index();
    let index: usize = match usize::try_from(selected) {
        Ok(i) => i,
        Err(_) => {
            show_no_output_port_connected(main_window);
            show_error(main_window, MSG_NO_OUTPUT_SELECTED);
            return false;
        }
    };
    // Do all Midi borrowing/mutation inside a tight scope, then update UI after.
    let ui_action: Result<String, String> = {
        let mut midi_mut = midi.borrow_mut();
        let Some(name) = midi_mut.output_port_names().get(index).cloned()
        else {
            return false;
        };
        match midi_mut.connect_output_port(index) {
            Ok(()) => Ok(name),
            Err(err) => Err(err.to_string()),
        }
    };
    match ui_action {
        Ok(name) => {
            show_connected_output_port_name(main_window, &name);
            true
        }
        Err(message) => {
            show_no_output_port_connected(main_window);
            show_error(main_window, message);
            false
        }
    }
}

fn handle_close_request(main_window_weak: Weak<MainWindow>, midi: &SharedMidi) -> CloseRequestResponse {
    let mut response = CloseRequestResponse::HideWindow;
    if *IS_CLOSE_ERROR_SHOWN.lock().unwrap() {
        // If a close error message is already shown, allow the window to be closed.
        return response
    }
    with_main_window(main_window_weak, |main_window| {
        if let Err(err) = midi.borrow_mut().close() {
            response = CloseRequestResponse::KeepWindowShown;
            show_error(main_window, err.to_string());
            *IS_CLOSE_ERROR_SHOWN.lock().unwrap() = true;
        }
    });
    response
}

fn init(main_window: &MainWindow, midi: &SharedMidi) {
    if let Err(err) = midi.borrow_mut().init() {
        show_error(main_window, err.to_string());
        return;
    }
    set_input_ports_model(&main_window, midi);
    set_output_ports_model(&main_window, midi);
    connect_initial_input_port(&main_window, &midi);
    connect_initial_output_port(&main_window, &midi);
    let midi2 = midi.borrow();
    if midi2.output_port().is_none() {
        if midi2.input_port().is_none() {
            show_warning(&main_window, MSG_CONNECT_BOTH);
        } else {
            main_window.invoke_focus_output_port();
        }
    }
    init_ui_handlers(&main_window, Rc::clone(&midi));
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
            connect_input_port(window_weak.clone(), &midi)
        });
    }
    {
        let mut midi: SharedMidi = Rc::clone(&midi);
        let window_weak = window_weak.clone();
        main_window.on_refresh_input_ports(move || {
            refresh_input_ports(window_weak.clone(), &mut midi)
        });
    }
    {
        let midi: SharedMidi = Rc::clone(&midi);
        let window_weak = window_weak.clone();
        main_window.on_connect_output_port(move || {
            connect_output_port(window_weak.clone(), &midi)
        });
    }
    {
        let mut midi: SharedMidi = Rc::clone(&midi);
        let window_weak = window_weak.clone();
        main_window.on_refresh_output_ports(move || {
            refresh_output_ports(window_weak.clone(), &mut midi)
        });
    }
}

fn refresh_input_ports(
    main_window_weak: Weak<MainWindow>, midi: &SharedMidi) {
    with_main_window(main_window_weak, |main_window| {
        if let Err(err) = midi.borrow_mut().refresh_input_ports() {
            show_error(main_window, err.to_string());
            return;
        }
        set_input_ports_model(&main_window, midi);
        show_no_input_port_connected(main_window);
        show_warning(main_window, MSG_REFRESHED_INPUTS_RECONNECT);
    });
}

fn refresh_output_ports(
    main_window_weak: Weak<MainWindow>, midi: &SharedMidi) {
    with_main_window(main_window_weak, |main_window| {
        if let Err(err) = midi.borrow_mut().refresh_output_ports() {
            show_error(main_window, err.to_string());
            return;
        }
        set_output_ports_model(&main_window, midi);
        show_no_output_port_connected(main_window);
        show_warning(main_window, MSG_REFRESHED_OUTPUTS_RECONNECT);
    });
}

fn set_input_ports_model(main_window: &MainWindow, midi: &SharedMidi) {
    let input_port_items: Vec<ComboBoxItem> = midi.borrow().input_port_names()
        .iter()
        .map(|text| ComboBoxItem { text: text.into() })
        .collect();
    let model = Rc::new(InputPortsModel(input_port_items));
    main_window.set_input_ports_model(slint::ModelRc::from(model));
}

fn set_output_ports_model(main_window: &MainWindow, midi: &SharedMidi) {
    let output_port_items: Vec<ComboBoxItem> = midi.borrow().output_port_names()
        .iter()
        .map(|text| ComboBoxItem { text: text.into() })
        .collect();
    let model = Rc::new(OutputPortsModel(output_port_items));
    main_window.set_output_ports_model(slint::ModelRc::from(model));
}

fn show_connected_input_port_name(main_window: &MainWindow, port_name: &str) {
    let message_type = if port_name == PORT_NONE {
        MessageType::Warning }
    else {
        MessageType::Info
    };
    main_window.invoke_show_connected_input_port_name(port_name.into(), message_type);
}

fn show_connected_output_port_name(main_window: &MainWindow, port_name: &str) {
    let message_type = if port_name == PORT_NONE {
        MessageType::Warning }
    else {
        MessageType::Info
    };
    main_window.invoke_show_connected_output_port_name(port_name.into(), message_type);
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

fn show_no_input_port_connected(main_window: &MainWindow) {
    show_connected_input_port_name(main_window, PORT_NONE);
}

fn show_no_output_port_connected(main_window: &MainWindow) {
    show_connected_output_port_name(main_window, PORT_NONE);
}

fn show_warning(main_window: &MainWindow, message: impl Into<SharedString>) {
    show_message(main_window, message, MessageType::Warning);
}

// fn update_input_ports_or_show_error(
//     main_window: &MainWindow,
//     midi: &SharedMidiManager,
// ) -> Option<InputPortsData> {
//     match midi.borrow_mut().update_input_ports() {
//         Ok(data) => Some(data),
//         Err(err) => {
//             show_error(main_window, err.to_string());
//             None
//         }
//     }
// }

// fn update_output_ports_or_show_error(
//     main_window: &MainWindow,
//     midi: &SharedMidiManager,
// ) -> Option<OutputPortsData> {
//     match midi.borrow_mut().update_output_ports() {
//         Ok(data) => Some(data),
//         Err(err) => {
//             show_error(main_window, err.to_string());
//             None
//         }
//     }
// }

fn with_main_window(main_window_weak: Weak<MainWindow>, f: impl FnOnce(&MainWindow)) {
    if let Some(main_window) = main_window_weak.upgrade() {
        f(&main_window);
    }
}
