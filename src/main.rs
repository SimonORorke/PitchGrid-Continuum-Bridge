// Prevent console window in addition to Slint window in Windows release builds when, e.g., starting the app via file manager. Ignored on other platforms.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::rc::Rc;
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

fn main() {
    // use slint::Model; // For Slint component for loops.
    let main_window = MainWindow::new().unwrap();
    main_window.set_window_title("PitchGrid-Continuum Companion".into());

        let midi_input_names = Rc::new(vec![
                "First",
                "Second",
                "Third",
        ]);
        let midi_input_items: Vec<ComboBoxItem> = midi_input_names
            .iter() // Use .iter() instead of .into_iter() to avoid consuming it
            .map(|text| ComboBoxItem { text: (*text).into() })
            .collect();    
        let midi_inputs_model = MidiInputsModel(midi_input_items);
        let midi_inputs_model = Rc::new(midi_inputs_model);
        main_window.set_midi_inputs_model(slint::ModelRc::from(midi_inputs_model.clone()));

        main_window.on_midi_input_changed(move |index: i32| {
            if index >= 0 {
                if let Some(name) = midi_input_names.clone().get(index as usize) {
                    println!("MIDI input changed to {name}");
                }
            }
        });

        main_window.run().unwrap();
}
