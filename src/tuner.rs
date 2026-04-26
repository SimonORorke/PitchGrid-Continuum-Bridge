mod tuner_refs;
use std::cmp::{max};
use std::sync::atomic::{AtomicBool, AtomicU8, AtomicUsize, Ordering};
use cxx::UniquePtr;
use round::round;
use tuner_refs::{default_pitch_keys, keys_clone, params_clone, root_freq_override, set_keys};
use crate::{midi_static};
use crate::midi::Midi;
use crate::tuner::ffi::MOS;
use crate::tuning_params::{TuningParams,};

pub fn init(pitch_table: u8) {
    PITCH_TABLE.store(pitch_table, Ordering::Relaxed);
    tuner_refs::pitch_tables();
    midi_static::midi_clone().lock().unwrap()
        .add_tuning_updated_callback(Box::from(on_tuning_updated));
}

/// Update tuning parameters from the OSC message.
///     Args:
///         mode: Mode index
///         root_freq: Root pitch in Hz
///         stretch: Equave as log2 frequency ratio (e.g. 1.0 for octave)
///         skew: Generator as log2 frequency ratio
///         mode_offset: Mode offset (float)
///         steps: Number of steps per period
///         mos_a: MOS parameter a
///         mos_b: MOS parameter b
///             MOS (a,b) is not the same as (L,s). Sometimes (a,b) = (L,s) and sometimes
///             (a,b) = (s,L) depending on the relative size of the vectors.
///             The scalatrix MOS class also has mos.nL and mos.nS properties
///             which will be used when displaying the MOS system (like 5L 2s).
pub fn on_tuning_received(tuning_params: TuningParams) {
    // println!(
    //     "tuner.on_tuning_received: depth = {}; mode = {}; root_freq = {}; stretch = {}; \
    //     skew = {}; mode_offset = {}; steps = {}",
    //     depth, mode, root_freq, stretch, skew, mode_offset, steps);
    *params_clone().lock().unwrap() = tuning_params;
    tune();
}

/// Calculates the tuning and either sends it to the instrument, provided another tuning update is
/// not already in progress, or stores it for sending once the current update completes.
/// This is decoupled from receiving the tuning parameters from PitchGrid, as additional parameters
/// specified in the UI may need to be updated
fn tune() {
    let send_now:bool;
    {
        let params_clone = params_clone();
        let params = params_clone.lock().unwrap();
        let key_pitches = calculate_key_pitches(TuningParams::new(
            params.mode(), params.root_freq(), params.stretch(),
            params.skew(), params.mode_offset(), max(1, params.steps()),
            params.mos_a(), params.mos_b()));
        set_keys(key_pitches.iter().enumerate()
            .map(|(i, pitch)| {
                Key {
                    number: i as u8,
                    required_pitch: *pitch,
                    to_number: 0,
                    offset_ratio: 0.0,
                    offset_msb: 0,
                    offset_lsb: 0,
                }
            }).collect());
        // If the player sweeps one of the tuning controls in PitchGrid,
        // we will receive new tunings much faster than the instrument can update the tuning table,
        // which takes in the order of half a second.
        // If we were to keep sending tunings to the instrument regardless, its processor would
        // be swamped, probably for minutes.
        // The solution is to not send more updates to the instrument while another update is in
        // progress and, once the update is complete, send the most recently received following
        // tuning if there is one.
        // That will work because we will have just overwritten any previous pending tuning with a
        // new one.
        let is_already_updating = IS_ALREADY_UPDATING.load(Ordering::Relaxed);
        // println!("tuner.on_tuning_received: is_already_updating = {is_already_updating}");
        if is_already_updating {
            IS_ANOTHER_UPDATE_PENDING.store(true, Ordering::Relaxed);
            send_now = false;
        } else {
            IS_ALREADY_UPDATING.store(true, Ordering::Relaxed);
            send_now = true;
        }
    }
    if send_now {
        send_tuning();
    }
}

