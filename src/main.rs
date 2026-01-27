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
use midi::MidiManager;
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

lazy_static! {
    static ref has_close_error: Mutex<bool> = Mutex::new(false);
}

fn main() {
    let main_window = MainWindow::new().unwrap();
    main_window.set_window_title(global::APP_TITLE.into());
    let mut midi: SharedMidiManager = Rc::new(RefCell::new(MidiManager::new()));
    init_output_ports(&main_window, &mut midi);
    init_midi_ui_handlers(&main_window, Rc::clone(&midi));
    main_window.run().unwrap();
}

fn close(main_window_weak: Weak<MainWindow>, midi: &mut SharedMidiManager) -> CloseRequestResponse {
    let mut response = CloseRequestResponse::HideWindow;
    if *has_close_error.lock().unwrap() {
        // If a close error message is already shown, allow the window to be closed.
        return response
    }
    with_main_window(main_window_weak, |main_window| {
        if let Err(err) = midi.borrow_mut().close() {
            response = CloseRequestResponse::KeepWindowShown;
            show_error(main_window, format!("Error: {}", err));
            *has_close_error.lock().unwrap() = true;
        }
    });
    response
}

fn connect_to_output_port(main_window_weak: Weak<MainWindow>, midi: &SharedMidiManager) {
    with_main_window(main_window_weak, |main_window| {
        let mut midi_manager = midi.borrow_mut();
        let output_port_names = midi_manager.get_output_port_names();
        let index = main_window.get_selected_output_port_index() as usize;
        if let Some(name) = output_port_names.get(index) {
            match midi_manager.connect_to_output_port(index) {
                Ok(_) =>
                    show_info(main_window, format!("Connected to MIDI output port {name}")),
                Err(err) => {
                    show_warning(&main_window, MSG_CONNECT);
                    show_error(main_window, format!("Error: {}", err));
                },
            }
        }
    });
}

fn init_midi_ui_handlers(main_window: &MainWindow, midi: SharedMidiManager) {
    let window_weak = main_window.as_weak();
    {
        let mut midi: SharedMidiManager = Rc::clone(&midi);
        let window_weak = window_weak.clone();
        main_window.window().on_close_requested(move || {
            close(window_weak.clone(), &mut midi)
        });
    }
    {
        let midi: SharedMidiManager = Rc::clone(&midi);
        let window_weak = window_weak.clone();
        main_window.on_connect_to_output_port(move || {
            connect_to_output_port(window_weak.clone(), &midi)
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

fn init_output_ports(main_window: &MainWindow, midi: &mut SharedMidiManager) {
    let output_ports_data = midi.borrow_mut().update_output_ports();
    if let Err(err) = output_ports_data {
        show_error(main_window, err.to_string());
        return;
    }
    let output_ports_data = output_ports_data.unwrap();
    set_output_ports(&main_window, output_ports_data.get_port_names());
    if let Some(persisted_port) = output_ports_data.get_persisted_port() {
        let index = persisted_port.get_index();
        match midi.borrow_mut().connect_to_output_port(index) {
            Ok(_) => {
                main_window.set_selected_output_port_index(index as i32);
                show_info(main_window, format!("Connected to MIDI output port {}",
                                               persisted_port.get_name()));
            }
            Err(err) =>
                show_error(main_window, format!("Error: {}", err)),
        }
    } else {
        show_warning(&main_window, MSG_CONNECT);
    }
}

fn refresh_output_ports(
    main_window_weak: Weak<MainWindow>, midi: &mut SharedMidiManager) {
    with_main_window(main_window_weak, |main_window| {
        let output_ports_data =
            midi.borrow_mut().update_output_ports();
        if let Err(err) = output_ports_data {
            show_error(main_window, err.to_string());
            return;
        }
        let output_ports_data = output_ports_data.unwrap();
        set_output_ports(&main_window, output_ports_data.get_port_names());
        show_warning(main_window, MSG_REFRESHED_OUTPUTS_RECONNECT);
    });
}

fn set_output_ports(main_window: &MainWindow, port_names: &Vec<String>) {
    let output_port_items: Vec<ComboBoxItem> = port_names
        .into_iter()
        .map(|text| ComboBoxItem { text: text.into() })
        .collect();
    let model = Rc::new(OutputPortsModel(output_port_items));
    main_window.set_output_ports_model(slint::ModelRc::from(model));
}

fn show_error(main_window: &MainWindow, message: impl Into<SharedString>) {
    main_window.invoke_show_message(message.into(), MessageType::Error);
}

fn show_info(main_window: &MainWindow, message: impl Into<SharedString>) {
    main_window.invoke_show_message(message.into(), MessageType::Info);
}

fn show_warning(main_window: &MainWindow, message: impl Into<SharedString>) {
    main_window.invoke_show_message(message.into(), MessageType::Warning);
}

fn with_main_window(main_window_weak: Weak<MainWindow>, f: impl FnOnce(&MainWindow)) {
    if let Some(main_window) = main_window_weak.upgrade() {
        f(&main_window);
    }
}
