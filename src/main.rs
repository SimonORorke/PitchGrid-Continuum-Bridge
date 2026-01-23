// Prevent console window in addition to Slint window in Windows release builds when, e.g., starting the app via file manager. Ignored on other platforms.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod midi;

use std::rc::Rc;
use slint::Weak;
use midi::MidiManager;
slint::include_modules!();

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
    let mut midi_manager = MidiManager::new();
    set_output_ports(&main_window, &mut midi_manager);

    let mut main_window_weak = main_window.as_weak();
    on_output_port_changed(main_window_weak, &mut midi_manager, 0);

    main_window_weak = main_window.as_weak();
    main_window.on_output_port_changed(move |index: i32| {
        on_output_port_changed(main_window_weak.clone(), &mut midi_manager, index);
    });

    main_window_weak = main_window.as_weak();
    main_window.on_refresh_output_ports(move || {
        refresh_output_ports(main_window_weak.clone(), &mut midi_manager);
    });

    main_window.run().unwrap();
}

fn on_output_port_changed(
    main_window_weak: Weak<MainWindow>, midi_manager: &mut MidiManager, index: i32) {
    if index < 0 {
        return;
    }
    let index = index as usize;
    if let Some(main_window) = main_window_weak.upgrade() {
        let output_port_names = midi_manager.get_output_port_names();
        if let Some(name) = output_port_names.get(index) {
            midi_manager.connect_to_output_port(index);
            let message = format!("Connected to MIDI output port {name}");
            main_window.invoke_show_message(message.into(), MessageType::Info);
        }
    }
}

fn refresh_output_ports(main_window_weak: Weak<MainWindow>, midi_manager: &mut MidiManager) {
    if let Some(main_window) = main_window_weak.upgrade() {
        set_output_ports(&main_window, midi_manager);
        main_window.invoke_show_message("Refreshed MIDI outputs".into(), MessageType::Info);
    }
}
// fn refresh_output_ports(main_window_weak: Weak<MainWindow>) {
//     if let Some(main_window) = main_window_weak.upgrade() {
//         set_output_ports(&main_window);
//         main_window.invoke_show_message("Refreshed MIDI outputs".into(), MessageType::Info);
//     }
// }

fn set_output_ports(main_window: &MainWindow, midi_manager: &mut MidiManager) {
    let output_port_names = midi_manager.get_output_port_names();
    let output_port_items: Vec<ComboBoxItem> = output_port_names
        .iter()
        .map(|text| ComboBoxItem { text: text.into() })
        .collect();
    let output_ports_model = OutputPortsModel(output_port_items);
    let output_ports_model = Rc::new(output_ports_model);
    main_window.set_output_ports_model(slint::ModelRc::from(output_ports_model.clone()));
}
