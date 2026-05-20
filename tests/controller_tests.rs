mod mock_midi;
mod mock_osc;
mod mock_settings;
mod mock_tuner;
mod mock_ui_methods;

use std::sync::{Arc, Mutex};
use googletest::assert_that;
use googletest::matchers::{eq, some};
use pitchgrid_continuum::controller::Controller;
use pitchgrid_continuum::midi_static::MidiStatic;
use pitchgrid_continuum::osc::Osc;
use pitchgrid_continuum::tuner::Tuner;
use mock_midi::{MockMidi, midi_state};
use mock_osc::{MockOsc, osc_state};
use mock_settings::{MockSettings, settings_state};
use mock_tuner::{MockTuner, tuner_state};
use mock_ui_methods::{MockUiMethods, ui_state};

/// Tests must run sequentially to avoid races on static data.
static TEST_MUTEX: Mutex<()> = Mutex::new(());

#[googletest::gtest]
fn init_no_settings() {
    let _guard = test_mutex_guard();
    let _controller = create_controller();
    assert_that!(osc_state().listening_port,
        some(eq(Osc::default_listening_port())));
    assert_that!(tuner_state().pitch_table,
        some(eq(Tuner::default_pitch_table())));
}

fn create_controller() -> Controller {
    MidiStatic::set_midi(Box::new(MockMidi::new()));
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
