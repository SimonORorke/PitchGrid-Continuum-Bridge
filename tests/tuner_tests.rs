mod mock_midi_sending;

use std::sync::Mutex;
use googletest::assert_that;
use googletest::matchers::{eq};
use pitchgrid_continuum::tuner::{self, ITuner, Tuner};
use pitchgrid_continuum::tuning_params::TuningParams;
use mock_midi_sending::{MockMidiSender, sent_midi};

// PITCH_TABLE is a shared static written by tuner.init() and tuner.set_pitch_table().
// Tests must run sequentially to avoid data races on it.
static TEST_MUTEX: Mutex<()> = Mutex::new(());

#[googletest::gtest]
fn on_tuning_received() {
    println!("***********************************");
    println!("on_tuning_received test started");
    println!("***********************************");
    let _guard = TEST_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
    let tuner = init_tuner();
    tuner.set_override_rounding_initial(true);
    tuner.set_override_rounding_rate(true);
    tuner.set_rounding_rate(MAX_ROUNDING_RATE);
    tuner.on_tuning_received(params_31_19());
    assert_that!(tuner.is_root_freq_overridden(), eq(false));
    // tuning start, key params 128 * 6, tuning end,
    // Rounding Initial, Rounding Rate, active pitch table
    assert_that!(sent_midi().control_change_count, eq(773));
    // Active pitch table
    assert_that!(sent_midi().last_control_change_channel, eq(16));
    assert_that!(sent_midi().last_control_change_cc_no, eq(51));
    assert_that!(sent_midi().last_control_change_value, eq(PITCH_TABLE));
    // Rounding Mode Normal, because Rounding Rate is on
    assert_that!(sent_midi().matrix_poke_count, eq(1));
    assert_that!(sent_midi().last_matrix_poke_id, eq(10));
    assert_that!(sent_midi().last_matrix_poke_value, eq(0));
    let formatted = tuner.formatted_tuning_params();
    assert_that!(formatted.root_freq, eq("261.626 Hz"));
    assert_that!(formatted.stretch, eq("1200 ct"));
    assert_that!(formatted.skew, eq("0.58065"));
    assert_that!(formatted.mode_offset, eq("8.25"));
    assert_that!(formatted.steps, eq("19"));
    assert_that!(formatted.mos_large_step_count, eq("5"));
    assert_that!(formatted.mos_small_step_count, eq("2"));
    println!("***********************************");
    println!("on_tuning_received test completed");
    println!("***********************************");
}

#[googletest::gtest]
fn remove_data() {
    println!("***********************************");
    println!("remove_data test started");
    println!("***********************************");
    let _guard = TEST_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
    let tuner = init_tuner();
    tuner.on_tuning_received(params_31_19());
    assert_that!(tuner.has_data(), eq(true));
    tuner.remove_data();
    assert_that!(tuner.has_data(), eq(false));
    let formatted = tuner.formatted_tuning_params();
    assert_that!(formatted.root_freq, eq(""));
    println!("***********************************");
    println!("remove_data test completed");
    println!("***********************************");
}

#[googletest::gtest]
fn send_current_preset_update() {
    println!("*****************************************");
    println!("send_current_preset_update test started");
    println!("*****************************************");
    let _guard = TEST_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
    let tuner = init_tuner();
    tuner.set_override_rounding_initial(true);
    tuner.set_override_rounding_rate(true);
    tuner.set_rounding_rate(MAX_ROUNDING_RATE);
    tuner.on_tuning_received(params_31_19());
    let first_time_sent_control_change_count = sent_midi().control_change_count;
    assert_that!(tuner.send_current_preset_update(), eq(true));
    let cumulative_sent_control_change_count = sent_midi().control_change_count;
    // send_current_preset_update should have sent only the rounding parameters and the
    // active pitch table CC (tuning data is not re-sent as it is assumed to already be on the
    // instrument).
    assert_that!(
        cumulative_sent_control_change_count, eq(first_time_sent_control_change_count + 3));
    // Rounding Mode Normal, because Rounding Rate is on
    assert_that!(sent_midi().matrix_poke_count, eq(2));
    println!("*****************************************");
    println!("send_current_preset_update test completed");
    println!("*****************************************");
}

#[googletest::gtest]
fn set_pitch_table() {
    let _guard = TEST_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
    let new_pitch_table: u8 = tuner::default_pitch_table();
    let tuner = init_tuner();
    assert_that!(tuner::pitch_table(), eq(PITCH_TABLE));
    tuner.set_pitch_table(new_pitch_table);
    assert_that!(tuner::pitch_table(), eq(new_pitch_table));
}

#[googletest::gtest]
fn set_root_freq_override_note_no() {
    let _guard = TEST_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
    let tuner = init_tuner();
    tuner.on_tuning_received(params_31_19());
    tuner.set_root_freq_override_note_no(4 /* A */, true);
    assert_that!(tuner.is_root_freq_overridden(), eq(true));
    let formatted = tuner.formatted_tuning_params();
    assert_that!(formatted.root_freq, eq("220 Hz"));
}

fn init_tuner() -> Tuner {
    let tuner = Tuner::new();
    tuner.init(PITCH_TABLE);
    tuner.set_midi_sender(Box::new(MockMidiSender::new()));
    tuner.remove_data(); // Ensure is_already_updating is false
    tuner
}

fn params_31_19() -> TuningParams {
    TuningParams::new(1, 261.62558, 0.99999994, 0.5806459,
                      8.250002, 19, 5, 2)
}

const MAX_ROUNDING_RATE: u8 = 127;
const PITCH_TABLE: u8 = 81;