/// Calculates and returns the pitch required for each key in the MIDI range,
/// given the tuning parameters.
fn calculate_key_pitches(tuning_params: TuningParams) -> Vec<f32> {
    // println!("tuner.calculate_key_pitches");
    let root_freq = {
        let override_freq = root_freq_override();
        let mut override_freq_guard = override_freq.lock().unwrap();
        if ROOT_FREQ_OVERRIDE_NOTE_NO.load(Ordering::Relaxed) == 0 {
            // Override not required
            // println!("tuner.calculate_key_pitches: Override not required");
            *override_freq_guard = 0.0;
            tuning_params.root_freq()
        } else {
            let note_no = ROOT_FREQ_OVERRIDE_NOTE_NO.load(Ordering::Relaxed);
            let pitch = default_pitch_keys()[note_no];
            // println!("tuner.calculate_key_pitches: Overriding root freq with note {}, pitch {} Hz",
            //          note_no, pitch);
            *override_freq_guard = pitch;
            pitch
        }
    };
    let mos = mos_from_tuning_params(&tuning_params);
    let a1 = ffi::vector2d(0.0, 0.0);
    let a2 = ffi::vector2d(
        ffi::get_mos_v_gen_x(&mos) as f64, ffi::get_mos_v_gen_y(&mos) as f64);
    let a3 = ffi::vector2d(
        ffi::get_mos_a(&mos) as f64, ffi::get_mos_b(&mos) as f64);
    let b1 = ffi::vector2d(
        0.0, (tuning_params.mode_offset() as f64 + 0.5) / tuning_params.steps() as f64);
    let b2 = ffi::vector2d(
        (tuning_params.skew() * tuning_params.stretch()) as f64, (tuning_params.mode_offset() as f64 + 1.5) / tuning_params.steps() as f64);
    let b3 = ffi::vector2d(
        tuning_params.stretch() as f64, (tuning_params.mode_offset() as f64 + 0.5) / tuning_params.steps() as f64);
    let affine = ffi::affine_from_three_dots(
        &a1, &a2, &a3,
        &b1, &b2, &b3);
    let scale = ffi::scale_from_affine(
        &affine, root_freq as f64,
        128, // MIDI note number range 0 to 127
        60); // Middle C
    let scale_nodes = ffi::get_scale_nodes(&scale);
    scale_nodes.iter().map(|node|
        // root_freq is received as f32 rounded to 5 decimal places,
        // so let's store the pitch with the same precision.
        round(ffi::get_node_pitch(&node), 5) as f32)
        .collect()
}

/// If tuning data has previously been received, resends it to the instrument.
/// Returns whether tuning data was resent.
pub fn resend_tuning() -> bool {
    let can_send = keys_clone().len() > 0usize;
    if can_send {
        // Tuning data has previously been received.
        // println!("tuner.resend_tuning: Resending tuning data to instrument.");
        send_tuning();
    }
    can_send
}

fn send_tuning() {
    let mut keys = keys_clone();
    set_to_key_numbers(&mut keys);
    calculate_offsets(&mut keys);
    Midi::on_updating_tuning();
    send_rounding_params();
    send_pitch_table(pitch_table(), &keys);
}

/// Sets the to_number field of each Key in TUNER_DATA.keys to
/// the index of the pitch in DEFAULT_KEY_PITCHES
/// that matches the Key's pitch.
/// A match is when a pitch in TUNER_DATA.note_pitches is
/// greater than or equal to the pitch in DEFAULT_KEY_PITCHES
/// and less than the next pitch, if any, in DEFAULT_KEY_PITCHES.
/// Pitches in both vectors are assumed to be sorted in ascending order.
fn set_to_key_numbers(keys: &mut Vec<Key>) {
    // println!("tuner.set_to_key_numbers");
    // Exact match: Ok(i) → returns i ✓
    // pitch < first: Err(0) → returns 0 ✓
    // pitch between elements: Err(i) where i > 0 → returns i - 1 ✓
    // pitch > last: Err(128) → returns 127 ✓
    for key_being_matched in keys.iter_mut() {
        key_being_matched.to_number = default_pitch_keys().binary_search_by(|&x|
            x.partial_cmp(&key_being_matched.required_pitch).unwrap()).unwrap_or_else(|i| {
            if i == 0 {
                0  // Below first pitch
            } else {
                i - 1  // pitch is between DEFAULT_KEY_PITCHES[i-1] and [i]
            }
        }) as u8;
    }
}

