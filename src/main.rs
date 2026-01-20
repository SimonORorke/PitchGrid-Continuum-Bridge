// Prevent console window in addition to Slint window in Windows release builds when, e.g., starting the app via file manager. Ignored on other platforms.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod midi;

use std::rc::Rc;
use std::sync::Arc;
use lazy_static::lazy_static;
use slint::Weak;
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
    static ref MIDI_OUTPUT_NAMES: Arc<Vec<String>> = Arc::new(Vec::new());
}

fn main() {
    let main_window = MainWindow::new().unwrap();
    main_window.set_window_title("PitchGrid-Continuum Companion".into());

    let midi_output_names = set_midi_output_names(&main_window);

    let main_window_weak = main_window.as_weak();
    main_window.on_midi_output_changed(move |index: i32| {
        on_midi_output_changed(midi_output_names.clone(), main_window_weak.clone(), index);
    });

    main_window.on_refresh_midi_outputs(move || {
    });

    main_window.run().unwrap();
}

fn on_midi_output_changed(midi_output_names: Rc<Vec<String>>, main_window_weak: Weak<MainWindow>, index: i32) {
    if index >= 0 {
        if let Some(main_window) = main_window_weak.upgrade() {
            if let Some(name) = midi_output_names.get(index as usize) {
                let message = format!("MIDI output changed to {name}");
                main_window.invoke_show_message(message.into(), MessageType::Info);
            }
        }
    }
}

fn set_midi_output_names(main_window: &MainWindow) -> Rc<Vec<String>> {
    let midi_output_names = Rc::new(MIDI_MANAGER.get_midi_output_names());
    let midi_output_items: Vec<ComboBoxItem> = midi_output_names
        .iter()
        .map(|text| ComboBoxItem { text: text.into() })
        .collect();
    let midi_outputs_model = MidiInputsModel(midi_output_items);
    let midi_outputs_model = Rc::new(midi_outputs_model);
    main_window.set_midi_outputs_model(slint::ModelRc::from(midi_outputs_model.clone()));
    midi_output_names
}
