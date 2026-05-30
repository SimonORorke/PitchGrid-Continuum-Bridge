mod temp_path_finder;

use googletest::assert_that;
use googletest::matchers::{eq, ok};
use pitchgrid_continuum::i_settings::ISettings;
use pitchgrid_continuum::settings::Settings;
use temp_path_finder::TempPathFinder;

#[googletest::gtest]
fn persist() {
    const MAIN_WINDOW_X: i32 = 100;
    const MAIN_WINDOW_Y: i32 = 200;
    const MIDI_INPUT_DEVICE: &str = "Input Device 1";
    const MIDI_OUTPUT_DEVICE: &str = "Output Device 1";
    const OSC_LISTENING_PORT: u16 = 34563;
    const PITCH_TABLE: u8 = 81;
    const OVERRIDE_ROUNDING_INITIAL: bool = false; // as the default is true
    const OVERRIDE_ROUNDING_RATE: bool = false; // as the default is true
    const ROUNDING_RATE: u8 = 100;
    let mut settings = Settings::new();
    settings.set_main_window_x(MAIN_WINDOW_X);
    settings.set_main_window_y(MAIN_WINDOW_Y);
    settings.set_midi_input_device(MIDI_INPUT_DEVICE);
    settings.set_midi_output_device(MIDI_OUTPUT_DEVICE);
    settings.set_osc_listening_port(OSC_LISTENING_PORT);
    settings.set_override_rounding_initial(OVERRIDE_ROUNDING_INITIAL);
    settings.set_override_rounding_rate(OVERRIDE_ROUNDING_RATE);
    settings.set_rounding_rate(ROUNDING_RATE);
    let temp_path_finder = TempPathFinder::new();
    settings.set_system_path_finder(Box::new(temp_path_finder.clone()));
    settings.set_pitch_table(PITCH_TABLE);
    assert_that!(settings.write_to_file(), ok(()));
    settings = Settings::new();
    settings.set_system_path_finder(Box::new(temp_path_finder.clone()));
    assert_that!(settings.read_from_file(), ok(()));
    assert_that!(settings.main_window_x(), eq(MAIN_WINDOW_X));
    assert_that!(settings.main_window_y(), eq(MAIN_WINDOW_Y));
    assert_that!(settings.midi_input_device(), eq(MIDI_INPUT_DEVICE));
    assert_that!(settings.midi_output_device(), eq(MIDI_OUTPUT_DEVICE));
    assert_that!(settings.pitch_table(), eq(PITCH_TABLE));
    assert_that!(settings.osc_listening_port(), eq(OSC_LISTENING_PORT));
    assert_that!(settings.override_rounding_initial(), eq(OVERRIDE_ROUNDING_INITIAL));
    assert_that!(settings.override_rounding_rate(), eq(OVERRIDE_ROUNDING_RATE));
    assert_that!(settings.rounding_rate(), eq(ROUNDING_RATE));
}