/// Calculates the offset ratio and 14-bit offset values for each note.
/// The offset ratio is the offset specified as % of a semitone between
/// the required pitch and the closest standard tuning pitch that is less than or equal to it.
fn calculate_offsets(keys: &mut Vec<Key>) {
    // println!("tuner.calculate_offsets");
    for i in 0..keys.len() {
        let note_pitch = keys[i].required_pitch;
        // Get the closest standard tuning pitch that is less than or equal to the
        // required note pitch.
        let to_note_pitch = default_pitch_keys()[keys[i].to_number as usize];
        // offset_ratio is the offset specified as % of a semitone between
        // note_pitch and to_note_pitch.
        let mut offset_ratio = 12.0 * (note_pitch / to_note_pitch).log2();
        if offset_ratio > 1.0 { // Could happen if to_number is 127.
            offset_ratio = 1.0;
        } else if offset_ratio < 0.0 { // Shouldn't happen
            offset_ratio = 0.0; // Could happen with low notes.
        }
        keys[i].offset_ratio = offset_ratio;
        // Convert offset_ratio (0.0 to 1.0) to 14-bit value (0 to 16,383)
        let offset_14bit = (offset_ratio * 16383.0).round() as u16;
        // Extract the upper 7 bits for MSB by shifting right 7 positions.
        // Extract the lower 7 bits for LSB using a mask.
        // Mask both with 0x7F to ensure they're 7-bit values.
        keys[i].offset_msb = ((offset_14bit >> 7) & 0x7F) as u8;
        keys[i].offset_lsb = (offset_14bit & 0x7F) as u8;
        // println!(
        //     "note {}: pitch = {}; to_note = {}; to_note_pitch = {}; offset_ratio = {}; \
        //     offset_msb = {}, offset_lsb = {}",
        //     notes[i].number, note_pitch, notes[i].to_number, to_note_pitch,
        //     notes[i].offset_ratio, notes[i].offset_msb, notes[i].offset_lsb);
    }
}

fn send_pitch_table(pitch_table: u8, keys: &Vec<Key>) {
    // println!("tuner.send_pitch_table_to_instrument");
    // Select pitch table to update.
    Midi::send_control_change(16, 109, pitch_table);
    // Tuning for each MIDI key
    for key in keys {
        // Base/From MIDI key
        Midi::send_control_change(16, 38, key.number);
        // Base/From MIDI key tuning MSB
        Midi::send_control_change(16, 38, 0);
        // Base/From MIDI key tuning LSB
        Midi::send_control_change(16, 38, 0);
        // Re-tuned/To MIDI key
        Midi::send_control_change(16, 38, key.to_number);
        // Re-tuned/To MIDI key tuning MSB
        Midi::send_control_change(16, 38, key.offset_msb);
        // Re-tuned/To MIDI key tuning LSB
        Midi::send_control_change(16, 38, key.offset_lsb);
    }
    // Save pitch table on instrument.
    Midi::send_control_change(16, 109, 101);
    // Set active pitch table for performance.
    Midi::send_control_change(16, 51, pitch_table); // Grid
}

/// The tuning parameters formatted for display.
pub fn formatted_tuning_params() -> FormattedTuningParams {
    // println!("tuner.formatted_tuning_params");
    let params_clone = params_clone();
    let params = params_clone.lock().unwrap();
    // Show the root frequency override if there is one.
    let root_freq = {
      if ROOT_FREQ_OVERRIDE_NOTE_NO.load(Ordering::Relaxed) == 0 {
          // No override
          params.root_freq()
      } else {
          root_freq_override().lock().unwrap().clone()
      }
    };
    let mos = mos_from_tuning_params(&params);
    FormattedTuningParams {
        root_freq: format!("{} Hz", round(root_freq as f64, 3)),
        // The stretch parameter is in octaves, so we need to multiply by 1200 to get the
        // number of cents to display.
        stretch: format!("{} ct", (params.stretch() * 1200.0).round()),
        skew: format!("{}", round(params.skew() as f64, 5)),
        mode_offset: format!("{}", round(params.mode_offset() as f64, 5)),
        steps: format!("{}", params.steps()),
        mos_large_step_count: format!("{}", ffi::get_mos_large_step_count(&mos)),
        mos_small_step_count: format!("{}", ffi::get_mos_small_step_count(&mos)),
    }
}

