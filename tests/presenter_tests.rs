mod mock_midi_manager;
mod mock_continuum_protocol;
// `mock_midi_sender()` is used only by `tuner_tests`, which shares this mock.  Each is compiled
// separately. So there would be a compiler warning.
// That is suppressed by the `#[allow(dead_code)]` annotation.
#[allow(dead_code)] mod mock_midi_sender;
mod mock_osc;
mod mock_settings;
mod mock_ui_methods;

// `mod test_tunings` is declared twice, here and in `tests/tuner_tests.rs`.
// Each is compiled separately. So there would be a compiler warning for any `test_tunings`
// functions that are not used in this module.
// The warnings are suppressed by the `#[allow(dead_code)]` annotation.
#[allow(dead_code)] mod test_tunings;

use std::any::type_name_of_val;
use std::sync::{Arc, LazyLock, Mutex, MutexGuard};
use std::thread::sleep;
use std::time::Duration;
use googletest::assert_that;
use googletest::matchers::{
    contains_substring, displays_as, eq, err, len, ok, not, some, starts_with};
use log::{debug};
use slint::ComponentHandle;
use pitchgrid_continuum::{MainWindow};
use pitchgrid_continuum::presenter::Presenter;
use pitchgrid_continuum::presentation::{AWAITING_DATA_DOWNLOAD_COMPLETION, AWAITING_PITCHGRID_CONNECTION, CANNOT_UPDATE_TUNING_LOST, CHECKING_INSTRUMENT_CONNECTION, DEVICE_NONE, DISCONNECTED_FROM_PITCHGRID, INSTRUMENT_DISCONNECTED, INSTRUMENT_NOT_CONNECTED, INSTRUMENT_TUNING_UPDATE_NOT_CONFIRMED, INSTRUMENT_TUNING_UPDATED, MIDI_SEND_ERROR, OPENING_PITCHGRID_CONNECTION, PITCHGRID_CONNECTION_CLOSED, PITCHGRID_NOT_CONNECTED, PRESET_TUNING_LOADED, UPDATING_INSTRUMENT_TUNING, UPDATING_ROOT_FREQ_OVERRIDE, WAITING_FOR_DATA_DOWNLOAD};
use pitchgrid_continuum::global::{MessageType, DeviceType};
use pitchgrid_continuum::i_settings::ISettings;
use pitchgrid_continuum::osc::Osc;
use pitchgrid_continuum::device_strategy::{InputStrategy, OutputStrategy};
use pitchgrid_continuum::tuner::Tuner;
use mock_midi_manager::{MockMidiManager, mock_midi};
use mock_midi_manager::mock_io::{input_state, output_state};
use mock_continuum_protocol::MockContinuumProtocol;
use mock_osc::{MockOsc, mock_osc};
use mock_settings::{MockSettings, mock_settings};
use mock_midi_sender::MockMidiSender;
use mock_ui_methods::{MockUiMethods, mock_ui_methods};
use pitchgrid_continuum::midi_sender::{SharedMidiSender};
use pitchgrid_continuum::ui_methods::UiMethods;
use test_tunings::TestTunings;

#[googletest::gtest]
fn init_from_settings() {
    let _guard = test_mutex_guard();
    debug!("init_from_settings");
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
    let presenter = create_presenter(settings, true);
    presenter.lock().unwrap().init(&presenter);
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
    debug!("init_no_settings");
    let presenter = create_presenter(MockSettings::new(), false);
    presenter.lock().unwrap().init(&presenter);
    assert_that!(mock_ui_methods().main_window_position_x, some(eq(0)));
    assert_that!(mock_ui_methods().main_window_position_y, some(eq(0)));
    assert_that!(mock_ui_methods().set_devices_model_count, eq(2));
    assert_that!(mock_ui_methods().set_devices_model_device_names, some(len(eq(4))));
    // Extract the value rather than holding the `mock_ui_methods()` guard: it locks a
    // non-reentrant Mutex, so keeping it alive across the later `mock_ui_methods()` calls
    // would deadlock.
    let device_type = mock_ui_methods().set_devices_model_device_strategy
        .as_ref().map(|s| *s.device_type());
    assert_that!(device_type, some(eq(DeviceType::Output)));
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
    debug!("init_read_settings_err");
    const ERR_MSG: &str = "Test error";
    let settings = MockSettings::new();
    MockSettings::simulate_read_from_file_err(ERR_MSG);
    let presenter = create_presenter(settings, true);
    presenter.lock().unwrap().init(&presenter);
    assert_that!(mock_ui_methods().show_message_count, eq(1));
    assert_that!(mock_ui_methods().show_message_msg, some(eq(ERR_MSG)));
    assert_that!(mock_ui_methods().show_message_msg_type, some(eq(MessageType::Error)));
    assert_that!(mock_midi().start_instrument_connection_monitor_count, eq(0));
}

