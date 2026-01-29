// Prevent console window in addition to Slint window in Windows release builds when, e.g., starting the app via file manager. Ignored on other platforms.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod global;
mod midi;
mod settings;

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Mutex;
use lazy_static::lazy_static;
use slint::{CloseRequestResponse, SharedString, Weak};
use midi::{MidiManager, OutputPortsData};
slint::include_modules!();

/// 'Rc<RefCell<MidiManager>>' gives **shared ownership** ('Rc')
/// + **interior mutability** ('RefCell'), so multiple closures can mutate
/// the same manager safely (single-threaded UI context).
/// If you later move MIDI work off the UI thread, you’ll want 'Arc<Mutex<_>>' instead.
/// But for Slint’s typical single-threaded event loop, 'Rc<RefCell<_>>' is the right fix.
type SharedMidiManager = Rc<RefCell<MidiManager>>;

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

const MSG_CONNECT: &str = "Connect to a MIDI output port.";
const MSG_REFRESHED_OUTPUTS_RECONNECT: &str = "Refreshed MIDI output ports. You must (re)connect.";
const PORT_NONE: &str = "[None]";

lazy_static! {
    static ref IS_CLOSE_ERROR_SHOWN: Mutex<bool> = Mutex::new(false);
}

fn main() {
    let main_window = MainWindow::new().unwrap();
    main_window.set_midi_output_title("To Haken Continuum or EaganMatrix Module".into());
    main_window.set_window_title(global::APP_TITLE.into());
    let mut midi: SharedMidiManager = Rc::new(RefCell::new(MidiManager::new()));
    init_output_ports(&main_window, &mut midi);
    init_midi_ui_handlers(&main_window, Rc::clone(&midi));
    main_window.run().unwrap();
}

fn connect_output_port(main_window_weak: Weak<MainWindow>, midi: &SharedMidiManager) {
    with_main_window(main_window_weak, |main_window| {
        connect_selected_output_port(main_window, midi);
    });
}

fn connect_selected_output_port(main_window: &MainWindow, midi: &SharedMidiManager) {
    let index = main_window.get_selected_output_port_index() as usize;
    let mut midi_manager = midi.borrow_mut();
    let output_port_names = midi_manager.get_output_port_names();
    let Some(name) = output_port_names.get(index)
    else {
        show_no_output_port_connected(main_window);
        show_error(
            main_window,
            format!("No MIDI output port at index {}.", index),
        );
        return;
    };
    match midi_manager.connect_output_port(index) {
        Ok(()) => {
            show_connected_output_port_name(main_window, name);
            show_info(main_window, format!("Connected to MIDI output port {name}"));
        }
        Err(err) => {
            show_no_output_port_connected(main_window);
            show_error(main_window, err.to_string());
        }
    }
}

fn handle_close_request(main_window_weak: Weak<MainWindow>, midi: &SharedMidiManager) -> CloseRequestResponse {
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

fn init_midi_ui_handlers(main_window: &MainWindow, midi: SharedMidiManager) {
    let window_weak = main_window.as_weak();
    {
        let mut midi: SharedMidiManager = Rc::clone(&midi);
        let window_weak = window_weak.clone();
        main_window.window().on_close_requested(move || {
            handle_close_request(window_weak.clone(), &mut midi)
        });
    }
    {
        let midi: SharedMidiManager = Rc::clone(&midi);
        let window_weak = window_weak.clone();
        main_window.on_connect_output_port(move || {
            connect_output_port(window_weak.clone(), &midi)
        });
    }
    {
        let mut midi: SharedMidiManager = Rc::clone(&midi);
        let window_weak = window_weak.clone();
        main_window.on_refresh_output_ports(move || {
            refresh_output_ports(window_weak.clone(), &mut midi)
        });
    }
}

fn init_output_ports(main_window: &MainWindow, midi: &SharedMidiManager) {
    let Some(output_ports_data) = update_output_ports_or_show_error(
        main_window, midi)
    else {
        return;
    };
    set_output_ports_model(&main_window, output_ports_data.get_port_names());
    if let Some(persisted_port) = output_ports_data.get_persisted_port() {
        let index = persisted_port.get_index();
        main_window.set_selected_output_port_index(index as i32);
        connect_selected_output_port(main_window, midi);
    } else {
        show_warning(&main_window, MSG_CONNECT);
    }
}

fn refresh_output_ports(
    main_window_weak: Weak<MainWindow>, midi: &SharedMidiManager) {
    with_main_window(main_window_weak, |main_window| {
        let Some(output_ports_data) = update_output_ports_or_show_error(
            main_window, midi)
        else {
            return;
        };
        set_output_ports_model(&main_window, output_ports_data.get_port_names());
        show_no_output_port_connected(main_window);
        show_warning(main_window, MSG_REFRESHED_OUTPUTS_RECONNECT);
    });
}

fn set_output_ports_model(main_window: &MainWindow, port_names: &[String]) {
    let output_port_items: Vec<ComboBoxItem> = port_names
        .iter()
        .map(|text| ComboBoxItem { text: text.into() })
        .collect();
    let model = Rc::new(OutputPortsModel(output_port_items));
    main_window.set_output_ports_model(slint::ModelRc::from(model));
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

fn show_no_output_port_connected(main_window: &MainWindow) {
    show_connected_output_port_name(main_window, PORT_NONE);
}

fn show_warning(main_window: &MainWindow, message: impl Into<SharedString>) {
    show_message(main_window, message, MessageType::Warning);
}

fn update_output_ports_or_show_error(
    main_window: &MainWindow,
    midi: &SharedMidiManager,
) -> Option<OutputPortsData> {
    match midi.borrow_mut().update_output_ports() {
        Ok(data) => Some(data),
        Err(err) => {
            show_error(main_window, err.to_string());
            None
        }
    }
}

fn with_main_window(main_window_weak: Weak<MainWindow>, f: impl FnOnce(&MainWindow)) {
    if let Some(main_window) = main_window_weak.upgrade() {
        f(&main_window);
    }
}
