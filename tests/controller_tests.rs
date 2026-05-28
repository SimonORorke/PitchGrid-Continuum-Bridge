mod mock_midi;
mod mock_osc;
mod mock_settings;
mod mock_tuner;
mod mock_ui_methods;
mod tuner_tests;

use std::sync::{Arc, LazyLock, Mutex, MutexGuard};
use googletest::assert_that;
use googletest::matchers::{
    anything, displays_as, eq, err, len, ok, not, some, starts_with};
use pitchgrid_continuum::controller::Controller;
use pitchgrid_continuum::global::{MessageType, PortType};
use pitchgrid_continuum::i_settings::ISettings;
use pitchgrid_continuum::midi_static::MidiStatic;
use pitchgrid_continuum::osc::Osc;
use pitchgrid_continuum::port_strategy::InputStrategy;
use pitchgrid_continuum::tuner::Tuner;
use mock_midi::{MockMidi, midi_state};
use mock_midi::mock_io::{input_state, output_state};
use mock_osc::{MockOsc, osc_state};
use mock_settings::{MockSettings, settings_state};
use mock_tuner::{MockTuner, tuner_state};
use mock_ui_methods::{MockUiMethods, ui_state};
use tuner_tests::{params_16_16};

#[googletest::gtest]
fn init_from_settings() {
    let _guard = test_mutex_guard();
    const LISTENING_PORT: u16 = 34563;
    const PITCH_TABLE: u8 = 81;
    const OVERRIDE_ROUNDING_INITIAL: bool = false; // as the default is true
    const OVERRIDE_ROUNDING_RATE: bool = false; // as the default is true
    const ROUNDING_RATE: u8 = 100;
    let mut mock_settings = MockSettings::new();
    mock_settings.set_osc_listening_port(LISTENING_PORT);
    mock_settings.set_pitch_table(PITCH_TABLE);
    mock_settings.set_override_rounding_initial(OVERRIDE_ROUNDING_INITIAL);
    mock_settings.set_override_rounding_rate(OVERRIDE_ROUNDING_RATE);
    mock_settings.set_rounding_rate(ROUNDING_RATE);
    let mut controller = create_controller(mock_settings, true);
    controller.init();
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
fn init_midi_err() {
    let _guard = test_mutex_guard();
    const ERR_MSG: &str = "Test error";
    let mut controller = create_controller(MockSettings::new(), true);
    MockMidi::simulate_init_err(ERR_MSG);
    controller.init();
    assert_that!(ui_state().show_message_count, eq(1));
    assert_that!(ui_state().show_message_msg, some(eq(ERR_MSG)));
    assert_that!(ui_state().show_message_msg_type, some(eq(MessageType::Error)));
    assert_that!(midi_state().start_instrument_connection_monitor_count, eq(0));
}

#[googletest::gtest]
fn init_no_settings() {
    let _guard = test_mutex_guard();
    let mut controller = create_controller(MockSettings::new(), false);
    controller.init();
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

#[googletest::gtest]
fn init_read_settings_err() {
    let _guard = test_mutex_guard();
    const ERR_MSG: &str = "Test error";
    let mut mock_settings = MockSettings::new();
    mock_settings.simulate_read_from_file_err(ERR_MSG);
    let mut controller = create_controller(mock_settings, true);
    controller.init();
    assert_that!(ui_state().show_message_count, eq(1));
    assert_that!(ui_state().show_message_msg, some(eq(ERR_MSG)));
    assert_that!(ui_state().show_message_msg_type, some(eq(MessageType::Error)));
    assert_that!(midi_state().start_instrument_connection_monitor_count, eq(0));
}

#[googletest::gtest]
fn refresh_devices() {
    let _guard = test_mutex_guard();
    let mut controller = create_controller(MockSettings::new(), true);
    controller.init();
    assert_that!(ui_state().set_devices_model_count, eq(2));
    MockMidi::set_is_receiving_data(true);
    MockMidi::set_are_ports_connected(true);
    MockMidi::simulate_download_completed();
    MockOsc::simulate_tuning_received(params_16_16());
    MockMidi::simulate_updating_tuning();
    MockMidi::simulate_tuning_updated();
    assert_that!(ui_state().show_tuning_count, eq(1));
    let port_strategy = InputStrategy::new();
    controller.refresh_devices(&port_strategy);
    assert_that!(midi_state().stop_instrument_connection_monitor_count, eq(1));
    assert_that!(osc_state().stop_count, eq(1));
    assert_that!(tuner_state().remove_data_count, eq(1));
    let formatted_tuning =
        tuner_state().tuning_params.as_ref().unwrap().format_tuning_params();
    assert_that!(formatted_tuning.root_freq, eq("")); // Tuning data has been removed.
    assert_that!(ui_state().show_tuning_count, eq(2));
    assert_that!(ui_state().show_pitchgrid_status_msg,
        some(eq("Disconnected from PitchGrid because MIDI is not connected")));
    assert_that!(ui_state().show_pitchgrid_status_msg_type, some(eq(MessageType::Warning)));
    assert_that!(ui_state().set_devices_model_count, eq(3));
    let strategy = ui_state().set_devices_model_port_strategy;
    assert_that!(strategy.as_ref().map(|s| *s.port_type()), some(eq(PortType::Input)));
    assert_that!(ui_state().show_connected_device_name_name, some(eq("[None]")));
    assert_that!(ui_state().show_message_msg, some(starts_with("Refreshed MIDI input ports.")));
    assert_that!(ui_state().show_message_msg_type, some(eq(MessageType::Warning)));
}

#[googletest::gtest]
fn close() {
    let _guard = test_mutex_guard();
    const OLD_MAIN_WINDOW_X: i32 = 100;
    const OLD_MAIN_WINDOW_Y: i32 = 200;
    const NEW_MAIN_WINDOW_X: i32 = 150;
    const NEW_MAIN_WINDOW_Y: i32 = 250;
    let mut mock_settings = MockSettings::new();
    mock_settings.set_main_window_x(OLD_MAIN_WINDOW_X);
    mock_settings.set_main_window_y(OLD_MAIN_WINDOW_Y);
    let mut controller = create_controller(mock_settings, true);
    controller.init();
    assert_that!(midi_state().start_instrument_connection_monitor_count, eq(1));
    let result = controller.close(NEW_MAIN_WINDOW_X, NEW_MAIN_WINDOW_Y);
    assert_that!(result, ok(()));
    assert_that!(midi_state().close_count, eq(1));
    assert_that!(osc_state().stop_count, eq(1));
    assert_that!(settings_state().main_window_x, eq(NEW_MAIN_WINDOW_X));
    assert_that!(settings_state().main_window_y, eq(NEW_MAIN_WINDOW_Y));
}

#[googletest::gtest]
fn close_err() {
    let _guard = test_mutex_guard();
    const ERR_MSG: &str = "Test error";
    const OLD_MAIN_WINDOW_X: i32 = 100;
    const OLD_MAIN_WINDOW_Y: i32 = 200;
    const NEW_MAIN_WINDOW_X: i32 = 150;
    const NEW_MAIN_WINDOW_Y: i32 = 250;
    let mut mock_settings = MockSettings::new();
    mock_settings.set_main_window_x(OLD_MAIN_WINDOW_X);
    mock_settings.set_main_window_y(OLD_MAIN_WINDOW_Y);
    mock_settings.simulate_write_to_file_err(ERR_MSG);
    let mut controller = create_controller(mock_settings, true);
    controller.init();
    assert_that!(midi_state().start_instrument_connection_monitor_count, eq(1));
    let result = controller.close(NEW_MAIN_WINDOW_X, NEW_MAIN_WINDOW_Y);
    assert_that!(result, err(displays_as(eq(ERR_MSG))));
    assert_that!(midi_state().close_count, eq(1));
    assert_that!(osc_state().stop_count, eq(1));
    assert_that!(settings_state().main_window_x, eq(NEW_MAIN_WINDOW_X));
    assert_that!(settings_state().main_window_y, eq(NEW_MAIN_WINDOW_Y));
    assert_that!(ui_state().show_message_msg, some(eq(ERR_MSG)));
    assert_that!(ui_state().show_message_msg_type, some(eq(MessageType::Error)));
}

#[googletest::gtest]
fn on_receiving_data_started_show_waiting_for_download() {
    let _guard = test_mutex_guard();
    let mut controller = create_controller(MockSettings::new(), true);
    controller.init();
    MockMidi::simulate_receiving_data_started();
    assert_that!(ui_state().show_message_msg, some(starts_with("Waiting (maximum 6 seconds)")));
    assert_that!(ui_state().show_message_msg_type, some(eq(MessageType::Info)));
}

#[googletest::gtest]
fn on_data_download_started() {
    let _guard = test_mutex_guard();
    let mut controller = create_controller(MockSettings::new(), true);
    controller.init();
    MockMidi::set_are_ports_connected(true);
    MockMidi::simulate_download_started();
    assert_that!(ui_state().show_message_msg, some(starts_with("Awaiting completion")));
    assert_that!(ui_state().show_message_msg_type, some(eq(MessageType::Info)));
}

#[googletest::gtest]
fn on_data_download_completed_start_osc() {
    let _guard = test_mutex_guard();
    let mut controller = create_controller(MockSettings::new(), true);
    controller.init();
    MockMidi::set_is_receiving_data(true);
    MockMidi::set_are_ports_connected(true);
    MockMidi::simulate_download_completed();
    assert_that!(osc_state().start_count, eq(1));
    assert_that!(ui_state().show_message_msg, some(starts_with("Opening PitchGrid connection")));
    assert_that!(ui_state().show_message_msg_type, some(eq(MessageType::Info)));
}

#[googletest::gtest]
fn on_osc_tuning_received() {
    let _guard = test_mutex_guard();
    let mut controller = create_controller(MockSettings::new(), true);
    controller.init();
    MockMidi::set_is_receiving_data(true);
    MockMidi::set_are_ports_connected(true);
    MockMidi::simulate_download_completed();
    MockOsc::simulate_tuning_received(params_16_16());
    assert_that!(tuner_state().tuning_params, some(eq(&params_16_16())));
    assert_that!(ui_state().show_pitchgrid_status_count, eq(1));
    assert_that!(ui_state().show_pitchgrid_status_msg,
        some(eq("Updating instrument tuning")));
    assert_that!(ui_state().show_pitchgrid_status_msg_type, some(eq(MessageType::Info)));
}

#[googletest::gtest]
fn on_updating_tuning() {
    let _guard = test_mutex_guard();
    let mut controller = create_controller(MockSettings::new(), true);
    controller.init();
    MockMidi::simulate_updating_tuning();
    assert_that!(ui_state().show_message_msg_type, some(not(eq(MessageType::Error))));
}

#[googletest::gtest]
fn on_tuning_updated() {
    let _guard = test_mutex_guard();
    const NOTE_INDEX: usize = 1;
    let mut controller = create_controller(MockSettings::new(), true);
    controller.init();
    controller.set_root_freq_override(NOTE_INDEX);
    MockMidi::set_is_receiving_data(true);
    MockMidi::set_are_ports_connected(true);
    MockMidi::simulate_download_completed();
    MockOsc::simulate_tuning_received(params_16_16());
    MockMidi::simulate_updating_tuning();
    MockMidi::simulate_tuning_updated();
    assert_that!(tuner_state().on_tuning_updated_count, eq(1));
    assert_that!(ui_state().show_tuning_count, eq(1));
    assert_that!(ui_state().show_tuning_is_root_freq_overridden, some(eq(true)));
    assert_that!(tuner_state().tuning_params, some(anything()));
    assert_that!(ui_state().show_tuning_formatted_tuning,
        some(eq(&tuner_state().tuning_params.unwrap().format_tuning_params())));
}

#[googletest::gtest]
fn on_new_preset_selected() {
    let _guard = test_mutex_guard();
    const NOTE_INDEX: usize = 1;
    let mut controller = create_controller(MockSettings::new(), true);
    controller.init();
    controller.set_root_freq_override(NOTE_INDEX);
    MockMidi::set_is_receiving_data(true);
    MockMidi::set_are_ports_connected(true);
    MockMidi::simulate_download_completed();
    MockOsc::simulate_tuning_received(params_16_16());
    assert_that!(ui_state().show_pitchgrid_status_count, eq(1));
    MockMidi::simulate_updating_tuning();
    MockMidi::simulate_tuning_updated();
    MockMidi::simulate_new_preset_selected();
    assert_that!(tuner_state().send_current_preset_update_count, eq(1));
    assert_that!(ui_state().show_pitchgrid_status_msg,
        some(starts_with("New instrument preset selected")));
    assert_that!(ui_state().show_pitchgrid_status_msg_type, some(eq(MessageType::Info)));
}

#[googletest::gtest]
fn  set_root_freq_override() {
    let _guard = test_mutex_guard();
    const NOTE_INDEX: usize = 1;
    let mut controller = create_controller(MockSettings::new(), true);
    controller.init();
    MockMidi::set_is_receiving_data(true);
    MockMidi::set_are_ports_connected(true);
    MockMidi::simulate_download_completed();
    assert_that!(ui_state().show_pitchgrid_status_count, eq(0));
    MockOsc::simulate_pitchgrid_connected_changed(true);
    assert_that!(ui_state().show_pitchgrid_status_count, eq(1));
    controller.set_root_freq_override(NOTE_INDEX);
    assert_that!(ui_state().show_pitchgrid_status_count, eq(2));
    assert_that!(ui_state().show_pitchgrid_status_msg, some(starts_with("Updating root")));
    assert_that!(ui_state().show_pitchgrid_status_msg_type, some(eq(MessageType::Info)));
    assert_that!(tuner_state().root_freq_override_note_no, some(eq(NOTE_INDEX)));
    assert_that!(tuner_state().set_root_freq_override_note_no_send_tuning, some(eq(true)));
}

fn create_controller(mut mock_settings: MockSettings, default_midi_devices: bool) -> Controller {
    if default_midi_devices {
        mock_settings.set_midi_input_device(&INPUT_DEVICE_NAMES[0]);
        mock_settings.set_midi_output_device(&OUTPUT_DEVICE_NAMES[0]);
    }
    MidiStatic::set_midi(Box::new(MockMidi::new(
        INPUT_DEVICE_NAMES.clone(), OUTPUT_DEVICE_NAMES.clone(),
        mock_settings.midi_input_device(), mock_settings.midi_output_device())));
    // Controller::init calls clone_controller(), which requires the CONTROLLER singleton to be set.
    // In main, the same shared instance is used for both set_controller and init. Here we use
    // separate instances: the local controller is used to call init() directly without locking a
    // shared controller, while the singleton is used only for MIDI callbacks. Both are configured
    // with MockOsc so that callback-triggered OSC calls are recorded in osc_state().
    let mut singleton = Controller::new(Box::new(MockUiMethods::new()));
    singleton.set_osc(Box::new(MockOsc::new()));
    singleton.set_tuner(Arc::new(MockTuner::new()));
    Controller::set_controller(Arc::new(Mutex::new(singleton)));
    let mut controller = Controller::new(Box::new(MockUiMethods::new()));
    controller.set_osc(Box::new(MockOsc::new()));
    controller.set_settings(Box::new(mock_settings));
    controller.set_tuner(Arc::new(MockTuner::new()));
    controller
}

/// To avoid races on static data, hold the returned guard in each test to ensure sequential
/// execution of tests.
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
