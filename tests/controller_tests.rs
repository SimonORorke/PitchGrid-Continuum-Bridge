mod mock_midi;
mod mock_midi_sender;
mod mock_osc;
mod mock_settings;
mod mock_ui_methods;

// `mod test_tunings` is declared twice, here and in `tests/tuner_tests.rs`.
// Each is compiled separately. So there would be a compiler warning for any `test_tunings`
// functions that are not used in this module.  The workaround is the `#[allow(dead_code)]`
// annotation.
#[allow(dead_code)] mod test_tunings;

use std::sync::{Arc, LazyLock, Mutex, MutexGuard};
use googletest::assert_that;
use googletest::matchers::{
    displays_as, eq, err, len, ok, not, some, starts_with};
use pitchgrid_continuum::controller::{Controller, AWAITING_DATA_DOWNLOAD_COMPLETION, AWAITING_PITCHGRID_CONNECTION, CANNOT_UPDATE_TUNING_LOST, CHECKING_INSTRUMENT_CONNECTION, DEVICE_NONE, DISCONNECTED_FROM_PITCHGRID, INSTRUMENT_DISCONNECTED, INSTRUMENT_NOT_CONNECTED, INSTRUMENT_TUNING_UPDATED, NEW_PRESET_SELECTED, OPENING_PITCHGRID_CONNECTION, PITCHGRID_CONNECTION_CLOSED, PITCHGRID_NOT_CONNECTED, UPDATING_INSTRUMENT_TUNING, UPDATING_ROOT_FREQ_OVERRIDE, WAITING_FOR_DATA_DOWNLOAD};
use pitchgrid_continuum::global::{MessageType, DeviceType};
use pitchgrid_continuum::i_settings::ISettings;
use pitchgrid_continuum::midi::Midi;
use pitchgrid_continuum::osc::Osc;
use pitchgrid_continuum::device_strategy::{InputStrategy, OutputStrategy};
use pitchgrid_continuum::i_tuner::ITuner;
use pitchgrid_continuum::tuner::Tuner;
use mock_midi::{MockMidi, mock_midi};
use mock_midi::mock_io::{input_state, output_state};
use mock_osc::{MockOsc, mock_osc};
use mock_settings::{MockSettings, mock_settings};
use mock_midi_sender::MockMidiSender;
use mock_ui_methods::{MockUiMethods, mock_ui_methods};
use test_tunings::TestTunings;

#[googletest::gtest]
fn init_from_settings() {
    let _guard = test_mutex_guard();
    const LISTENING_PORT: u16 = 34563;
    const PITCH_TABLE: u8 = 81;
    const OVERRIDE_ROUNDING_INITIAL: bool = false; // as the default is true
    const OVERRIDE_ROUNDING_RATE: bool = false; // as the default is true
    const ROUNDING_RATE: u8 = 100;
    let mut settings = MockSettings::new();
    settings.set_osc_listening_port(LISTENING_PORT);
    settings.set_pitch_table(PITCH_TABLE);
    settings.set_override_rounding_initial(OVERRIDE_ROUNDING_INITIAL);
    settings.set_override_rounding_rate(OVERRIDE_ROUNDING_RATE);
    settings.set_rounding_rate(ROUNDING_RATE);
    let mut controller = create_controller(settings, true);
    controller.init();
    assert_that!(mock_midi().init_input_device_name, some(eq(&INPUT_DEVICE_NAMES[0])));
    assert_that!(input_state().device_name(), some(eq(&INPUT_DEVICE_NAMES[0])));
    assert_that!(input_state().device_index(), some(eq(0)));
    assert_that!(mock_midi().init_output_device_name, some(eq(&OUTPUT_DEVICE_NAMES[0])));
    assert_that!(output_state().device_name(), some(eq(&OUTPUT_DEVICE_NAMES[0])));
    assert_that!(output_state().device_index(), some(eq(0)));
    assert_that!(mock_ui_methods().show_connected_device_name_count, eq(2));
    assert_that!(mock_ui_methods().show_connected_device_name_name, some(eq(&INPUT_DEVICE_NAMES[0])));
    assert_that!(mock_ui_methods().show_connected_device_name_msg_type, some(eq(MessageType::Info)));
    assert_that!(mock_ui_methods().show_message_count, eq(1));
    assert_that!(mock_ui_methods().show_message_msg, some(eq(CHECKING_INSTRUMENT_CONNECTION)));
    assert_that!(mock_ui_methods().show_message_msg_type, some(eq(MessageType::Info)));
    assert_that!(mock_midi().start_instrument_connection_monitor_count, eq(1));
    assert_that!(mock_osc().listening_port, some(eq(LISTENING_PORT)));
    assert_that!(Tuner::pitch_table(), eq(PITCH_TABLE));
    assert_that!(mock_midi().start_instrument_connection_monitor_count, eq(1));
}

