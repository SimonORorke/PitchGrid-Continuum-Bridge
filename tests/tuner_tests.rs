mod mock_midi_sender;

use std::sync::{Mutex, MutexGuard};
use googletest::assert_that;
use googletest::matchers::{eq, gt};
use pitchgrid_continuum::tuner::{ITuner, Tuner};
use pitchgrid_continuum::tuning_params::TuningParams;
use mock_midi_sender::{MockMidiSender, sent_midi};

/// PITCH_TABLE is a shared static written by tuner.init() and tuner.set_pitch_table().
/// Tests must run sequentially to avoid data races on it.
static TEST_MUTEX: Mutex<()> = Mutex::new(());

#[googletest::gtest]
fn on_tuning_received() {
    println!("***********************************");
    println!("on_tuning_received test started");
    println!("***********************************");
    let _guard = test_mutex_guard();
    let tuner = create_tuner();
    tuner.set_override_rounding_initial(true);
    tuner.set_override_rounding_rate(true);
    tuner.set_rounding_rate(MAX_ROUNDING_RATE);
    tuner.on_tuning_received(params_31_19());
    assert_that!(tuner.is_root_freq_overridden(), eq(false));
    // tuning start, key params 128 * 6, tuning end,
    // Rounding Initial, Rounding Rate, active pitch table
    assert_that!(sent_midi().control_change_count, eq(773));
    // Active pitch table
    assert_that!(sent_midi().control_change_channel, eq(16));
    assert_that!(sent_midi().control_change_cc_no, eq(51));
    assert_that!(sent_midi().control_change_value, eq(PITCH_TABLE));
    // Rounding Mode Normal, because Rounding Rate is on
    assert_that!(sent_midi().matrix_poke_count, eq(1));
    assert_that!(sent_midi().matrix_poke_id, eq(10));
    assert_that!(sent_midi().matrix_poke_value, eq(0));
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
fn on_tuning_updated() {
    let _guard = test_mutex_guard();
    let tuner = create_tuner();
    assert_that!(tuner.has_data(), eq(false));
    assert_that!(sent_midi().control_change_count, eq(0));
    // In case there's an on_tuning_updated notification when no tuning data has been received,
    // which should not happen, there should still have been no MIDI messages sent.
    tuner.on_tuning_updated();
    assert_that!(sent_midi().control_change_count, eq(0));
    // First tuning received.
    // No tuning updates are pending, so the tuning should be sent immediately.
    tuner.on_tuning_received(params_31_19());
    let single_tuning_control_change_count = sent_midi().control_change_count;
    println!("First tuning should have been sent: cumulative control_change_count = {}",
             sent_midi().control_change_count);
    // Check that the tuning has been sent.
    assert_that!(sent_midi().control_change_count, gt(0),
        "First tuning should have been sent immediately after receiving");
    tuner.on_tuning_updated(); // Confirm that the first tuning has been updated on the instrument.
    // There was no pending tuning waiting to be sent when update was confirmed
    // for the first tuning sent. So no more MIDI messages should have been sent.
    println!("First tuning update confirmed: cumulative control_change_count = {}",
             sent_midi().control_change_count);
    assert_that!(sent_midi().control_change_count, eq(single_tuning_control_change_count),
        "First tuning update confirmed.");
    // Second tuning received.
    // No tuning updates are pending, so the tuning should be sent immediately.
    tuner.on_tuning_received(params_16_16());
    // Check that the tuning has been sent.
    println!("Second tuning should have been sent: cumulative control_change_count = {}",
             sent_midi().control_change_count);
    assert_that!(sent_midi().control_change_count, eq(single_tuning_control_change_count * 2),
        "Second tuning should have been sent immediately after receiving");
    // Send the third tuning before update confirmation has been received for the second tuning.
    // Because update of a previously sent tuning is pending, the third tuning should not be sent
    // yet.
    tuner.on_tuning_received(params_17_17());
    // Check that the tuning has not yet been sent.
    println!("Third tuning has been received but should not have been sent yet: \
        cumulative control_change_count = {}", sent_midi().control_change_count);
    assert_that!(sent_midi().control_change_count, eq(single_tuning_control_change_count * 2),
        "Third tuning has been received but should not have been sent yet");
    tuner.on_tuning_updated(); // Confirm that the second tuning was updated on the instrument.
    // The third tuning was waiting to be sent when update was confirmed
    // for the second tuning sent. So the third tuning should have been sent now.
    println!("Second tuning update confirmed, so the third tuning should have been sent now: \
        cumulative control_change_count = {}", sent_midi().control_change_count);
    assert_that!(sent_midi().control_change_count, eq(single_tuning_control_change_count * 3),
        "Second tuning update confirmed, so the third tuning should have been sent now.");
}

#[googletest::gtest]
fn remove_data() {
    println!("***********************************");
    println!("remove_data test started");
    println!("***********************************");
    let _guard = test_mutex_guard();
    let tuner = create_tuner();
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
    let _guard = test_mutex_guard();
    let tuner = create_tuner();
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
    let _guard = test_mutex_guard();
    let new_pitch_table: u8 = Tuner::default_pitch_table();
    let tuner = create_tuner();
    assert_that!(Tuner::pitch_table(), eq(PITCH_TABLE));
    assert_that!(tuner.pitch_table_index(), eq(1));
    tuner.set_pitch_table(new_pitch_table);
    assert_that!(Tuner::pitch_table(), eq(new_pitch_table));
    assert_that!(tuner.pitch_table_index(), eq(0));
}

#[googletest::gtest]
fn set_root_freq_override_note_no() {
    let _guard = test_mutex_guard();
    let tuner = create_tuner();
    tuner.on_tuning_received(params_31_19());
    tuner.on_tuning_updated(); // Allow a subsequent tuning to be sent.
    tuner.set_root_freq_override_note_no(4 /* A */, true);
    assert_that!(tuner.is_root_freq_overridden(), eq(true));
    let mut formatted = tuner.formatted_tuning_params();
    assert_that!(formatted.root_freq, eq("220 Hz"), "Overridden");
    tuner.set_root_freq_override_note_no(0 /* No override */, true);
    formatted = tuner.formatted_tuning_params();
    assert_that!(formatted.root_freq, eq("261.626 Hz"), "No override");
}

fn create_tuner() -> Tuner {
    let tuner = Tuner::new();
    tuner.init(PITCH_TABLE);
    tuner.set_midi_sender(Box::new(MockMidiSender::new()));
    tuner
}

/// Gets parameters for PitchGrid's "Mavila (16-TET, 16 keys/8ve)" tuning.
fn params_16_16() -> TuningParams {
    TuningParams::new(1, 261.62558, 0.99999994, 0.56250846,
                      6.8750014, 16, 5, 2)
}

/// Gets parameters for PitchGrid's "Dicoid 17-TET (7L 3s)" tuning.
fn params_17_17() -> TuningParams {
    TuningParams::new(1, 256.86972, 0.99999994, 0.7058833,
                      7.333337, 17, 7, 3)
}

/// Gets parameters for PitchGrid's "31-TET (19 keys per 8ve)" tuning.
fn params_31_19() -> TuningParams {
    TuningParams::new(1, 261.62558, 0.99999994, 0.5806459,
                      8.250002, 19, 5, 2)
}

fn test_mutex_guard() -> MutexGuard<'static, ()> {
    TEST_MUTEX.lock().unwrap_or_else(|e| e.into_inner())
}

const MAX_ROUNDING_RATE: u8 = 127;
const PITCH_TABLE: u8 = 81;
