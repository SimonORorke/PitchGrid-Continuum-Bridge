// Prevent console window in addition to Slint window in Windows release builds when, e.g., starting the app via file manager. Ignored on other platforms.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod midi;

use std::cell::RefCell;
use std::rc::Rc;
use slint::{SharedString, Weak};
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

const MSG_CONNECT: &str = "Connect to a MIDI output port";
const MSG_REFRESHED_OUTPUTS: &str = "Refreshed MIDI output ports. You must (re)connect.";
const WINDOW_TITLE: &str = "PitchGrid-Continuum Companion";

fn main() {
    let main_window = MainWindow::new().unwrap();
    main_window.set_window_title(WINDOW_TITLE.into());
    let mut midi: SharedMidiManager = Rc::new(RefCell::new(MidiManager::new()));
    set_output_ports(&main_window, &mut midi);
    show_warning(&main_window, MSG_CONNECT);
    init_midi_ui_handlers(&main_window, Rc::clone(&midi));
    main_window.run().unwrap();
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
                Err(err) =>
                    show_error(main_window, format!("Error: {}", err)),
            }
        }
    });
}

fn init_midi_ui_handlers(main_window: &MainWindow, midi: SharedMidiManager) {
    let window_weak = main_window.as_weak();

    {
        let midi: SharedMidiManager = Rc::clone(&midi);
        let window_weak = window_weak.clone();
        main_window.on_connect_to_output_port(move || {
            connect_to_output_port(window_weak.clone(), &midi);
        });
    }

    {
        let mut midi: SharedMidiManager = Rc::clone(&midi);
        let window_weak = window_weak.clone();
        main_window.on_refresh_output_ports(move || {
            refresh_output_ports(window_weak.clone(), &mut midi);
        });
    }
}

fn refresh_output_ports(
    main_window_weak: Weak<MainWindow>, midi: &mut SharedMidiManager) {
    with_main_window(main_window_weak, |main_window| {
        set_output_ports(&main_window, midi);
        show_warning(main_window, MSG_REFRESHED_OUTPUTS);
    });
}

fn set_output_ports(
    main_window: &MainWindow, midi: &mut SharedMidiManager) {
    let output_port_items: Vec<ComboBoxItem> = midi
        .borrow_mut()
        .get_output_port_names()
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