#[googletest::gtest]
fn init_no_settings() {
    let _guard = test_mutex_guard();
    let mut controller = create_controller(MockSettings::new(), false);
    controller.init();
    assert_that!(mock_ui_methods().main_window_position_x, some(eq(0)));
    assert_that!(mock_ui_methods().main_window_position_y, some(eq(0)));
    assert_that!(mock_ui_methods().set_devices_model_count, eq(2));
    assert_that!(mock_ui_methods().set_devices_model_device_names, some(len(eq(4))));
    let guard = mock_ui_methods();
    let strategy = &guard.set_devices_model_device_strategy;
    assert_that!(strategy.as_ref().map(|s| *s.device_type()), some(eq(DeviceType::Output)));
    // Won't attempt to connect MIDI input device, as MIDI output device has not been
    // read from settings and so cannot be connected. So a warning message is shown for the MIDI
    // output device.
    assert_that!(mock_ui_methods().show_connected_device_name_count, eq(1));
    assert_that!(mock_ui_methods().show_connected_device_name_name, some(eq(DEVICE_NONE)));
    assert_that!(mock_ui_methods().show_connected_device_name_msg_type, some(eq(MessageType::Warning)));
    assert_that!(mock_ui_methods().show_message_count, eq(1));
    assert_that!(mock_ui_methods().show_message_msg, some(eq("Connect MIDI output device")));
    assert_that!(mock_ui_methods().show_message_msg_type, some(eq(MessageType::Warning)));
    assert_that!(mock_osc().listening_port, some(eq(Osc::default_listening_port())));
    assert_that!(mock_ui_methods().selected_osc_listening_port_index,
        some(eq(Osc::listening_port_index() as i32)));
    assert_that!(Tuner::pitch_table(), eq(Tuner::default_pitch_table()));
    assert_that!(mock_ui_methods().selected_pitch_table_index, some(eq(0)));
    assert_that!(mock_ui_methods().override_rounding_initial, some(eq(true)));
    assert_that!(mock_ui_methods().override_rounding_rate, some(eq(true)));
    assert_that!(mock_ui_methods().rounding_rate, some(eq(127)));
    assert_that!(mock_midi().start_instrument_connection_monitor_count, eq(0));
}

#[googletest::gtest]
fn init_read_settings_err() {
    let _guard = test_mutex_guard();
    const ERR_MSG: &str = "Test error";
    let settings = MockSettings::new();
    MockSettings::simulate_read_from_file_err(ERR_MSG);
    let mut controller = create_controller(settings, true);
    controller.init();
    assert_that!(mock_ui_methods().show_message_count, eq(1));
    assert_that!(mock_ui_methods().show_message_msg, some(eq(ERR_MSG)));
    assert_that!(mock_ui_methods().show_message_msg_type, some(eq(MessageType::Error)));
    assert_that!(mock_midi().start_instrument_connection_monitor_count, eq(0));
}