#[googletest::gtest]
fn connect_device() {
    let _guard = test_mutex_guard();
    debug!("connect_device");
    let presenter = create_presenter(MockSettings::new(), true);
    presenter.lock().unwrap().init(&presenter);
    assert_that!(mock_midi().start_instrument_connection_monitor_count, eq(1));
    MockMidiManager::set_is_receiving_data(true);
    MockMidiManager::set_are_devices_connected(true);
    MockContinuumProtocol::simulate_download_completed();
    MockOsc::simulate_tuning_received(TestTunings::params_17_17());
    MockContinuumProtocol::simulate_updating_tuning();
    MockContinuumProtocol::simulate_tuning_updated();
    assert_that!(tuner().has_data(), eq(true));
    assert_that!(tuner().formatted_tuning_params().root_freq, not(eq("")));
    let device_strategy = InputStrategy::new();
    MockUiMethods::set_selected_device_index(1);
    presenter.lock().unwrap().connect_device(&device_strategy);
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
    debug!("connect_device_after_refreshing_other_device_list");
    let presenter = create_presenter(MockSettings::new(), true);
    presenter.lock().unwrap().init(&presenter);
    assert_that!(mock_midi().start_instrument_connection_monitor_count, eq(1));
    MockMidiManager::set_is_receiving_data(true);
    MockMidiManager::set_are_devices_connected(true);
    MockContinuumProtocol::simulate_download_completed();
    MockOsc::simulate_tuning_received(TestTunings::params_17_17());
    MockContinuumProtocol::simulate_updating_tuning();
    MockContinuumProtocol::simulate_tuning_updated();
    let output_strategy = OutputStrategy::new();
    presenter.lock().unwrap().refresh_devices(&output_strategy);
    let input_strategy = InputStrategy::new();
    MockUiMethods::set_selected_device_index(1);
    presenter.lock().unwrap().connect_device(&input_strategy);
    assert_that!(mock_ui_methods().show_message_msg, some(eq("Connect MIDI output device")));
    assert_that!(mock_ui_methods().show_message_msg_type, some(eq(MessageType::Warning)));
    assert_that!(mock_midi().start_instrument_connection_monitor_count, eq(1));
}

#[googletest::gtest]
fn connect_device_err() {
    let _guard = test_mutex_guard();
    debug!("connect_device_err");
    const ERR_MSG: &str = "Test error";
    let presenter = create_presenter(MockSettings::new(), false);
    presenter.lock().unwrap().init(&presenter);
    assert_that!(mock_midi().start_instrument_connection_monitor_count, eq(0));
    let device_strategy = OutputStrategy::new();
    MockUiMethods::set_selected_device_index(1);
    MockMidiManager::simulate_connect_device_err(ERR_MSG);
    presenter.lock().unwrap().connect_device(&device_strategy);
    assert_that!(mock_ui_methods().show_message_msg, some(eq(ERR_MSG)));
    assert_that!(mock_ui_methods().show_message_msg_type, some(eq(MessageType::Error)));
}

#[googletest::gtest]
fn connect_device_with_no_selection_is_silent_no_op() {
    // An empty device list leaves the combobox unselected (-1), which `UiMethods` converts to
    // `usize::MAX`. The `device_names().get(index)` guard in `connect_selected_device` must reject
    // that silently: no MIDI connection attempt and no "connected" feedback (and no panic).
    let _guard = test_mutex_guard();
    debug!("connect_device_with_no_selection_is_silent_no_op");
    let presenter = create_presenter(MockSettings::new(), false);
    presenter.lock().unwrap().init(&presenter);
    let device_strategy = InputStrategy::new();
    MockUiMethods::set_selected_device_index(usize::MAX);
    let connect_before = mock_midi().connect_device_count;
    let show_connected_before = mock_ui_methods().show_connected_device_name_count;
    presenter.lock().unwrap().connect_device(&device_strategy);
    // The MIDI layer was never asked to connect, and no device-connected name was shown.
    assert_that!(mock_midi().connect_device_count, eq(connect_before));
    assert_that!(mock_ui_methods().show_connected_device_name_count, eq(show_connected_before));
}

