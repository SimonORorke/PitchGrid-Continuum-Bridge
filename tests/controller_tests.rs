mod mock_midi;
mod mock_osc;
mod mock_settings;
mod mock_tuner;
mod mock_ui_methods;

use std::sync::{Arc, LazyLock, Mutex, MutexGuard};
use googletest::assert_that;
use googletest::matchers::{eq, len, ok, some};
use pitchgrid_continuum::controller::Controller;
use pitchgrid_continuum::global::{MessageType, PortType};
use pitchgrid_continuum::midi_static::MidiStatic;
use pitchgrid_continuum::osc::Osc;
use pitchgrid_continuum::tuner::Tuner;
use mock_midi::{MockMidi, midi_state};
use mock_midi::mock_io::{input_state, output_state};
use mock_osc::{MockOsc, osc_state};
use mock_settings::{MockSettings, settings_state};
use mock_tuner::{MockTuner, tuner_state};
use mock_ui_methods::{MockUiMethods, ui_state};
use pitchgrid_continuum::i_settings::ISettings;

#[googletest::gtest]
fn init_from_settings() {
    const LISTENING_PORT: u16 = 34563;
    const PITCH_TABLE: u8 = 81;
    const OVERRIDE_ROUNDING_INITIAL: bool = false; // as the default is true
    const OVERRIDE_ROUNDING_RATE: bool = false; // as the default is true
    const ROUNDING_RATE: u8 = 100;
    let _guard = test_mutex_guard();
    let mut mock_settings = MockSettings::new();
    mock_settings.set_midi_input_device(&INPUT_DEVICE_NAMES[0]);
    mock_settings.set_midi_output_device(&OUTPUT_DEVICE_NAMES[0]);
    mock_settings.set_osc_listening_port(LISTENING_PORT);
    mock_settings.set_pitch_table(PITCH_TABLE);
    mock_settings.set_override_rounding_initial(OVERRIDE_ROUNDING_INITIAL);
    mock_settings.set_override_rounding_rate(OVERRIDE_ROUNDING_RATE);
    mock_settings.set_rounding_rate(ROUNDING_RATE);
    let _controller = create_controller(mock_settings);
    assert_that!(midi_state().init_input_device_name, some(eq(&INPUT_DEVICE_NAMES[0])));
    assert_that!(input_state().device_name(), some(eq(&INPUT_DEVICE_NAMES[0])));
    assert_that!(input_state().device_index(), some(eq(0)));
    assert_that!(midi_state().init_output_device_name, some(eq(&OUTPUT_DEVICE_NAMES[0])));
    assert_that!(output_state().device_name(), some(eq(&OUTPUT_DEVICE_NAMES[0])));
    assert_that!(output_state().device_index(), some(eq(0)));
    assert_that!(ui_state().show_connected_device_name_count, eq(2));
    assert_that!(ui_state().show_connected_device_name_name, some(eq(&INPUT_DEVICE_NAMES[0])));
    assert_that!(ui_state().show_connected_device_name_msg_type, some(eq(MessageType::Info)));
    assert_that!(ui_state().show_message_count, eq(1));
    assert_that!(ui_state().show_message_msg, some(eq("Checking instrument connection...")));
    assert_that!(ui_state().show_message_msg_type, some(eq(MessageType::Info)));
    assert_that!(midi_state().start_instrument_connection_monitor_count, eq(1));
    assert_that!(osc_state().listening_port, some(eq(LISTENING_PORT)));
    assert_that!(tuner_state().pitch_table, some(eq(PITCH_TABLE)));
    assert_that!(tuner_state().override_rounding_initial, some(eq(OVERRIDE_ROUNDING_INITIAL)));
    assert_that!(tuner_state().override_rounding_rate, some(eq(OVERRIDE_ROUNDING_RATE)));
    assert_that!(tuner_state().rounding_rate, some(eq(ROUNDING_RATE)));
    assert_that!(midi_state().start_instrument_connection_monitor_count, eq(1));
}