#[googletest::gtest]
fn connect_device() {
    let _guard = test_mutex_guard();
    let mut controller = create_controller(MockSettings::new(), true);
    controller.init();
    assert_that!(mock_midi().start_instrument_connection_monitor_count, eq(1));
    MockMidi::set_is_receiving_data(true);
    MockMidi::set_are_devices_connected(true);
    MockMidi::simulate_download_completed();
    MockOsc::simulate_tuning_received(TestTunings::params_17_17());
    MockMidi::simulate_updating_tuning();
    MockMidi::simulate_tuning_updated();
    assert_that!(tuner().has_data(), eq(true));
    assert_that!(tuner().formatted_tuning_params().root_freq, not(eq("")));
    let device_strategy = InputStrategy::new();
    MockUiMethods::set_selected_device_index(1);
    controller.connect_device(&device_strategy);
    assert_that!(mock_midi().stop_instrument_connection_monitor_count, eq(1));
    assert_that!(mock_osc().stop_count, eq(1));
    assert_that!(tuner().has_data(), eq(false));
    assert_that!(tuner().formatted_tuning_params().root_freq, eq(""));
    assert_that!(mock_ui_methods().show_pitchgrid_status_msg,
        some(eq(DISCONNECTED_FROM_PITCHGRID)));
    assert_that!(mock_ui_methods().show_pitchgrid_status_msg_type, some(eq(MessageType::Warning)));
    assert_that!(mock_ui_methods().show_connected_device_name_name, some(eq(&INPUT_DEVICE_NAMES[1])));
    assert_that!(mock_ui_methods().show_connected_device_name_msg_type, some(eq(MessageType::Info)));
    assert_that!(mock_ui_methods().show_message_msg, some(eq("Checking instrument connection...")));
    assert_that!(mock_ui_methods().show_message_msg_type, some(eq(MessageType::Info)));
    assert_that!(mock_midi().start_instrument_connection_monitor_count, eq(2));
}

#[googletest::gtest]
fn connect_device_after_refreshing_other_device_list() {
    let _guard = test_mutex_guard();
    let mut controller = create_controller(MockSettings::new(), true);
    controller.init();
    assert_that!(mock_midi().start_instrument_connection_monitor_count, eq(1));
    MockMidi::set_is_receiving_data(true);
    MockMidi::set_are_devices_connected(true);
    MockMidi::simulate_download_completed();
    MockOsc::simulate_tuning_received(TestTunings::params_17_17());
    MockMidi::simulate_updating_tuning();
    MockMidi::simulate_tuning_updated();
    let output_strategy = OutputStrategy::new();
    controller.refresh_devices(&output_strategy);
    let input_strategy = InputStrategy::new();
    MockUiMethods::set_selected_device_index(1);
    controller.connect_device(&input_strategy);
    assert_that!(mock_ui_methods().show_message_msg, some(eq("Connect MIDI output device")));
    assert_that!(mock_ui_methods().show_message_msg_type, some(eq(MessageType::Warning)));
    assert_that!(mock_midi().start_instrument_connection_monitor_count, eq(1));
}

#[googletest::gtest]
fn connect_device_err() {
    let _guard = test_mutex_guard();
    const ERR_MSG: &str = "Test error";
    let mut controller = create_controller(MockSettings::new(), false);
    controller.init();
    assert_that!(mock_midi().start_instrument_connection_monitor_count, eq(0));
    let device_strategy = OutputStrategy::new();
    MockUiMethods::set_selected_device_index(1);
    MockMidi::simulate_connect_device_err(ERR_MSG);
    controller.connect_device(&device_strategy);
    assert_that!(mock_ui_methods().show_message_msg, some(eq(ERR_MSG)));
    assert_that!(mock_ui_methods().show_message_msg_type, some(eq(MessageType::Error)));
}

