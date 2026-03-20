use std::cmp::max;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, AtomicU8, AtomicUsize, Ordering};
use lazy_static::lazy_static;
use round::round;
use crate::global::{SharedMidi};
use crate::midi::Midi;

/// Update tuning parameters from the OSC message.
///     Args:
///         depth: MOS depth (generation)
///         mode: Mode index
///         root_freq: Root pitch in Hz
///         stretch: Stretch factor
///         skew: Skew factor
///         mode_offset: Mode offset
///         steps: Number of steps per period
pub fn on_tuning_received(depth: i32, mode: i32, root_freq: f32, stretch: f32,
                          skew: f32, mode_offset: i32, steps: i32) {
    // println!(
    //     "tuner.on_tuning_received: depth = {}; mode = {}; root_freq = {}; stretch = {}; \
    //     skew = {}; mode_offset = {}; steps = {}",
    //     depth, mode, root_freq, stretch, skew, mode_offset, steps);
    {
        let mut data = TUNER_DATA.lock().unwrap();
        data.tuning_params.depth = depth;
        data.tuning_params.mode = mode;
        data.tuning_params.root_freq = root_freq;
        data.tuning_params.stretch = stretch;
        data.tuning_params.skew = skew;
        data.tuning_params.mode_offset = mode_offset;
        data.tuning_params.steps = steps;
    }
    tune();
}