pub fn is_root_freq_overridden() -> bool {
    ROOT_FREQ_OVERRIDE_NOTE_NO.load(Ordering::Relaxed) != 0
}

/// Sets the root frequency override and optionally sends it to the instrument.
pub fn set_root_freq_override_note_no(index: usize, send_tuning: bool) {
    let note_no = {
        if index == 0 {
            0usize // No override
        } else {
            index + 53 // E.g. for middle C, index = 7 note_no = 60.
        }
    };
    // println!("tuner.set_root_freq_override_not_no: index = {}, note_no = {}", index, note_no);
    ROOT_FREQ_OVERRIDE_NOTE_NO.store(note_no, Ordering::Relaxed);
    if send_tuning {
        tune();
    }
}

/// Sets whether initial rounding is to be to be overridden to on the next time tuning is sent.
pub fn set_override_rounding_initial(value: bool) {
    OVERRIDE_ROUNDING_INITIAL.store(value, Ordering::Relaxed);
}

/// Sets whether rounding rate is to be overridden the next time tuning is sent.
pub fn set_override_rounding_rate(value: bool) {
    OVERRIDE_ROUNDING_RATE.store(value, Ordering::Relaxed);
}

/// Sets the rounding rate, if it is to be to be overridden, the next time tuning is sent.
pub fn set_rounding_rate(rate: u8) {
    ROUNDING_RATE.store(rate, Ordering::Relaxed);
}

pub fn set_pitch_table(pitch_table: u8) {
    PITCH_TABLE.store(pitch_table, Ordering::Relaxed);
}

pub fn default_pitch_table() -> u8 { 80 }

fn on_tuning_updated() {
    // println!("tuner.on_tuning_updated");
    // See comment in on_tuning_received.
    let send_again:bool;
    {
        let is_another_update_pending = IS_ANOTHER_UPDATE_PENDING.load(Ordering::Relaxed);
        // println!("tuner.on_tuning_updated: is_another_update_pending = {is_another_update_pending}");
        if is_another_update_pending {
            IS_ANOTHER_UPDATE_PENDING.store(false, Ordering::Relaxed);
            send_again = true;
        } else {
            IS_ALREADY_UPDATING.store(false, Ordering::Relaxed);
            send_again = false;
        }
    }
    if send_again {
        send_tuning();
    }
}

/// Sends pitch rounding parameters, if required, to the instrument.
/// If the Rounding Initial override is On, rounds each note's initial pitch to the key's specified
/// tuning pitch; otherwise the current preset's Initial Rounding parameter is unchanged.
/// If the Rounding Rate override is On, sends Rounding Mode Normal with the specified Rounding Rate
/// value; otherwise the current preset's Rounding Mode and Rounding Rate parameters are unchanged.
/// Rounding Rate option On with Rounding Rate 127 (the maximum) effectively enforces initial
/// rounding, even when the current preset's Initial Rounding is off. In addition, it prevents
/// the pitch from being changed by subsequent motion of the finger on the fingerboard.
/// Note: A bug in the EaganMatrix firmware was fixed in firmware version 10.73 where previously,
/// when the current preset's Initial Rounding was on, the instrument's octave shifting, with
/// buttons or pedals, would not work.
fn send_rounding_params() {
    if OVERRIDE_ROUNDING_INITIAL.load(Ordering::Relaxed) {
        // Turn on Rounding Initial
        Midi::send_control_change(1, 28, 127); // RndIni
    }
    if OVERRIDE_ROUNDING_RATE.load(Ordering::Relaxed) {
        // Rounding Mode Normal
        Midi::send_matrix_poke(10, 0); // RoundMode
        // Max Rounding Rate
        Midi::send_control_change(1, 25, ROUNDING_RATE.load(Ordering::Relaxed)); // RoundRate
    }
}

pub fn pitch_table_index() -> usize {
    // Return the index of the PITCH_TABLES item that equals pitch_table.
    pitch_tables().iter().position(|&x| x == pitch_table()).unwrap_or(0)
}

pub fn pitch_table() -> u8 {
    PITCH_TABLE.load(Ordering::Relaxed)
}

pub fn pitch_tables<'a>() -> &'a Vec<u8> {
    tuner_refs::pitch_tables()
}