#[googletest::gtest]
fn refresh_devices() {
    let _guard = test_mutex_guard();
    let mut controller = create_controller(MockSettings::new(), true);
    controller.init();
    assert_that!(mock_ui_methods().set_devices_model_count, eq(2));
    MockMidi::set_is_receiving_data(true);
    MockMidi::set_are_devices_connected(true);
    MockMidi::simulate_download_completed();
    MockOsc::simulate_tuning_received(TestTunings::params_16_16());
    MockMidi::simulate_updating_tuning();
    MockMidi::simulate_tuning_updated();
    assert_that!(mock_ui_methods().show_tuning_count, eq(1));
    let input_strategy = InputStrategy::new();
    controller.refresh_devices(&input_strategy);
    assert_that!(mock_midi().stop_instrument_connection_monitor_count, eq(1));
    assert_that!(mock_osc().stop_count, eq(1));
    assert_that!(tuner().has_data(), eq(false));
    assert_that!(tuner().formatted_tuning_params().root_freq, eq(""));
    assert_that!(mock_ui_methods().show_tuning_count, eq(2));
    assert_that!(mock_ui_methods().show_pitchgrid_status_msg, some(eq(DISCONNECTED_FROM_PITCHGRID)));
    assert_that!(mock_ui_methods().show_pitchgrid_status_msg_type, some(eq(MessageType::Warning)));
    assert_that!(mock_ui_methods().set_devices_model_count, eq(3));
    let guard = mock_ui_methods();
    let strategy = &guard.set_devices_model_device_strategy;
    assert_that!(strategy.as_ref().map(|s| *s.device_type()), some(eq(DeviceType::Input)));
    assert_that!(mock_ui_methods().show_connected_device_name_name, some(eq(DEVICE_NONE)));
    assert_that!(mock_ui_methods().show_message_msg, some(starts_with("Refreshed MIDI input devices.")));
    assert_that!(mock_ui_methods().show_message_msg_type, some(eq(MessageType::Warning)));
}

#[googletest::gtest]
fn on_devices_connected_changed_to_connected() {
    let _guard = test_mutex_guard();
    let mut controller = create_controller(MockSettings::new(), true);
    controller.init();
    MockMidi::set_are_devices_connected(true);
    MockOsc::set_is_running_result(true);
    MockMidi::simulate_devices_connected_changed();
    assert_that!(mock_osc().stop_count,eq(0));
}

#[googletest::gtest]
fn on_devices_connected_changed_to_not_connected() {
    let _guard = test_mutex_guard();
    let mut controller = create_controller(MockSettings::new(), true);
    controller.init();
    MockMidi::set_are_devices_connected(false);
    MockOsc::set_is_running_result(true);
    MockMidi::simulate_devices_connected_changed();
    assert_that!(mock_osc().stop_count,eq(1));
    assert_that!(mock_ui_methods().show_message_msg, some(eq(INSTRUMENT_DISCONNECTED)));
    assert_that!(mock_ui_methods().show_message_msg_type, some(eq(MessageType::Warning)));
    assert_that!(mock_ui_methods().show_pitchgrid_status_msg, some(eq(PITCHGRID_CONNECTION_CLOSED)));
    assert_that!(mock_ui_methods().show_pitchgrid_status_msg_type, some(eq(MessageType::Warning)));
}

#[googletest::gtest]
fn close() {
    let _guard = test_mutex_guard();
    const OLD_MAIN_WINDOW_X: i32 = 100;
    const OLD_MAIN_WINDOW_Y: i32 = 200;
    const NEW_MAIN_WINDOW_X: i32 = 150;
    const NEW_MAIN_WINDOW_Y: i32 = 250;
    let mut settings = MockSettings::new();
    settings.set_main_window_x(OLD_MAIN_WINDOW_X);
    settings.set_main_window_y(OLD_MAIN_WINDOW_Y);
    let mut controller = create_controller(settings, true);
    controller.init();
    assert_that!(mock_midi().start_instrument_connection_monitor_count, eq(1));
    let result = controller.close(NEW_MAIN_WINDOW_X, NEW_MAIN_WINDOW_Y);
    assert_that!(result, ok(()));
    assert_that!(mock_midi().close_count, eq(1));
    assert_that!(mock_osc().stop_count, eq(1));
    assert_that!(mock_settings().main_window_x, eq(NEW_MAIN_WINDOW_X));
    assert_that!(mock_settings().main_window_y, eq(NEW_MAIN_WINDOW_Y));
}

