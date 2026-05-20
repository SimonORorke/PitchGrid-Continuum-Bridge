mod mock_midi;
mod mock_osc;
mod mock_settings;
mod mock_tuner;
mod mock_ui_methods;

use std::sync::{Arc, Mutex};
use googletest::assert_that;
use googletest::matchers::{eq};
use pitchgrid_continuum::controller::Controller;
use pitchgrid_continuum::midi_static;
use mock_midi::{MockMidi, midi_state};
use mock_osc::{MockOsc, osc_state};
use mock_settings::{MockSettings, settings_state};
use mock_tuner::{MockTuner, tuner_state};
use mock_ui_methods::{MockUiMethods, ui_state};

/// Tests must run sequentially to avoid races on static data.
static TEST_MUTEX: Mutex<()> = Mutex::new(());

#[googletest::gtest]
fn init() {
    let _guard = test_mutex_guard();
    let mut controller = create_controller();
}

fn create_controller() -> Controller {
    midi_static::set_midi(Box::new(MockMidi::new()));
    let mut controller = Controller::new(Box::new(MockUiMethods::new()));
    controller.set_osc(Box::new(MockOsc::new()));
    controller.set_settings(Box::new(MockSettings::new()));
    controller.set_tuner(Arc::new(MockTuner::new()));
    controller.init();
    controller
}

fn test_mutex_guard() -> std::sync::MutexGuard<'static, ()> {
    TEST_MUTEX.lock().unwrap_or_else(|e| e.into_inner())
}