#[googletest::gtest]
fn init_no_settings() {
    let _guard = test_mutex_guard();
    let _controller = create_controller(MockSettings::new());
    assert_that!(settings_state().read_from_file_count, eq(1));
    assert_that!(settings_state().read_from_file_result, ok(()));
    assert_that!(ui_state().main_window_position_x, some(eq(0)));
    assert_that!(ui_state().main_window_position_y, some(eq(0)));
    assert_that!(ui_state().set_devices_model_count, eq(2));
    assert_that!(ui_state().set_devices_model_device_names, some(len(eq(4))));
    let strategy = ui_state().set_devices_model_port_strategy;
    assert_that!(strategy.as_ref().map(|s| *s.port_type()), some(eq(PortType::Output)));
    // Won't attempt to connect MIDI input port, as MIDI output port has not been
    // read from settings and so cannot be connected. So a warning message is shown for the MIDI
    // output port.
    assert_that!(ui_state().show_connected_device_name_count, eq(1));
    assert_that!(ui_state().show_connected_device_name_name, some(eq("[None]")));
    assert_that!(ui_state().show_connected_device_name_msg_type, some(eq(MessageType::Warning)));
    assert_that!(ui_state().show_message_count, eq(1));
    assert_that!(ui_state().show_message_msg, some(eq("Connect MIDI output port")));
    assert_that!(ui_state().show_message_msg_type, some(eq(MessageType::Warning)));
    assert_that!(osc_state().listening_port, some(eq(Osc::default_listening_port())));
    assert_that!(ui_state().selected_osc_listening_port_index,
        some(eq(Osc::listening_port_index() as i32)));
    assert_that!(tuner_state().init_count, eq(1));
    assert_that!(tuner_state().pitch_table, some(eq(Tuner::default_pitch_table())));
    assert_that!(ui_state().selected_pitch_table_index, some(eq(0)));
    assert_that!(tuner_state().override_rounding_initial, some(eq(true)));
    assert_that!(tuner_state().override_rounding_rate, some(eq(true)));
    assert_that!(tuner_state().rounding_rate, some(eq(127)));
    assert_that!(ui_state().override_rounding_initial, some(eq(true)));
    assert_that!(ui_state().override_rounding_rate, some(eq(true)));
    assert_that!(ui_state().rounding_rate, some(eq(127)));
    assert_that!(midi_state().start_instrument_connection_monitor_count, eq(0));
}

fn create_controller(mock_settings: MockSettings) -> Controller {
    MidiStatic::set_midi(Box::new(MockMidi::new(
        INPUT_DEVICE_NAMES.clone(), OUTPUT_DEVICE_NAMES.clone(),
        mock_settings.midi_input_device(), mock_settings.midi_output_device())));
    let mut controller = Controller::new(Box::new(MockUiMethods::new()));
    controller.set_osc(Box::new(MockOsc::new()));
    controller.set_settings(Box::new(mock_settings));
    controller.set_tuner(Arc::new(MockTuner::new()));
    controller.init();
    controller
}



/// Hold the returned guard in each test to ensure sequential execution of tests.
fn test_mutex_guard() -> MutexGuard<'static, ()> {
    TEST_MUTEX.lock().unwrap_or_else(|e| e.into_inner())
}

static INPUT_DEVICE_NAMES: LazyLock<Vec<String>> = LazyLock::new(|| {
    vec!["Input Device 0".to_string(), "Input Device 1".to_string(), "Input Device 2".to_string()]
});

static OUTPUT_DEVICE_NAMES: LazyLock<Vec<String>> = LazyLock::new(|| {
    vec!["Output Device 0".to_string(), "Output Device 1".to_string(),
         "Output Device 2".to_string(), "Output Device 3".to_string()]
});

/// Tests must run sequentially to avoid races on static data.
static TEST_MUTEX: Mutex<()> = Mutex::new(());