#[googletest::gtest]
fn close_err() {
    let _guard = test_mutex_guard();
    const ERR_MSG: &str = "Test error";
    const OLD_MAIN_WINDOW_X: i32 = 100;
    const OLD_MAIN_WINDOW_Y: i32 = 200;
    const NEW_MAIN_WINDOW_X: i32 = 150;
    const NEW_MAIN_WINDOW_Y: i32 = 250;
    let mut settings = MockSettings::new();
    settings.set_main_window_x(OLD_MAIN_WINDOW_X);
    settings.set_main_window_y(OLD_MAIN_WINDOW_Y);
    MockSettings::simulate_write_to_file_err(ERR_MSG);
    let mut controller = create_controller(settings, true);
    controller.init();
    assert_that!(mock_midi().start_instrument_connection_monitor_count, eq(1));
    let result = controller.close(NEW_MAIN_WINDOW_X, NEW_MAIN_WINDOW_Y);
    assert_that!(result, err(displays_as(eq(ERR_MSG))));
    assert_that!(mock_midi().close_count, eq(1));
    assert_that!(mock_osc().stop_count, eq(1));
    assert_that!(mock_settings().main_window_x, eq(NEW_MAIN_WINDOW_X));
    assert_that!(mock_settings().main_window_y, eq(NEW_MAIN_WINDOW_Y));
    assert_that!(mock_ui_methods().show_message_msg, some(eq(ERR_MSG)));
    assert_that!(mock_ui_methods().show_message_msg_type, some(eq(MessageType::Error)));
}

#[googletest::gtest]
fn on_receiving_data_started_show_waiting_for_download() {
    let _guard = test_mutex_guard();
    let mut controller = create_controller(MockSettings::new(), true);
    controller.init();
    MockMidi::simulate_receiving_data_started();
    assert_that!(mock_ui_methods().show_message_msg, some(eq(WAITING_FOR_DATA_DOWNLOAD)));
    assert_that!(mock_ui_methods().show_message_msg_type, some(eq(MessageType::Info)));
}

#[googletest::gtest]
fn on_data_download_started() {
    let _guard = test_mutex_guard();
    let mut controller = create_controller(MockSettings::new(), true);
    controller.init();
    MockMidi::set_are_devices_connected(true);
    MockMidi::simulate_download_started();
    assert_that!(mock_ui_methods().show_message_msg, some(eq(AWAITING_DATA_DOWNLOAD_COMPLETION)));
    assert_that!(mock_ui_methods().show_message_msg_type, some(eq(MessageType::Info)));
}

#[googletest::gtest]
fn on_data_download_completed_start_osc() {
    let _guard = test_mutex_guard();
    let mut controller = create_controller(MockSettings::new(), true);
    controller.init();
    MockMidi::set_is_receiving_data(true);
    MockMidi::set_are_devices_connected(true);
    MockMidi::simulate_download_completed();
    assert_that!(mock_osc().start_count, eq(1));
    assert_that!(mock_ui_methods().show_message_msg, some(eq(OPENING_PITCHGRID_CONNECTION)));
    assert_that!(mock_ui_methods().show_message_msg_type, some(eq(MessageType::Info)));
}

#[googletest::gtest]
fn on_tuning_received() {
    let _guard = test_mutex_guard();
    let mut controller = create_controller(MockSettings::new(), true);
    controller.init();
    MockMidi::set_is_receiving_data(true);
    MockMidi::set_are_devices_connected(true);
    MockMidi::simulate_download_completed();
    MockOsc::simulate_tuning_received(TestTunings::params_16_16());
    assert_that!(tuner().has_data(), eq(true));
    assert_that!(mock_ui_methods().show_pitchgrid_status_count, eq(1));
    assert_that!(mock_ui_methods().show_pitchgrid_status_msg, some(eq(UPDATING_INSTRUMENT_TUNING)));
    assert_that!(mock_ui_methods().show_pitchgrid_status_msg_type, some(eq(MessageType::Info)));
}