#[googletest::gtest]
fn refresh_devices() {
    let _guard = test_mutex_guard();
    debug!("refresh_devices");
    let presenter = create_presenter(MockSettings::new(), true);
    presenter.lock().unwrap().init(&presenter);
    assert_that!(mock_ui_methods().set_devices_model_count, eq(2));
    MockMidiManager::set_is_receiving_data(true);
    MockMidiManager::set_are_devices_connected(true);
    MockContinuumProtocol::simulate_download_completed();
    MockOsc::simulate_tuning_received(TestTunings::params_16_16());
    MockContinuumProtocol::simulate_updating_tuning();
    MockContinuumProtocol::simulate_tuning_updated();
    assert_that!(mock_ui_methods().show_tuning_count, eq(1));
    let input_strategy = InputStrategy::new();
    presenter.lock().unwrap().refresh_devices(&input_strategy);
    assert_that!(mock_midi().stop_instrument_connection_monitor_count, eq(1));
    assert_that!(mock_osc().stop_count, eq(1));
    assert_that!(tuner().has_data(), eq(false));
    assert_that!(tuner().formatted_tuning_params().root_freq, eq(""));
    assert_that!(mock_ui_methods().show_tuning_count, eq(2));
    assert_that!(mock_ui_methods().show_pitchgrid_status_msg, some(eq(DISCONNECTED_FROM_PITCHGRID)));
    assert_that!(mock_ui_methods().show_pitchgrid_status_msg_type, some(eq(MessageType::Warning)));
    assert_that!(mock_ui_methods().set_devices_model_count, eq(3));
    // Extract the value rather than holding the `mock_ui_methods()` guard: it locks a
    // non-reentrant Mutex, so keeping it alive across the later `mock_ui_methods()` calls
    // would deadlock.
    let device_type = mock_ui_methods().set_devices_model_device_strategy
        .as_ref().map(|s| *s.device_type());
    assert_that!(device_type, some(eq(DeviceType::Input)));
    assert_that!(mock_ui_methods().show_connected_device_name_name, some(eq(DEVICE_NONE)));
    assert_that!(mock_ui_methods().show_message_msg, some(starts_with("Refreshed MIDI input devices.")));
    assert_that!(mock_ui_methods().show_message_msg_type, some(eq(MessageType::Warning)));
}

#[googletest::gtest]
fn on_devices_connected_changed_to_connected() {
    let _guard = test_mutex_guard();
    debug!("on_devices_connected_changed_to_connected");
    let presenter = create_presenter(MockSettings::new(), true);
    presenter.lock().unwrap().init(&presenter);
    MockMidiManager::set_are_devices_connected(true);
    MockOsc::set_is_running_result(true);
    MockContinuumProtocol::simulate_devices_connected_changed();
    assert_that!(mock_osc().stop_count, eq(0));
}

#[googletest::gtest]
fn on_devices_connected_changed_to_not_connected() {
    let _guard = test_mutex_guard();
    debug!("on_devices_connected_changed_to_not_connected");
    let presenter = create_presenter(MockSettings::new(), true);
    presenter.lock().unwrap().init(&presenter);
    MockMidiManager::set_are_devices_connected(false);
    MockOsc::set_is_running_result(true);
    MockContinuumProtocol::simulate_devices_connected_changed();
    assert_that!(mock_osc().stop_count, eq(1));
    assert_that!(mock_ui_methods().show_message_msg, some(eq(INSTRUMENT_DISCONNECTED)));
    assert_that!(mock_ui_methods().show_message_msg_type, some(eq(MessageType::Warning)));
    assert_that!(mock_ui_methods().show_pitchgrid_status_msg, some(eq(PITCHGRID_CONNECTION_CLOSED)));
    assert_that!(mock_ui_methods().show_pitchgrid_status_msg_type, some(eq(MessageType::Warning)));
}

#[googletest::gtest]
fn on_devices_connected_changed_to_not_connected_osc_not_running() {
    let _guard = test_mutex_guard();
    debug!("on_devices_connected_changed_to_not_connected");
    let presenter = create_presenter(MockSettings::new(), true);
    presenter.lock().unwrap().init(&presenter);
    MockMidiManager::set_are_devices_connected(false);
    MockOsc::set_is_running_result(false);
    MockContinuumProtocol::simulate_devices_connected_changed();
    assert_that!(mock_osc().stop_count, eq(0));
}

