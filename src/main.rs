// Prevent console window in addition to Slint window in Windows release builds when, e.g., starting the app via file manager. Ignored on other platforms.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod midi;

use std::rc::Rc;
use lazy_static::lazy_static;
use midi::MidiManager;
slint::include_modules!();

struct MidiInputsModel(Vec<ComboBoxItem>);

impl slint::Model for MidiInputsModel {
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

lazy_static! {
    static ref MIDI_MANAGER: MidiManager = MidiManager::new();
}

fn main() {
    let main_window = MainWindow::new().unwrap();
    main_window.set_window_title("PitchGrid-Continuum Companion".into());

    let midi_output_names = Rc::new(MIDI_MANAGER.get_midi_output_names());
    let midi_output_items: Vec<ComboBoxItem> = midi_output_names
        .iter()
        .map(|text| ComboBoxItem { text: text.into() })
        .collect();
    let midi_outputs_model = MidiInputsModel(midi_output_items);
    let midi_outputs_model = Rc::new(midi_outputs_model);
    main_window.set_midi_outputs_model(slint::ModelRc::from(midi_outputs_model.clone()));

    main_window.on_midi_output_changed(move |index: i32| {
        if index >= 0 {
            if let Some(name) = midi_output_names.clone().get(index as usize) {
                println!("MIDI output changed to {name}");
            }
        }
    });

    main_window.run().unwrap();
}