#[googletest::gtest]
fn on_tuning_received_when_instrument_disconnected() {
    let _guard = test_mutex_guard();
    let mut controller = create_controller(MockSettings::new(), true);
    controller.init();
    MockMidi::set_is_receiving_data(true);
    MockMidi::set_are_devices_connected(true);
    MockMidi::simulate_download_completed();
    MockOsc::simulate_tuning_received(TestTunings::params_16_16());
    MockMidi::set_is_receiving_data(false);
    MockOsc::simulate_tuning_received(TestTunings::params_17_17());
    assert_that!(tuner().has_data(), eq(false));
    assert_that!(mock_ui_methods().show_pitchgrid_status_msg, some(eq(CANNOT_UPDATE_TUNING_LOST)));
    assert_that!(mock_ui_methods().show_pitchgrid_status_msg_type, some(eq(MessageType::Error)));
}

#[googletest::gtest]
fn on_updating_tuning() {
    let _guard = test_mutex_guard();
    let mut controller = create_controller(MockSettings::new(), true);
    controller.init();
    MockMidi::simulate_updating_tuning();
    assert_that!(mock_ui_methods().show_message_msg_type, some(not(eq(MessageType::Error))));
}

#[googletest::gtest]
fn on_tuning_updated() {
    let _guard = test_mutex_guard();
    const NOTE_INDEX: usize = 1;
    let mut controller = create_controller(MockSettings::new(), true);
    controller.init();
    controller.set_root_freq_override(NOTE_INDEX);
    MockMidi::set_is_receiving_data(true);
    MockMidi::set_are_devices_connected(true);
    MockMidi::simulate_download_completed();
    MockOsc::simulate_tuning_received(TestTunings::params_16_16());
    MockMidi::simulate_updating_tuning();
    MockMidi::simulate_tuning_updated();
    assert_that!(tuner().has_data(), eq(true));
    assert_that!(tuner().is_root_freq_overridden(), eq(true));
    assert_that!(mock_ui_methods().show_tuning_count, eq(1));
    assert_that!(mock_ui_methods().show_tuning_is_root_freq_overridden, some(eq(true)));
    assert_that!(mock_ui_methods().show_tuning_formatted_tuning,
        some(eq(&tuner().formatted_tuning_params())));
    assert_that!(mock_ui_methods().show_pitchgrid_status_msg, some(eq(INSTRUMENT_TUNING_UPDATED)));
    assert_that!(mock_ui_methods().show_pitchgrid_status_msg_type, some(eq(MessageType::Info)));
}

#[googletest::gtest]
fn on_new_preset_selected() {
    let _guard = test_mutex_guard();
    const NOTE_INDEX: usize = 1;
    let mut controller = create_controller(MockSettings::new(), true);
    controller.init();
    controller.set_root_freq_override(NOTE_INDEX);
    MockMidi::set_is_receiving_data(true);
    MockMidi::set_are_devices_connected(true);
    MockMidi::simulate_download_completed();
    MockOsc::simulate_tuning_received(TestTunings::params_16_16());
    assert_that!(mock_ui_methods().show_pitchgrid_status_count, eq(1));
    MockMidi::simulate_updating_tuning();
    MockMidi::simulate_tuning_updated();
    MockMidi::simulate_new_preset_selected();
    assert_that!(tuner().has_data(), eq(true));
    assert_that!(mock_ui_methods().show_pitchgrid_status_msg, some(eq(NEW_PRESET_SELECTED)));
    assert_that!(mock_ui_methods().show_pitchgrid_status_msg_type, some(eq(MessageType::Info)));
}