#[googletest::gtest]
fn close() {
    let _guard = test_mutex_guard();
    debug!("close");
    const OLD_MAIN_WINDOW_X: i32 = 100;
    const OLD_MAIN_WINDOW_Y: i32 = 200;
    const NEW_MAIN_WINDOW_X: i32 = 150;
    const NEW_MAIN_WINDOW_Y: i32 = 250;
    let mut settings = MockSettings::new();
    settings.set_main_window_x(OLD_MAIN_WINDOW_X);
    settings.set_main_window_y(OLD_MAIN_WINDOW_Y);
    let presenter = create_presenter(settings, true);
    presenter.lock().unwrap().init(&presenter);
    assert_that!(mock_midi().start_instrument_connection_monitor_count, eq(1));
    let result = presenter.lock().unwrap().close(NEW_MAIN_WINDOW_X, NEW_MAIN_WINDOW_Y);
    assert_that!(result, ok(()));
    assert_that!(mock_midi().close_count, eq(1));
    assert_that!(mock_osc().stop_count, eq(1));
    assert_that!(mock_settings().main_window_x, eq(NEW_MAIN_WINDOW_X));
    assert_that!(mock_settings().main_window_y, eq(NEW_MAIN_WINDOW_Y));
}

#[googletest::gtest]
fn close_err() {
    let _guard = test_mutex_guard();
    debug!("close_err");
    const ERR_MSG: &str = "Test error";
    const OLD_MAIN_WINDOW_X: i32 = 100;
    const OLD_MAIN_WINDOW_Y: i32 = 200;
    const NEW_MAIN_WINDOW_X: i32 = 150;
    const NEW_MAIN_WINDOW_Y: i32 = 250;
    let mut settings = MockSettings::new();
    settings.set_main_window_x(OLD_MAIN_WINDOW_X);
    settings.set_main_window_y(OLD_MAIN_WINDOW_Y);
    MockSettings::simulate_write_to_file_err(ERR_MSG);
    let presenter = create_presenter(settings, true);
    presenter.lock().unwrap().init(&presenter);
    assert_that!(mock_midi().start_instrument_connection_monitor_count, eq(1));
    let result = presenter.lock().unwrap().close(NEW_MAIN_WINDOW_X, NEW_MAIN_WINDOW_Y);
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
    debug!("on_receiving_data_started_show_waiting_for_download");
    let presenter = create_presenter(MockSettings::new(), true);
    presenter.lock().unwrap().init(&presenter);
    MockContinuumProtocol::simulate_receiving_data_started();
    assert_that!(mock_ui_methods().show_message_msg, some(eq(WAITING_FOR_DATA_DOWNLOAD)));
    assert_that!(mock_ui_methods().show_message_msg_type, some(eq(MessageType::Info)));
}

#[googletest::gtest]
fn on_receiving_data_started_output_not_connected() {
    let _guard = test_mutex_guard();
    debug!("on_receiving_data_started_output_disconnected");
    let presenter = create_presenter(MockSettings::new(), true);
    presenter.lock().unwrap().init(&presenter);
    // As we have just started receiving data from the instrument, the MIDI input must be
    // connected. So the following simulates the MIDI output not being connected.
    MockMidiManager::set_are_devices_connected(false);
    MockContinuumProtocol::simulate_receiving_data_started();
    // We don't want to show the WAITING_FOR_DATA_DOWNLOAD message, as that would
    // overwrite the "Connect MIDI output device" warning message that should already be displayed.
    assert_that!(mock_ui_methods().show_message_msg, some(not(eq(WAITING_FOR_DATA_DOWNLOAD))));
}

#[googletest::gtest]
fn on_data_download_started() {
    let _guard = test_mutex_guard();
    debug!("on_data_download_started");
    let presenter = create_presenter(MockSettings::new(), true);
    presenter.lock().unwrap().init(&presenter);
    MockMidiManager::set_are_devices_connected(true);
    MockContinuumProtocol::simulate_download_started();
    assert_that!(mock_ui_methods().show_message_msg, some(eq(AWAITING_DATA_DOWNLOAD_COMPLETION)));
    assert_that!(mock_ui_methods().show_message_msg_type, some(eq(MessageType::Info)));
}

#[googletest::gtest]
fn on_data_download_completed_not_connected() {
    let _guard = test_mutex_guard();
    debug!("on_data_download_completed_start_osc");
    let presenter = create_presenter(MockSettings::new(), true);
    presenter.lock().unwrap().init(&presenter);
    MockMidiManager::set_is_receiving_data(true);
    MockMidiManager::set_are_devices_connected(false);
    MockContinuumProtocol::simulate_download_completed();
    assert_that!(mock_osc().start_count, eq(0));
}

#[googletest::gtest]
fn on_data_download_completed_start_osc() {
    let _guard = test_mutex_guard();
    debug!("on_data_download_completed_start_osc");
    let presenter = create_presenter(MockSettings::new(), true);
    presenter.lock().unwrap().init(&presenter);
    MockMidiManager::set_is_receiving_data(true);
    MockMidiManager::set_are_devices_connected(true);
    MockContinuumProtocol::simulate_download_completed();
    assert_that!(mock_osc().start_count, eq(1));
    assert_that!(mock_ui_methods().show_message_msg, some(eq(OPENING_PITCHGRID_CONNECTION)));
    assert_that!(mock_ui_methods().show_message_msg_type, some(eq(MessageType::Info)));
}

