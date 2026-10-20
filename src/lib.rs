pub mod centered_dialog;
pub mod continuum_midi_batch;
pub mod continuum_protocol;
pub mod ui_device_strategy;
pub mod global;
pub mod i_continuum_protocol;
pub mod i_osc;
pub mod i_settings;
pub mod i_ui_methods;
pub mod osc;
pub mod presenter;
pub mod presentation;
pub mod settings;
pub mod tuner;
pub mod tuning_params;
pub mod tuning_update_watchdog;
pub mod ui_methods;

pub use centered_dialog::CenteredDialog;
pub use continuum_midi_batch::ContinuumMidiBatch;
pub use continuum_protocol::{ContinuumProtocol, ContinuumProtocolReceiver};
pub use ui_device_strategy::{UiDeviceStrategy, UiInputStrategy, UiOutputStrategy};
pub use global::{MessageType, override_note_names};
pub use i_continuum_protocol::{
    ContinuumProtocolListener, IContinuumProtocol, TuningUpdateSignaller,
};
pub use i_osc::{IOsc, OscCallbacks};
pub use i_settings::ISettings;
pub use i_ui_methods::IUiMethods;
pub use osc::Osc;
pub use presenter::{NewVersionCallbacks, NewVersionCallbacksWrapper, Presenter};
pub use presentation::Presentation;
pub use settings::Settings;
pub use tuner::Tuner;
pub use tuning_params::{FormattedTuningParams, TuningParams};
pub use tuning_update_watchdog::TuningUpdateWatchdog;
pub use ui_methods::UiMethods;

use std::sync::{Arc, Mutex};

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