#[googletest::gtest]
fn set_root_freq_override() {
    let _guard = test_mutex_guard();
    const NOTE_INDEX: usize = 1;
    let mut controller = create_controller(MockSettings::new(), true);
    controller.init();
    MockMidi::set_is_receiving_data(true);
    MockMidi::set_are_devices_connected(true);
    MockMidi::simulate_download_completed();
    assert_that!(mock_ui_methods().show_pitchgrid_status_count, eq(0));
    MockOsc::simulate_pitchgrid_connected_changed(true);
    assert_that!(mock_ui_methods().show_pitchgrid_status_count, eq(1));
    controller.set_root_freq_override(NOTE_INDEX);
    assert_that!(mock_ui_methods().show_pitchgrid_status_count, eq(2));
    assert_that!(mock_ui_methods().show_pitchgrid_status_msg, some(eq(UPDATING_ROOT_FREQ_OVERRIDE)));
    assert_that!(mock_ui_methods().show_pitchgrid_status_msg_type, some(eq(MessageType::Info)));
    assert_that!(tuner().is_root_freq_overridden(), eq(true));
}

#[googletest::gtest]
fn set_override_rounding_initial() {
    let _guard = test_mutex_guard();
    const OVERRIDE_ROUNDING_INITIAL: bool = false; // as the default is true
    let mut controller = create_controller(MockSettings::new(), true);
    controller.init();
    assert_that!(mock_settings().override_rounding_initial, eq(true));
    controller.set_override_rounding_initial(OVERRIDE_ROUNDING_INITIAL);
    assert_that!(mock_settings().override_rounding_initial, eq(OVERRIDE_ROUNDING_INITIAL));
}

#[googletest::gtest]
fn set_override_rounding_rate() {
    let _guard = test_mutex_guard();
    const OVERRIDE_ROUNDING_RATE: bool = false; // as the default is true
    let mut controller = create_controller(MockSettings::new(), true);
    controller.init();
    assert_that!(mock_settings().override_rounding_rate, eq(true));
    controller.set_override_rounding_rate(OVERRIDE_ROUNDING_RATE);
    assert_that!(mock_settings().override_rounding_rate, eq(OVERRIDE_ROUNDING_RATE));
}

#[googletest::gtest]
fn set_rounding_rate() {
    let _guard = test_mutex_guard();
    const ROUNDING_RATE: u8 = 100;
    let mut controller = create_controller(MockSettings::new(), true);
    controller.init();
    assert_that!(mock_settings().rounding_rate, eq(127));
    controller.set_rounding_rate(ROUNDING_RATE);
    assert_that!(mock_settings().rounding_rate, eq(ROUNDING_RATE));
}

#[googletest::gtest]
fn set_osc_listening_port() {
    let _guard = test_mutex_guard();
    const LISTENING_PORT: u16 = 34560;
    const LISTENING_PORT_INDEX: usize = 0;
    let mut controller = create_controller(MockSettings::new(), true);
    controller.init();
    assert_that!(mock_settings().osc_listening_port, eq(0)); // Unspecified
    assert_that!(mock_osc().listening_port, some(eq(Osc::default_listening_port())));
    controller.set_osc_listening_port(LISTENING_PORT_INDEX);
    assert_that!(mock_settings().osc_listening_port, eq(LISTENING_PORT));
    assert_that!(mock_osc().listening_port, some(eq(LISTENING_PORT)));
}

#[googletest::gtest]
fn set_pitch_table() {
    let _guard = test_mutex_guard();
    const PITCH_TABLE: u8 = 81;
    const PITCH_TABLE_INDEX: usize = 1;
    let mut controller = create_controller(MockSettings::new(), true);
    controller.init();
    assert_that!(mock_settings().pitch_table, eq(0)); // Unspecified
    assert_that!(tuner().pitch_table_index(), eq(0));
    controller.set_pitch_table(PITCH_TABLE_INDEX);
    assert_that!(mock_settings().pitch_table, eq(PITCH_TABLE));
    assert_that!(tuner().pitch_table_index(), eq(PITCH_TABLE_INDEX));
}