fn mos_from_tuning_params(tuning_params: &TuningParams) -> UniquePtr<MOS> {
    ffi::mos_from_params(
        tuning_params.mos_a(),
        tuning_params.mos_b(),
        tuning_params.mode(),
        tuning_params.stretch() as f64,
        tuning_params.skew() as f64)
}

/// Interface to Peter Jung's scalatrix https://github.com/pitchgrid-io/scalatrix
/// C++ code, a version of which is embedded in this application,
/// used in the key pitch frequency calculation.
/// #[cxx::bridge] is a proc macro that requires the module body to be inline in the same file.
/// So the ffi module cannot be moved to a separate file.
#[cxx::bridge(namespace = "scalatrix")]
mod ffi {
    unsafe extern "C++" {
        include!("scalatrix.hpp");

        type AffineTransform;
        type MOS;
        type Node;
        type Scale;
        type Vector2d;

        // If you add, remove, or modify any of the functions defined below,
        // you must update the corresponding C++ functions defined in scalatrix/scalatrix.hpp.
        fn  affine_from_three_dots(
            a1: &Vector2d, a2: &Vector2d, a3: &Vector2d,
            b1: &Vector2d, b2: &Vector2d, b3: &Vector2d) -> UniquePtr<AffineTransform>;

        fn mos_from_params(a: i32, b: i32, m: i32, e: f64, g: f64) -> UniquePtr<MOS>;
        fn get_mos_a(mos: &MOS) -> i32;
        fn get_mos_b(mos: &MOS) -> i32;
        fn get_mos_large_step_count(mos: &MOS) -> i32;
        fn get_mos_small_step_count(mos: &MOS) -> i32;
        fn get_mos_v_gen_x(mos: &MOS) -> i32;
        fn get_mos_v_gen_y(mos: &MOS) -> i32;
        fn get_node_pitch(node: &Node) -> f64;
        fn get_scale_nodes(scale: &Scale) -> UniquePtr<CxxVector<Node>>;

        fn scale_from_affine(
            affine: &AffineTransform, base_freq: f64,
            num_nodes_to_generate: i32, root_index: i32) -> UniquePtr<Scale>;

        fn vector2d(x: f64, y: f64) -> UniquePtr<Vector2d>;
    }
}

#[derive(Clone)]
struct Key {
    /// The MIDI note number of the key (0-127).
    number: u8,
    /// The pitch required for the note, in Hz.
    required_pitch: f32,
    /// The MIDI note number of the key with the closest standard tuning pitch
    /// that is less than or equal to the required pitch.
    to_number: u8,
    /// The offset ratio (0.0 to 1.0) as a fraction of a semitone, between the required pitch
    /// and the closest standard tuning pitch that is less than or equal to it.
    offset_ratio: f32,
    /// The upper 7 bits (MSB) of the offset ratio, as a 14-bit value,
    /// for sending to the instrument via MIDI.
    offset_msb: u8,
    /// The lower 7 bits (LSB) of the offset ratio, as a 14-bit value,
    /// for sending to the instrument via MIDI.
    offset_lsb: u8,
}


/// The tuning parameters formatted for display.
pub struct FormattedTuningParams {
    pub root_freq: String, pub stretch: String,
    pub skew: String, pub mode_offset: String, pub steps: String,
    pub mos_large_step_count: String, pub mos_small_step_count: String,
}

static IS_ALREADY_UPDATING: AtomicBool = AtomicBool::new(false);
static IS_ANOTHER_UPDATE_PENDING: AtomicBool = AtomicBool::new(false);
static OVERRIDE_ROUNDING_INITIAL: AtomicBool = AtomicBool::new(true);
static OVERRIDE_ROUNDING_RATE: AtomicBool = AtomicBool::new(true);

/// About the name Pitch Table.
/// The EaganMatrix Overlay Developer's Guide calls them tuning grids.
/// But naming is inconsistent in the Continuum User Guide.
/// The main documentation calls them pitch tables or custom grids, though there are also
/// scattered references to tuning grids.
/// We call them pitch tables, as that has the best chance of being understood by users.
static PITCH_TABLE: AtomicU8 = AtomicU8::new(0);

static ROOT_FREQ_OVERRIDE_NOTE_NO: AtomicUsize = AtomicUsize::new(0);
static ROUNDING_RATE: AtomicU8 = AtomicU8::new(127);