/// Calculates the tuning and either sends it to the instrument, provided another tuning update is
/// not already in progress, or stores it for sending once the current update completes.
/// This is decoupled from receiving the tuning parameters from PitchGrid, as additional parameters
/// specified in the UI may need to be updated
fn tune() {
    let send_now:bool;
    {
        let mut data = TUNER_DATA.lock().unwrap();
        let params = &data.tuning_params;
        let key_pitches = calculate_key_pitches(
            max(1, params.depth), params.mode, params.root_freq, params.stretch, params.skew,
            params.mode_offset, max(1, params.steps));
        data.keys = Arc::new(key_pitches.iter().enumerate()
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
        let is_already_updating = data.is_already_updating.load(Ordering::Relaxed);
        // println!("tuner.on_tuning_received: is_already_updating = {is_already_updating}");
        if is_already_updating {
            data.is_another_update_pending.store(true, Ordering::Relaxed);
            send_now = false;
        } else {
            data.is_already_updating.store(true, Ordering::Relaxed);
            send_now = true;
        }
    }
    if send_now {
        send_tuning();
    }
}

/// Calculates and returns the pitch required for each key in the MIDI range,
/// given the tuning parameters.
fn calculate_key_pitches(depth: i32, mode: i32, root_freq: f32, stretch: f32,
                         skew: f32, mode_offset: i32, steps: i32) -> Vec<f32> {
    // println!("tuner.calculate_key_pitches");
    let root_freq = {
        let mut override_freq = ROOT_FREQ_OVERRIDE.lock().unwrap();
        if ROOT_FREQ_OVERRIDE_NOTE_NO.load(Ordering::Relaxed) == 0 {
            // Override not required
            // println!("tuner.calculate_key_pitches: Override not required");
            *override_freq = 0.0;
            root_freq
        } else {
            let note_no = ROOT_FREQ_OVERRIDE_NOTE_NO.load(Ordering::Relaxed);
            let pitch = DEFAULT_KEY_PITCHES[note_no];
            // println!("tuner.calculate_key_pitches: Overriding root freq with note {}, pitch {} Hz",
            //          note_no, pitch);
            *override_freq = pitch;
            pitch
        }
    };
    let mos = ffi:: mos_from_g(
        depth,
        mode,
        skew as f64,
        stretch as f64,
        1);
    let a1 = ffi::vector2d(0.0, 0.0);
    let a2 = ffi::vector2d(
        ffi::get_mos_v_gen_x(&mos) as f64, ffi::get_mos_v_gen_y(&mos) as f64);
    let a3 = ffi::vector2d(
        ffi::get_mos_a(&mos) as f64, ffi::get_mos_b(&mos) as f64);
    let b1 = ffi::vector2d(
        0.0, (mode_offset as f64 + 0.5) / steps as f64);
    let b2 = ffi::vector2d(
        (skew * stretch) as f64, (mode_offset as f64 + 1.5) / steps as f64);
    let b3 = ffi::vector2d(
        stretch as f64, (mode_offset as f64 + 0.5) / steps as f64);
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
    let can_send = TUNER_DATA.lock().unwrap().keys.len() > 0usize;
    if can_send {
        // Tuning data has previously been received.
        // println!("tuner.resend_tuning: Resending tuning data to instrument.");
        send_tuning();
    }
    can_send
}

fn send_tuning() {
    // println!("tuner.update_tuning");
    let data = TUNER_DATA.lock().unwrap();
    let mut keys = (*data.keys).clone();
    let pitch_table_no = data.pitch_table_no.load(Ordering::Relaxed);
    set_to_key_numbers(&mut keys);
    calculate_offsets(&mut keys);
    Midi::on_updating_tuning();
    send_rounding_params(true);
    send_pitch_table(&keys, pitch_table_no);
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
        key_being_matched.to_number = DEFAULT_KEY_PITCHES.binary_search_by(|&x|
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
        let to_note_pitch = DEFAULT_KEY_PITCHES[keys[i].to_number as usize];
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

fn send_pitch_table(keys: &Vec<Key>, pitch_table_no: u8) {
    // println!("tuner.send_pitch_table_to_instrument");
    // Select pitch table to update.
    Midi::send_control_change(16, 109, pitch_table_no);
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
    Midi::send_control_change(16, 51, pitch_table_no); // Grid
}

pub fn formatted_tuning_params() -> FormattedTuningParams {
    // println!("tuner.formatted_tuning_params");
    let data = TUNER_DATA.lock().unwrap();
    // Show the root frequency override if there is one.
    let root_freq = {
      if ROOT_FREQ_OVERRIDE_NOTE_NO.load(Ordering::Relaxed) == 0 {
          // No override
          data.tuning_params.root_freq
      } else {
          ROOT_FREQ_OVERRIDE.lock().unwrap().clone()
      }
    };
    FormattedTuningParams {
        depth: format!("{}", data.tuning_params.depth),
        root_freq: format!("{} Hz", round(root_freq as f64, 3)),
        // The stretch parameter is in octaves, so we need to multiply by 1200 to get the
        // number of cents to display.
        stretch: format!("{} ct", (data.tuning_params.stretch * 1200.0).round()),
        skew: format!("{}", round(data.tuning_params.skew as f64, 5)),
        mode_offset: format!("{}", data.tuning_params.mode_offset),
        steps: format!("{}", data.tuning_params.steps),
    }
}

pub fn set_midi(midi: SharedMidi) {
    midi.lock().unwrap().add_tuning_updated_callback(Box::from(on_tuning_updated));
    TUNER_DATA.lock().unwrap().midi = Some(midi);
}

/// Sets the root frequency override and sends it to the instrument.
pub fn set_root_freq_override(index: usize, send_tuning: bool) {
    let note_no = {
        if index == 0 {
            0usize // No override
        } else {
            index + 53 // E.g. for middle C, index = 7 note_no = 60.
        }
    };
    // println!("tuner.set_root_freq_override: index = {}, note_no = {}", index, note_no);
    ROOT_FREQ_OVERRIDE_NOTE_NO.store(note_no, Ordering::Relaxed);
    if send_tuning {
        tune();
    }
}

pub fn set_pitch_table_no(pitch_table_no: u8) {
    TUNER_DATA.lock().unwrap().pitch_table_no.store(pitch_table_no, Ordering::Relaxed);
}

pub fn default_pitch_table_no() -> u8 { 80 }

fn on_tuning_updated() {
    // println!("tuner.on_tuning_updated");
    // See comment in on_tuning_received.
    let send_again:bool;
    {
        let data = TUNER_DATA.lock().unwrap();
        let is_another_update_pending = data.is_another_update_pending.load(Ordering::Relaxed);
        // println!("tuner.on_tuning_updated: is_another_update_pending = {is_another_update_pending}");
        if is_another_update_pending {
            data.is_another_update_pending.store(false, Ordering::Relaxed);
            send_again = true;
        } else {
            data.is_already_updating.store(false, Ordering::Relaxed);
            send_again = false;
        }
    }
    if send_again {
        send_tuning();
    }
}

fn send_rounding_params(on: bool) {
    if on {
        // Rounding Mode Normal
        Midi::send_matrix_poke(10, 0); // RoundMode
    }
    // Initial Rounding
    let initial_rounding_value:u8 = if on { 127 } else { 0 };
    Midi::send_control_change(1, 28, initial_rounding_value); // RndIni
    // Rounding Rate
    let rounding_rate_value:u8 =
        if on { 127 /* Immediate when initial rounding is on */ } else { 0 /* Off */ };
    Midi::send_control_change(1, 25, rounding_rate_value); // RoundRate
}

pub fn override_names() -> Vec<String> {
    vec!["".to_string(),
         "F#".to_string(), "G".to_string(), "G#".to_string(),
         "A".to_string(), "A#".to_string(), "B".to_string(),
         "C".to_string(),
         "C#".to_string(),"D".to_string(), "D#".to_string(),
         "E".to_string(), "F".to_string(), ]
}

pub fn pitch_table_index() -> usize {
    let pitch_table_no = TUNER_DATA.lock().unwrap().pitch_table_no.load(Ordering::Relaxed);
    // Return the index of the PITCH_TABLE_NOS item that equals pitch_table_no.
    PITCH_TABLE_NOS.iter().position(|&x| x == pitch_table_no).unwrap_or(0)
}

pub fn pitch_table_no() -> u8 {
    TUNER_DATA.lock().unwrap().pitch_table_no.load(Ordering::Relaxed)
}

pub fn pitch_table_nos() -> Vec<u8> {
    PITCH_TABLE_NOS.clone()
}

/// Returns the default key pitches in Hz, where the default scale is 12-TET
/// at standard concert pitch A=440.
#[allow(unused)] // The compiler thinks this function is unused,
// even though it's used to initialise DEFAULT_KEY_PITCHES.
fn create_default_key_pitches() -> Vec<f32> {
    vec![
        8.1758, 8.66196, 9.17703, 9.72272, 10.30086, 10.91339, 11.56233, 12.24986, 12.97828,
        13.75, 14.56762, 15.43385, 16.3516, 17.32392, 18.35405, 19.44544, 20.60172, 21.82677,
        23.12465, 24.49972, 25.95655, 27.5, 29.13524, 30.86771, 32.7032, 34.64784, 36.7081,
        38.89088, 41.20345, 43.65354, 46.2493, 48.99944, 51.9131, 55.0, 58.27048, 61.73541,
        65.4064, 69.29568, 73.4162, 77.78176, 82.40689, 87.30707, 92.4986, 97.99887, 103.8262,
        110.0, 116.54096, 123.47082, 130.8128, 138.59135, 146.8324, 155.56352, 164.81377,
        174.61414, 184.9972, 195.99773, 207.65239, 219.99998, 233.08191, 246.94164, 261.62558,
        277.18268, 293.66476, 311.12704, 329.62753, 349.22827, 369.9944, 391.99545, 415.30475,
        439.99997, 466.1638, 493.88324, 523.25116, 554.36536, 587.3295, 622.254, 659.255,
        698.4565, 739.9887, 783.99084, 830.6095, 879.9999, 932.3276, 987.7664, 1046.5022,
        1108.7307, 1174.6589, 1244.508, 1318.51, 1396.913, 1479.9773, 1567.9816, 1661.2189,
        1759.9998, 1864.655, 1975.5327, 2093.0044, 2217.4612, 2349.3179, 2489.0159, 2637.02,
        2793.8257, 2959.9546, 3135.9631, 3322.4377, 3519.9993, 3729.31, 3951.0654, 4186.009,
        4434.9224, 4698.6353, 4978.0317, 5274.0396, 5587.6514, 5919.9087, 6271.926, 6644.875,
        7039.9985, 7458.6196, 7902.1304, 8372.017, 8869.844, 9397.2705, 9956.0625, 10548.079,
        11175.302, 11839.817, 12543.852]
}

/// Interface to C++ code used in the key pitch frequency calculation.
#[cxx::bridge(namespace = "scalatrix")]
mod ffi {
    unsafe extern "C++" {
        include!("scalatrix.hpp");

        type AffineTransform;
        type MOS;
        type Node;
        type Scale;
        type Vector2d;

        fn  affine_from_three_dots(
            a1: &Vector2d, a2: &Vector2d, a3: &Vector2d,
            b1: &Vector2d, b2: &Vector2d, b3: &Vector2d) -> UniquePtr<AffineTransform>;

        fn mos_from_g(depth: i32, m: i32, g: f64, e: f64, repetitions: i32) -> UniquePtr<MOS>;
        fn get_mos_a(mos: &MOS) -> i32;
        fn get_mos_b(mos: &MOS) -> i32;
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

struct TunerData {
    tuning_params: TuningParams,
    midi: Option<SharedMidi>,
    keys:Arc<Vec<Key>>,
    is_already_updating: Arc<AtomicBool>,
    is_another_update_pending: Arc<AtomicBool>,
    /// About the name Pitch Table.
    /// The EaganMatrix Overlay Developer's Guide calls them tuning grids.
    /// But naming is inconsistent in the Continuum User Guide.
    /// The main documentation calls them pitch tables or custom grids, though there are also
    /// scattered references to tuning grids.
    /// We call them pitch tables, as that has the best chance of being understood by users.
    pitch_table_no: Arc<AtomicU8>,
}

struct TuningParams {
    depth: i32, mode: i32, root_freq: f32, stretch: f32,
    skew: f32, mode_offset: i32, steps: i32,
}

pub struct FormattedTuningParams {
    pub depth: String, pub root_freq: String, pub stretch: String,
    pub skew: String, pub mode_offset: String, pub steps: String,
}

lazy_static! {
    static ref TUNER_DATA: Mutex<TunerData> = Mutex::new(TunerData {
        tuning_params: TuningParams {
            depth: 0, mode: 0, root_freq: 0.0, stretch: 0.0, skew: 0.0, mode_offset: 0, steps: 0,
        },
        keys: Arc::new(vec![]),
        is_already_updating: Arc::new(Default::default()),
        is_another_update_pending: Arc::new(Default::default()),
        midi: None,
        pitch_table_no: Arc::new(AtomicU8::new(default_pitch_table_no())),
    });
    static ref DEFAULT_KEY_PITCHES: Vec<f32> = create_default_key_pitches();
    static ref PITCH_TABLE_NOS: Vec<u8> = (80..88).collect();
    static ref ROOT_FREQ_OVERRIDE: Arc<Mutex<f32>> = Arc::new(Mutex::new(0.0));
    static ref ROOT_FREQ_OVERRIDE_NOTE_NO: AtomicUsize = AtomicUsize::new(0);
}