#[googletest::gtest]
fn on_pitchgrid_disconnected() {
    let _guard = test_mutex_guard();
    let mut controller = create_controller(MockSettings::new(), true);
    controller.init();
    MockMidi::set_is_receiving_data(true);
    MockMidi::set_are_devices_connected(true);
    MockMidi::simulate_download_completed();
    MockOsc::simulate_tuning_received(TestTunings::params_16_16());
    MockMidi::simulate_tuning_updated();
    assert_that!(tuner().has_data(), eq(true));
    MockOsc::simulate_pitchgrid_connected_changed(false);
    assert_that!(tuner().has_data(), eq(false));
    assert_that!(mock_ui_methods().show_pitchgrid_status_msg, some(eq(PITCHGRID_NOT_CONNECTED)));
    assert_that!(mock_ui_methods().show_pitchgrid_status_msg_type, some(eq(MessageType::Error)));
    assert_that!(mock_ui_methods().show_message_msg, some(eq(AWAITING_PITCHGRID_CONNECTION)));
    assert_that!(mock_ui_methods().show_message_msg_type, some(eq(MessageType::Warning)));
}

#[googletest::gtest]
fn on_receiving_data_stopped() {
    let _guard = test_mutex_guard();
    let mut controller = create_controller(MockSettings::new(), true);
    controller.init();
    MockOsc::set_is_running_result(true);
    MockMidi::simulate_receiving_data_stopped();
    assert_that!(mock_ui_methods().show_message_msg, some(eq(INSTRUMENT_NOT_CONNECTED)));
    assert_that!(mock_ui_methods().show_message_msg_type, some(eq(MessageType::Warning)));
    assert_that!(mock_osc().stop_count, eq(1));
    assert_that!(tuner().has_data(), eq(false));
    assert_that!(mock_ui_methods().show_pitchgrid_status_msg, some(eq(CANNOT_UPDATE_TUNING_LOST)));
    assert_that!(mock_ui_methods().show_pitchgrid_status_msg_type, some(eq(MessageType::Error)));
}

fn create_controller(mut settings: MockSettings, default_midi_devices: bool) -> Controller {
    if default_midi_devices {
        settings.set_midi_input_device(&INPUT_DEVICE_NAMES[0]);
        settings.set_midi_output_device(&OUTPUT_DEVICE_NAMES[0]);
    }
    Midi::set_midi(MockMidi::new(
        INPUT_DEVICE_NAMES.clone(), OUTPUT_DEVICE_NAMES.clone(),
        settings.midi_input_device(), settings.midi_output_device()));
    let new_tuner = Arc::new(Tuner::new());
    new_tuner.init(Tuner::default_pitch_table());
    new_tuner.set_midi_sender(Box::new(MockMidiSender::new()));
    *TUNER.lock().unwrap_or_else(|e| e.into_inner()) = new_tuner.clone();
    // Controller::init calls clone_controller(), which requires the CONTROLLER singleton to be set.
    // In main, the same shared instance is used for both set_controller and init. Here we use
    // separate instances: the local controller is used to call init() directly without locking a
    // shared controller, while the singleton is used only for MIDI callbacks. Both are configured
    // with MockOsc so that callback-triggered OSC calls are recorded in mock_osc().
    let mut singleton = Controller::new(Box::new(MockUiMethods::new()));
    singleton.set_osc(Box::new(MockOsc::new()));
    singleton.set_tuner(new_tuner.clone() as Arc<dyn ITuner>);
    Controller::set_controller(Arc::new(Mutex::new(singleton)));
    let mut controller = Controller::new(Box::new(MockUiMethods::new()));
    controller.set_osc(Box::new(MockOsc::new()));
    controller.set_settings(Box::new(settings));
    controller.set_tuner(new_tuner as Arc<dyn ITuner>);
    controller
}

/// To avoid races on static data, hold the returned guard in each test to ensure sequential
/// execution of tests.
fn test_mutex_guard() -> MutexGuard<'static, ()> {
    TEST_MUTEX.lock().unwrap_or_else(|e| e.into_inner())
}

fn tuner() -> MutexGuard<'static, Arc<Tuner>> {
    TUNER.lock().unwrap_or_else(|e| e.into_inner())
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

static TUNER: LazyLock<Mutex<Arc<Tuner>>> = LazyLock::new(|| Mutex::new(Arc::new(Tuner::new())));