#[googletest::gtest]
fn on_tuning_received() {
    let _guard = test_mutex_guard();
    debug!("on_tuning_received");
    let presenter = create_presenter(MockSettings::new(), true);
    presenter.lock().unwrap().init(&presenter);
    MockMidiManager::set_is_receiving_data(true);
    MockMidiManager::set_are_devices_connected(true);
    MockContinuumProtocol::simulate_download_completed();
    MockOsc::simulate_tuning_received(TestTunings::params_16_16());
    assert_that!(tuner().has_data(), eq(true));
    assert_that!(mock_ui_methods().show_pitchgrid_status_count, eq(1));
    assert_that!(mock_ui_methods().show_pitchgrid_status_msg, some(eq(UPDATING_INSTRUMENT_TUNING)));
    assert_that!(mock_ui_methods().show_pitchgrid_status_msg_type, some(eq(MessageType::Info)));
}

#[googletest::gtest]
fn on_tuning_received_when_instrument_disconnected() {
    let _guard = test_mutex_guard();
    debug!("on_tuning_received_when_instrument_disconnected");
    let presenter = create_presenter(MockSettings::new(), true);
    presenter.lock().unwrap().init(&presenter);
    MockMidiManager::set_is_receiving_data(true);
    MockMidiManager::set_are_devices_connected(true);
    MockContinuumProtocol::simulate_download_completed();
    MockOsc::simulate_tuning_received(TestTunings::params_16_16());
    MockMidiManager::set_is_receiving_data(false);
    MockOsc::simulate_tuning_received(TestTunings::params_17_17());
    assert_that!(tuner().has_data(), eq(false));
    assert_that!(mock_ui_methods().show_pitchgrid_status_msg, some(eq(CANNOT_UPDATE_TUNING_LOST)));
    assert_that!(mock_ui_methods().show_pitchgrid_status_msg_type, some(eq(MessageType::Error)));
}

