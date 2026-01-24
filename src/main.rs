// Prevent console window in addition to Slint window in Windows release builds when, e.g., starting the app via file manager. Ignored on other platforms.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod midi;

use std::cell::RefCell;
use std::rc::Rc;
use slint::Weak;
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

fn main() {
    let main_window = MainWindow::new().unwrap();
    main_window.set_window_title("PitchGrid-Continuum Companion".into());

    let midi: SharedMidiManager = Rc::new(RefCell::new(MidiManager::new()));
    set_output_ports(&main_window, &midi);

    let main_window_weak = main_window.as_weak();
    on_output_port_changed(main_window_weak, &midi, 0);

    let main_window_weak = main_window.as_weak();
    let midi_for_output_change: SharedMidiManager = Rc::clone(&midi);
    main_window.on_output_port_changed(move |index: i32| {
        on_output_port_changed(
            main_window_weak.clone(), &midi_for_output_change, index as usize);
    });

    let main_window_weak = main_window.as_weak();
    let midi_for_refresh: SharedMidiManager = Rc::clone(&midi);
    main_window.on_refresh_output_ports(move || {
        refresh_output_ports(main_window_weak.clone(), &midi_for_refresh);
    });

    main_window.run().unwrap();
}

fn on_output_port_changed(
    main_window_weak: Weak<MainWindow>,
    midi: &SharedMidiManager,
    index: usize,
) {
    if let Some(main_window) = main_window_weak.upgrade() {
        let mut midi_manager = midi.borrow_mut();
        let output_port_names = midi_manager.get_output_port_names();
        if let Some(name) = output_port_names.get(index) {
            midi_manager.connect_to_output_port(index);
            let message = format!("Connected to MIDI output port {name}");
            main_window.invoke_show_message(message.into(), MessageType::Info);
        }
    }
}

fn refresh_output_ports(
    main_window_weak: Weak<MainWindow>, midi: &SharedMidiManager) {
    if let Some(main_window) = main_window_weak.upgrade() {
        set_output_ports(&main_window, midi);
        main_window.invoke_show_message("Refreshed MIDI outputs".into(), MessageType::Info);
    }
}

fn set_output_ports(
    main_window: &MainWindow, midi: &SharedMidiManager) {
    let output_port_names = midi.borrow_mut().get_output_port_names();
    let output_port_items: Vec<ComboBoxItem> = output_port_names
        .iter()
        .map(|text| ComboBoxItem { text: text.into() })
        .collect();
    let output_ports_model = OutputPortsModel(output_port_items);
    let output_ports_model = Rc::new(output_ports_model);
    main_window.set_output_ports_model(slint::ModelRc::from(output_ports_model.clone()));
}
