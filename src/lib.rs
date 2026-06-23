pub mod global;
pub mod continuum_protocol;
pub mod i_continuum_protocol;
pub mod error_notifier;
pub mod i_midi_manager;
pub mod i_osc;
pub mod i_settings;
pub mod i_tuner;
pub mod i_ui_methods;
pub mod midi_manager;
pub mod midi_ports;
pub mod midi_sender;
pub mod osc;
pub mod path_finder;
pub mod settings;
pub mod tuner;
pub mod device_strategy;
pub mod presenter;
pub mod presentation;
pub mod ui_methods;
pub mod tuning_params;
pub mod tuning_update_watchdog;

use std::sync::{Arc, Mutex};
use presenter::Presenter;

pub type SharedPresenter = Arc<Mutex<Presenter>>;

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