#[googletest::gtest]
fn on_tuning_updated() {
    let _guard = test_mutex_guard();
    debug!("on_tuning_updated");
    const NOTE_INDEX: usize = 1;
    let presenter = create_presenter(MockSettings::new(), true);
    presenter.lock().unwrap().init(&presenter);
    presenter.lock().unwrap().set_root_freq_override(NOTE_INDEX);
    MockMidiManager::set_is_receiving_data(true);
    MockMidiManager::set_are_devices_connected(true);
    MockContinuumProtocol::simulate_download_completed();
    MockOsc::simulate_tuning_received(TestTunings::params_16_16());
    MockContinuumProtocol::simulate_updating_tuning();
    MockContinuumProtocol::simulate_tuning_updated();
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
fn on_tuning_updated_no_tuning_data() {
    let _guard = test_mutex_guard();
    debug!("on_tuning_updated");
    const NOTE_INDEX: usize = 1;
    let presenter = create_presenter(MockSettings::new(), true);
    presenter.lock().unwrap().init(&presenter);
    presenter.lock().unwrap().set_root_freq_override(NOTE_INDEX);
    MockMidiManager::set_is_receiving_data(true);
    MockMidiManager::set_are_devices_connected(true);
    MockContinuumProtocol::simulate_download_completed();
    MockOsc::simulate_tuning_received(TestTunings::params_16_16());
    MockContinuumProtocol::simulate_updating_tuning();
    tuner().remove_data();
    assert_that!(tuner().has_data(), eq(false));
    MockContinuumProtocol::simulate_tuning_updated();
    assert_that!(mock_ui_methods().show_pitchgrid_status_msg, some(not(eq(INSTRUMENT_TUNING_UPDATED))));
}

#[googletest::gtest]
fn tuning_update_midi_send_error() {
    let _guard = test_mutex_guard();
    debug!("tuning_update_not_confirmed");
    let presenter = create_presenter(MockSettings::new(), true);
    MockMidiSender::simulate_error(true);
    presenter.lock().unwrap().init(&presenter);
    MockMidiManager::set_is_receiving_data(true);
    MockMidiManager::set_are_devices_connected(true);
    MockContinuumProtocol::simulate_download_completed();
    MockOsc::simulate_tuning_received(TestTunings::params_16_16());
    MockContinuumProtocol::simulate_updating_tuning();
    // Wait for TuningUpdateWatchdog::run to time out and show the error message.
    sleep(Duration::from_millis(50));
    assert_that!(mock_ui_methods().show_message_msg_type, some(eq(MessageType::Error)));
    assert_that!(mock_ui_methods().show_message_msg, some(eq(MIDI_SEND_ERROR)));
}

#[googletest::gtest]
fn tuning_update_not_confirmed() {
    let _guard = test_mutex_guard();
    debug!("tuning_update_not_confirmed");
    let presenter = create_presenter(MockSettings::new(), true);
    presenter.lock().unwrap().init(&presenter);
    MockContinuumProtocol::simulate_updating_tuning();
    // Wait for TuningUpdateWatchdog::run to time out and show the error message.
    sleep(Duration::from_millis(50));
    assert_that!(mock_ui_methods().show_message_msg_type, some(eq(MessageType::Error)));
    assert_that!(mock_ui_methods().show_message_msg, some(eq(INSTRUMENT_TUNING_UPDATE_NOT_CONFIRMED)));
}

#[googletest::gtest]
fn on_new_preset_selected() {
    let _guard = test_mutex_guard();
    debug!("on_new_preset_selected");
    const NOTE_INDEX: usize = 1;
    let presenter = create_presenter(MockSettings::new(), true);
    presenter.lock().unwrap().init(&presenter);
    presenter.lock().unwrap().set_root_freq_override(NOTE_INDEX);
    MockMidiManager::set_is_receiving_data(true);
    MockMidiManager::set_are_devices_connected(true);
    MockContinuumProtocol::simulate_download_completed();
    MockOsc::simulate_tuning_received(TestTunings::params_16_16());
    assert_that!(mock_ui_methods().show_pitchgrid_status_count, eq(1));
    MockContinuumProtocol::simulate_updating_tuning();
    MockContinuumProtocol::simulate_tuning_updated();
    MockContinuumProtocol::simulate_new_preset_selected();
    // The instrument's confirmation echo for the resend. With the preset-reselect flag set,
    // on_tuning_updated shows the preset-specific confirmation rather than the generic one.
    MockContinuumProtocol::simulate_tuning_updated();
    assert_that!(tuner().has_data(), eq(true));
    assert_that!(mock_ui_methods().show_pitchgrid_status_msg, some(eq(PRESET_TUNING_LOADED)));
    assert_that!(mock_ui_methods().show_pitchgrid_status_msg_type, some(eq(MessageType::Info)));
}

#[googletest::gtest]
fn on_new_preset_selected_no_tuning_data() {
    let _guard = test_mutex_guard();
    debug!("on_new_preset_selected_no_tuning_data");
    const NOTE_INDEX: usize = 1;
    let presenter = create_presenter(MockSettings::new(), true);
    presenter.lock().unwrap().init(&presenter);
    presenter.lock().unwrap().set_root_freq_override(NOTE_INDEX);
    MockMidiManager::set_is_receiving_data(true);
    MockMidiManager::set_are_devices_connected(true);
    MockContinuumProtocol::simulate_download_completed();
    MockOsc::simulate_tuning_received(TestTunings::params_16_16());
    assert_that!(mock_ui_methods().show_pitchgrid_status_count, eq(1));
    tuner().remove_data();
    MockContinuumProtocol::simulate_new_preset_selected();
    assert_that!(tuner().has_data(), eq(false));
}

#[googletest::gtest]
fn set_root_freq_override() {
    let _guard = test_mutex_guard();
    debug!("set_root_freq_override");
    const NOTE_INDEX: usize = 1;
    let presenter = create_presenter(MockSettings::new(), true);
    presenter.lock().unwrap().init(&presenter);
    MockMidiManager::set_is_receiving_data(true);
    MockMidiManager::set_are_devices_connected(true);
    MockContinuumProtocol::simulate_download_completed();
    assert_that!(mock_ui_methods().show_pitchgrid_status_count, eq(0));
    MockOsc::simulate_pitchgrid_connected_changed(true);
    assert_that!(mock_ui_methods().show_pitchgrid_status_count, eq(1));
    presenter.lock().unwrap().set_root_freq_override(NOTE_INDEX);
    assert_that!(mock_ui_methods().show_pitchgrid_status_count, eq(2));
    assert_that!(mock_ui_methods().show_pitchgrid_status_msg, some(eq(UPDATING_ROOT_FREQ_OVERRIDE)));
    assert_that!(mock_ui_methods().show_pitchgrid_status_msg_type, some(eq(MessageType::Info)));
    assert_that!(tuner().is_root_freq_overridden(), eq(true));
}

#[googletest::gtest]
fn set_override_rounding_initial() {
    let _guard = test_mutex_guard();
    debug!("set_override_rounding_initial");
    const OVERRIDE_ROUNDING_INITIAL: bool = false; // as the default is true
    let presenter = create_presenter(MockSettings::new(), true);
    presenter.lock().unwrap().init(&presenter);
    assert_that!(mock_settings().override_rounding_initial, eq(true));
    presenter.lock().unwrap().set_override_rounding_initial(OVERRIDE_ROUNDING_INITIAL);
    assert_that!(mock_settings().override_rounding_initial, eq(OVERRIDE_ROUNDING_INITIAL));
}

#[googletest::gtest]
fn set_override_rounding_rate() {
    let _guard = test_mutex_guard();
    debug!("set_override_rounding_rate");
    const OVERRIDE_ROUNDING_RATE: bool = false; // as the default is true
    let presenter = create_presenter(MockSettings::new(), true);
    presenter.lock().unwrap().init(&presenter);
    assert_that!(mock_settings().override_rounding_rate, eq(true));
    presenter.lock().unwrap().set_override_rounding_rate(OVERRIDE_ROUNDING_RATE);
    assert_that!(mock_settings().override_rounding_rate, eq(OVERRIDE_ROUNDING_RATE));
}

#[googletest::gtest]
fn set_rounding_rate() {
    let _guard = test_mutex_guard();
    debug!("set_rounding_rate");
    const ROUNDING_RATE: u8 = 100;
    let presenter = create_presenter(MockSettings::new(), true);
    presenter.lock().unwrap().init(&presenter);
    assert_that!(mock_settings().rounding_rate, eq(127));
    presenter.lock().unwrap().set_rounding_rate(ROUNDING_RATE);
    assert_that!(mock_settings().rounding_rate, eq(ROUNDING_RATE));
}

#[googletest::gtest]
fn set_osc_listening_port() {
    let _guard = test_mutex_guard();
    debug!("set_osc_listening_port");
    const LISTENING_PORT: u16 = 34560;
    const LISTENING_PORT_INDEX: usize = 0;
    let presenter = create_presenter(MockSettings::new(), true);
    presenter.lock().unwrap().init(&presenter);
    assert_that!(mock_settings().osc_listening_port, eq(0)); // Unspecified
    assert_that!(mock_osc().listening_port, some(eq(Osc::default_listening_port())));
    presenter.lock().unwrap().set_osc_listening_port(LISTENING_PORT_INDEX);
    assert_that!(mock_settings().osc_listening_port, eq(LISTENING_PORT));
    assert_that!(mock_osc().listening_port, some(eq(LISTENING_PORT)));
}

#[googletest::gtest]
fn set_pitch_table() {
    let _guard = test_mutex_guard();
    debug!("set_pitch_table");
    const PITCH_TABLE: u8 = 81;
    const PITCH_TABLE_INDEX: usize = 1;
    let presenter = create_presenter(MockSettings::new(), true);
    presenter.lock().unwrap().init(&presenter);
    assert_that!(mock_settings().pitch_table, eq(0)); // Unspecified
    assert_that!(tuner().pitch_table_index(), eq(0));
    presenter.lock().unwrap().set_pitch_table(PITCH_TABLE_INDEX);
    assert_that!(mock_settings().pitch_table, eq(PITCH_TABLE));
    assert_that!(tuner().pitch_table_index(), eq(PITCH_TABLE_INDEX));
}

#[googletest::gtest]
fn on_pitchgrid_disconnected() {
    let _guard = test_mutex_guard();
    debug!("on_pitchgrid_disconnected");
    let presenter = create_presenter(MockSettings::new(), true);
    presenter.lock().unwrap().init(&presenter);
    MockMidiManager::set_is_receiving_data(true);
    MockMidiManager::set_are_devices_connected(true);
    MockContinuumProtocol::simulate_download_completed();
    MockOsc::simulate_tuning_received(TestTunings::params_16_16());
    MockContinuumProtocol::simulate_tuning_updated();
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
    debug!("on_pitchgrid_disconnected");
    let presenter = create_presenter(MockSettings::new(), true);
    presenter.lock().unwrap().init(&presenter);
    MockOsc::set_is_running_result(true);
    MockContinuumProtocol::simulate_receiving_data_stopped();
    assert_that!(mock_ui_methods().show_message_msg, some(eq(INSTRUMENT_NOT_CONNECTED)));
    assert_that!(mock_ui_methods().show_message_msg_type, some(eq(MessageType::Warning)));
    assert_that!(mock_osc().stop_count, eq(1));
    assert_that!(tuner().has_data(), eq(false));
    assert_that!(mock_ui_methods().show_pitchgrid_status_msg, some(eq(CANNOT_UPDATE_TUNING_LOST)));
    assert_that!(mock_ui_methods().show_pitchgrid_status_msg_type, some(eq(MessageType::Error)));
}

#[googletest::gtest]
fn on_receiving_data_stopped_osc_not_running_devices_not_connected() {
    let _guard = test_mutex_guard();
    debug!("on_pitchgrid_disconnected");
    let presenter = create_presenter(MockSettings::new(), true);
    presenter.lock().unwrap().init(&presenter);
    MockOsc::set_is_running_result(false);
    MockMidiManager::set_are_devices_connected(false);
    MockContinuumProtocol::simulate_receiving_data_stopped();
    assert_that!(mock_osc().stop_count, eq(0));
    assert_that!(mock_ui_methods().show_message_msg, some(not(eq(INSTRUMENT_NOT_CONNECTED))));
}

#[googletest::gtest]
fn production_new() {
    // Just so we can get 100% test coverage of `Presenter`,
    // test the production `Presenter::new`, which does not allow us to inject all the mocks.
    // (Other tests call `Presenter::new2` to specify the mocks: see `create_presenter`.)
    let main_window = MainWindow::new().unwrap();
    let ui_methods = UiMethods::new(main_window.as_weak());
    let presenter = Presenter::new(Arc::new(ui_methods));
    // There's no application code we can check. So I've added this banale assertion.
    assert_that!(type_name_of_val(&presenter), contains_substring("Presenter"));
}

fn create_presenter(mut mock_settings: MockSettings, default_midi_devices: bool)
                    -> Arc<Mutex<Presenter>> {
    if default_midi_devices {
        mock_settings.set_midi_input_device(&INPUT_DEVICE_NAMES[0]);
        mock_settings.set_midi_output_device(&OUTPUT_DEVICE_NAMES[0]);
    }
    let mock_midi_manager = MockMidiManager::new(
        INPUT_DEVICE_NAMES.clone(), OUTPUT_DEVICE_NAMES.clone(),
        mock_settings.midi_input_device(), mock_settings.midi_output_device());
    let mock_midi_sender: SharedMidiSender =
        Arc::new(Mutex::new(MockMidiSender::new()));
    let watchdog_notifier =
        mock_midi_sender.lock().unwrap().error_notifier().clone();
    let mock_continuum_protocol = MockContinuumProtocol::new();
    let tuner =
        Arc::new(Tuner::new(mock_continuum_protocol.clone(), mock_midi_sender.clone()));
    tuner.init(Tuner::default_pitch_table());
    *TUNER.lock().unwrap_or_else(|e| e.into_inner()) =
        Some(tuner.clone());
    // A single shared presenter serves as both the test subject and its own MIDI/OSC callback
    // target: init() (called by the test, passing &presenter) records a weak self-reference.
    // The mock MIDI, OSC, etc. are injected.
    // Tests lock the returned Arc to drive it; simulate_* callbacks lock it too, which is
    // deadlock-free because they release the mock lock before invoking the callback.
    // Instead of `Presenter::new`, call `Presenter::new2`, as that allows us to inject all the
    // mocks.
    let presenter =
        Arc::new(Mutex::new(Presenter::new2(
            Arc::new(MockUiMethods::new()), 1,
            mock_continuum_protocol.clone(),
            Arc::new(Mutex::new(mock_midi_manager)),
            Box::new(MockOsc::new()),
            Box::new(mock_settings),
            tuner,
            watchdog_notifier,
        )));
    presenter
}

/// To avoid races on static data, hold the returned guard in each test to ensure sequential
/// execution of tests.
fn test_mutex_guard() -> MutexGuard<'static, ()> {
    init_test_logging();
    TEST_MUTEX.lock().unwrap_or_else(|e| e.into_inner())
}

/// Initialises `env_logger` for the test binary. Uses `try_init` (not `init`) so the repeated calls
/// from every test are silent no-ops after the first rather than panicking on a second global-logger
/// install. `is_test(true)` routes output through libtest's capture, so log lines surface only for
/// FAILING tests (or with `--nocapture`). The level is still chosen at run time via `RUST_LOG`,
/// e.g. `RUST_LOG=debug cargo test connect_device -- --nocapture`.
fn init_test_logging() {
    let _ = env_logger::builder().is_test(true).format_timestamp_millis().try_init();
    // Show log lines non-failing tests too.
    // let _ = env_logger::builder().format_timestamp_millis().try_init();
}

fn tuner() -> Arc<Tuner> {
    TUNER.lock().unwrap_or_else(|e| e.into_inner()).clone().expect("TUNER must be initialized")
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

static TUNER: Mutex<Option<Arc<Tuner>>> = Mutex::new(None);
