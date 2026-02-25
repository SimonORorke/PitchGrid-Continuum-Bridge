use std::cmp::max;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicI32, Ordering};
use lazy_static::lazy_static;
use round::round;

#[derive(Clone)]
struct Note {
    number: usize,
    pitch: f32,
    to_number: usize,
    offset_ratio: f32,
    offset_msb: u8,
    offset_lsb: u8,
}


struct Data {
    pub notes:Arc<Vec<Note>>,
    pub tuning_grid_no: Arc<AtomicI32>,
}

lazy_static! {
    static ref DATA: Mutex<Data> = Mutex::new(Data {
        notes: Arc::new(vec![]),
        tuning_grid_no: Arc::new(AtomicI32::new(80)),
    });
    static ref TUNING_GRID_NOS: Vec<i32> = (80..88).collect();
    static ref DEFAULT_NOTE_PITCHES: Vec<f32> = create_default_note_pitches();
}

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
    let mut data = DATA.lock().unwrap();
    let note_pitches = calculate_note_pitches(
        max(1, depth), mode, root_freq, stretch, skew, mode_offset, max(1, steps));
    data.notes = Arc::new(note_pitches.iter().enumerate()
        .map(|(i, pitch)| {
        Note {
            number: i,
            pitch: *pitch,
            to_number: 0,
            offset_ratio: 0.0,
            offset_msb: 0,
            offset_lsb: 0,
        }
    }).collect());
    for note in data.notes.iter() {
        println!("note {}: {}", note.number, note.pitch);
        if note.number < data.notes.len() - 1 {
            println!(
                "Semitone: {} Hz",
                round((data.notes[note.number + 1].number as f64)
                          - (note.pitch as f64), 4).to_string());
        }
    }
}

pub fn update_tuning() {
    set_to_note_numbers();
    set_offsets();
}

/// Calculates and returns the pitch of each note in the MIDI range,
/// given the tuning parameters.
fn calculate_note_pitches(depth: i32, mode: i32, root_freq: f32, stretch: f32,
                          skew: f32, mode_offset: i32, steps: i32) -> Vec<f32> {
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

pub fn default_tuning_grid_no() -> i32 { 80 }

fn set_offsets() {
    let mut data = DATA.lock().unwrap();
    let notes = Arc::make_mut(&mut data.notes);
    for i in 0..notes.len() {
        let note_pitch = notes[i].pitch;
        let to_note_pitch = notes[notes[i].to_number].pitch;
        let mut offset_ratio = (note_pitch / to_note_pitch).log2();
        if offset_ratio > 1.0 { // Could happen if to_number is 127
            offset_ratio = 1.0;
        } else if offset_ratio < 0.0 { // Shouldn't happen
            panic!("offset_ratio should not be negative.")
        }
        notes[i].offset_ratio = offset_ratio;
        // Convert offset_ratio (0.0 to 1.0) to 14-bit value (0 to 16,383)
        let offset_14bit = (offset_ratio * 16383.0).round() as u16;
        // Extract the upper 7 bits for MSB by shifting right 7 positions.
        // Extract the lower 7 bits for LSB using a mask.
        // Mask both with 0x7F to ensure they're 7-bit values.
        notes[i].offset_msb = ((offset_14bit >> 7) & 0x7F) as u8;
        notes[i].offset_lsb = (offset_14bit & 0x7F) as u8;
    }
}

/// Sets the to_number field of each Note in DATA.notes to
/// the index of the note pitch in DEFAULT_NOTE_PITCHES
/// that matches the Note's pitch.
/// A match is when a pitch in DATA.note_pitches is
/// greater than or equal to the pitch in DEFAULT_NOTE_PITCHES
/// and less than the next pitch, if any, in DEFAULT_NOTE_PITCHES.
/// Pitches in both vectors are assumed to be sorted in ascending order.
fn set_to_note_numbers() {
    let mut data = DATA.lock().unwrap();
    let notes = Arc::make_mut(&mut data.notes);
    // Exact match: Ok(i) → returns i ✓
    // pitch < first: Err(0) → returns 0 ✓
    // pitch between elements: Err(i) where i > 0 → returns i - 1 ✓
    // pitch > last: Err(128) → returns 127 ✓
    for note in notes.iter_mut() {
        note.to_number = DEFAULT_NOTE_PITCHES.binary_search_by(|&x|
            x.partial_cmp(&note.pitch).unwrap()).unwrap_or_else(|i| {
            if i == 0 {
                0  // Below first pitch
            } else {
                i - 1  // pitch is between DEFAULT_NOTE_PITCHES[i-1] and [i]
            }
        });
    }
}

fn get_default_note_semitone_hz(note_number: usize) -> f32 {
    if note_number < DEFAULT_NOTE_PITCHES.len() - 1 {
        return DEFAULT_NOTE_PITCHES[note_number + 1] - DEFAULT_NOTE_PITCHES[note_number];
    }
    // Note 127, so there's no next pitch.  But we can hard-code it: 13289.75 - 12543.852.   
    745.898
}

pub fn set_tuning_grid_no(tuning_grid_no: i32) {
    DATA.lock().unwrap().tuning_grid_no.store(tuning_grid_no, Ordering::Relaxed);
}

pub fn tuning_grid_index() -> usize {
    let tuning_grid_no = DATA.lock().unwrap().tuning_grid_no.load(Ordering::Relaxed);
    // Return the index of the TUNING_GRID_NOS item that equals tuning_grid_no.
    TUNING_GRID_NOS.iter().position(|&x| x == tuning_grid_no).unwrap_or(0)
}

pub fn tuning_grid_nos() -> Vec<i32> {
    TUNING_GRID_NOS.clone()
}

/// Returns the default note pitches in Hz, where the default scale is 12-TET
/// at standard concert pitch.
#[allow(unused)] // The compiler thinks this function is unused,
// even though it's used to initialise DEFAULT_NOTE_PITCHES.
fn create_default_note_pitches() -> Vec<f32> {
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
    // 11175.302, 11839.817, 12543.852, 13289.75] // 129 Notes
}

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
