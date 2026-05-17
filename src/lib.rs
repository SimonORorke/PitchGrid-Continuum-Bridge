pub mod global;
pub mod i_midi;
pub mod i_ui_methods;
pub mod midi;
pub mod midi_ports;
pub mod midi_sender;
pub mod osc;
pub mod system_paths;
pub mod settings;
pub mod tuner;
pub mod port_strategy;
pub mod controller;
pub mod ui_methods;
pub mod midi_static;
pub mod tuning_params;

use std::sync::{Arc, Mutex};
use controller::Controller;

pub type SharedController = Arc<Mutex<Controller>>;

pub struct ComboBoxModel(pub Vec<ComboBoxItem>);

impl slint::Model for ComboBoxModel {
    type Data = ComboBoxItem;
    fn row_count(&self) -> usize {
        self.0.len()
    }
    fn row_data(&self, row: usize) -> Option<Self::Data> {
        self.0.get(row).cloned()
    }
    fn model_tracker(&self) -> &dyn slint::ModelTracker {
        &()
    }
}

slint::include_modules!();
