mod tuner_refs;

use std::cmp::max;
use std::sync::atomic::{AtomicBool, AtomicU8, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use cxx::UniquePtr;
use round::round;
use crate::midi::Midi;
use crate::midi_sender::{IMidiSender, MidiSender};
use crate::tuner::ffi::MOS;
use crate::tuning_params::TuningParams;

pub use crate::i_tuner::{ITuner, SharedTuner};

/// Returns the currently selected pitch table number.
/// Remains a free function so that midi.rs can call it without holding a tuner reference.
pub fn pitch_table() -> u8 {
    PITCH_TABLE.load(Ordering::Relaxed)
}

pub fn pitch_tables<'a>() -> &'a Vec<u8> {
    PITCH_TABLES.get_or_init(|| (80..88).collect())
}

pub fn default_pitch_table() -> u8 { 80 }

/// A facility for tuning a Continuum from PitchGrid parameters.
pub struct Tuner {
    is_already_updating: AtomicBool,
    is_another_update_pending: AtomicBool,
    override_rounding_initial: AtomicBool,
    override_rounding_rate: AtomicBool,
    rounding_rate: AtomicU8,
    root_freq_override_note_no: AtomicUsize,
    keys: Mutex<Vec<Key>>,
    midi_sender: Mutex<Box<dyn IMidiSender>>,
    params: Arc<Mutex<TuningParams>>,
    root_freq_override: Arc<Mutex<f32>>,
}

impl Tuner {
    pub fn new() -> Self {
        Self {
            is_already_updating: AtomicBool::new(false),
            is_another_update_pending: AtomicBool::new(false),
            override_rounding_initial: AtomicBool::new(false),
            override_rounding_rate: AtomicBool::new(false),
            rounding_rate: AtomicU8::new(127),
            root_freq_override_note_no: AtomicUsize::new(0),
            keys: Mutex::new(vec![]),
            midi_sender: Mutex::new(Box::new(MidiSender::new())),
            params: Arc::new(Mutex::new(TuningParams::default())),
            root_freq_override: Arc::new(Mutex::new(0.0)),
        }
    }

    /// Calculates the tuning and either sends it to the instrument, provided another tuning update
    /// is not already in progress, or stores it for sending once the current update completes.
    fn tune(&self) {
        println!("tuner.tune");
        let send_now: bool;
        {
            let params = self.params.lock().unwrap();
            let key_pitches = self.calculate_key_pitches(TuningParams::new(
                params.mode(), params.root_freq(), params.stretch(),
                params.skew(), params.mode_offset(), max(1, params.steps()),
                params.mos_a(), params.mos_b()));
            *self.keys.lock().unwrap() = key_pitches.iter().enumerate()
                .map(|(i, pitch)| Key {
                    number: i as u8,
                    required_pitch: *pitch,
                    to_number: 0,
                    offset_ratio: 0.0,
                    offset_msb: 0,
                    offset_lsb: 0,
                }).collect();
            // If the player sweeps one of the tuning controls in PitchGrid,
            // we will receive new tunings much faster than the instrument can update the tuning
            // table, which takes in the order of half a second.
            // If we were to keep sending tunings to the instrument regardless, its processor would
            // be swamped, probably for minutes.
            // The solution is to not send more updates to the instrument while another update is
            // in progress and, once the update is complete, send the most recently received
            // following tuning if there is one.
            let is_already_updating = self.is_already_updating.load(Ordering::Relaxed);
            println!("tuner.tune: is_already_updating = {is_already_updating}");
            if is_already_updating {
                self.is_another_update_pending.store(true, Ordering::Relaxed);
                send_now = false;
            } else {
                println!("tuner.tune: Setting is_already_updating to true");
                self.is_already_updating.store(true, Ordering::Relaxed);
                send_now = true;
            }
        }
        if send_now {
            self.send_tuning_update(true);
        }
    }

    /// Calculates and returns the pitch required for each key in the MIDI range,
    /// given the tuning parameters.
    fn calculate_key_pitches(&self, tuning_params: TuningParams) -> Vec<f32> {
        let root_freq = {
            let mut override_freq = self.root_freq_override.lock().unwrap();
            if self.root_freq_override_note_no.load(Ordering::Relaxed) == 0 {
                *override_freq = 0.0;
                tuning_params.root_freq()
            } else {
                let note_no = self.root_freq_override_note_no.load(Ordering::Relaxed);
                let pitch = tuner_refs::default_pitch_keys()[note_no];
                *override_freq = pitch;
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
            (tuning_params.skew() * tuning_params.stretch()) as f64,
            (tuning_params.mode_offset() as f64 + 1.5) / tuning_params.steps() as f64);
        let b3 = ffi::vector2d(
            tuning_params.stretch() as f64,
            (tuning_params.mode_offset() as f64 + 0.5) / tuning_params.steps() as f64);
        let affine = ffi::affine_from_three_dots(&a1, &a2, &a3, &b1, &b2, &b3);
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

    /// Sends the instrument a tuning update.
    ///
    /// generate: Whether a tuning table is to be generated from the tuning parameters received
    /// from PitchGrid and sent to the instrument. Set to false if the latest tuning table has
    /// previously been sent to the instrument.
    ///
    /// The tuning table will be assigned to the instrument's current preset, which will also be
    /// updated with any rounding parameters that have been specified.
    fn send_tuning_update(&self, generate: bool) {
        println!("tuner.send_tuning_update: generate = {}", generate);
        Midi::on_updating_tuning();
        if generate {
            let mut keys = self.keys.lock().unwrap().clone();
            self.set_to_key_numbers(&mut keys);
            self.calculate_offsets(&mut keys);
            self.send_pitch_table(pitch_table(), &keys);
        }
        // The following commands update the instrument's current preset.
        self.send_rounding_params();
        // Set active pitch table for performance.
        println!("tuner.send_tuning_update: Setting active pitch table to {}", pitch_table());
        self.midi_sender.lock().unwrap().send_control_change(16, 51, pitch_table());
    }

    /// Sets the to_number field of each Key to the index of the pitch in DEFAULT_KEY_PITCHES
    /// that matches the Key's pitch.
    fn set_to_key_numbers(&self, keys: &mut Vec<Key>) {
        for key_being_matched in keys.iter_mut() {
            key_being_matched.to_number = tuner_refs::default_pitch_keys()
                .binary_search_by(|&x| x.partial_cmp(&key_being_matched.required_pitch).unwrap())
                .unwrap_or_else(|i| {
                    if i == 0 { 0 } else { i - 1 }
                }) as u8;
        }
    }

    /// Calculates the offset ratio and 14-bit offset values for each note.
    fn calculate_offsets(&self, keys: &mut Vec<Key>) {
        for i in 0..keys.len() {
            let note_pitch = keys[i].required_pitch;
            let to_note_pitch = tuner_refs::default_pitch_keys()[keys[i].to_number as usize];
            let mut offset_ratio = 12.0 * (note_pitch / to_note_pitch).log2();
            if offset_ratio > 1.0 {
                offset_ratio = 1.0;
            } else if offset_ratio < 0.0 {
                offset_ratio = 0.0;
            }
            keys[i].offset_ratio = offset_ratio;
            let offset_14bit = (offset_ratio * 16383.0).round() as u16;
            keys[i].offset_msb = ((offset_14bit >> 7) & 0x7F) as u8;
            keys[i].offset_lsb = (offset_14bit & 0x7F) as u8;
        }
    }

    fn send_pitch_table(&self, pitch_table: u8, keys: &Vec<Key>) {
        let sender = self.midi_sender.lock().unwrap();
        // Select pitch table to update.
        sender.send_control_change(16, 109, pitch_table);
        // Tuning for each MIDI key
        for key in keys {
            // Base/From MIDI key
            sender.send_control_change(16, 38, key.number);
            // Base/From MIDI key tuning MSB
            sender.send_control_change(16, 38, 0);
            // Base/From MIDI key tuning LSB
            sender.send_control_change(16, 38, 0);
            // Re-tuned/To MIDI key
            sender.send_control_change(16, 38, key.to_number);
            // Re-tuned/To MIDI key tuning MSB
            sender.send_control_change(16, 38, key.offset_msb);
            // Re-tuned/To MIDI key tuning LSB
            sender.send_control_change(16, 38, key.offset_lsb);
        }
        // Save pitch table on instrument.
        sender.send_control_change(16, 109, 101);
    }

    /// Sends pitch rounding parameters, if required, to the instrument.
    fn send_rounding_params(&self) {
        if self.override_rounding_initial.load(Ordering::Relaxed) {
            // Turn on Rounding Initial
            println!("tuner.send_rounding_params: Sending Rounding Initial");
            self.midi_sender.lock().unwrap().send_control_change(1, 28, 127); // RndIni
        }
        if self.override_rounding_rate.load(Ordering::Relaxed) {
            // Rounding Mode Normal
            println!("tuner.send_rounding_params: Sending Rounding Mode Normal");
            self.midi_sender.lock().unwrap().send_matrix_poke(10, 0); // RoundMode
            // Rounding Rate
            println!("tuner.send_rounding_params: Sending Rounding Rate");
            self.midi_sender.lock().unwrap().send_control_change(
                1, 25, self.rounding_rate.load(Ordering::Relaxed)); // RoundRate
        }
    }
}

impl ITuner for Tuner {
    fn init(&self, pitch_table: u8) {
        PITCH_TABLE.store(pitch_table, Ordering::Relaxed);
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
    fn on_tuning_received(&self, params: TuningParams) {
        println!(
            "tuner.on_tuning_received: mode = {}; root_freq = {}; stretch = {}; \
            skew = {}; mode_offset = {}; steps = {}; mos_a = {}; mos_b = {}",
            params.mode(), params.root_freq(), params.stretch(),
            params.skew(), params.mode_offset(), params.steps(), params.mos_a(), params.mos_b());
        *self.params.lock().unwrap() = params;
        self.tune();
    }

    fn has_data(&self) -> bool {
        self.keys.lock().unwrap().len() > 0
    }

    fn remove_data(&self) {
        println!("tuner.remove_data: Setting is_already_updating to false");
        self.is_already_updating.store(false, Ordering::Relaxed);
        self.is_another_update_pending.store(false, Ordering::Relaxed);
        *self.params.lock().unwrap() = TuningParams::default();
        *self.keys.lock().unwrap() = vec![];
    }

    /// If a tuning table generated from tuning parameters received from PitchGrid has previously
    /// been sent to the instrument, sends a tuning update for the instrument's current preset.
    /// The current tuning table will be assigned to the preset, which will also be updated
    /// with any rounding parameters that have been specified.
    /// Returns whether an update has been sent.
    fn send_current_preset_update(&self) -> bool {
        println!("tuner.send_current_preset_update");
        let can_update = self.has_data();
        if can_update {
            println!("tuner.send_current_preset_update: Sending update");
            self.send_tuning_update(false);
        }
        can_update
    }

    /// The tuning parameters formatted for display.
    fn formatted_tuning_params(&self) -> FormattedTuningParams {
        if !self.has_data() {
            return FormattedTuningParams::default();
        }
        let params = self.params.lock().unwrap();
        let root_freq = {
            if self.root_freq_override_note_no.load(Ordering::Relaxed) == 0 {
                // No override
                params.root_freq()
            } else {
                self.root_freq_override.lock().unwrap().clone()
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

    fn is_root_freq_overridden(&self) -> bool {
        self.root_freq_override_note_no.load(Ordering::Relaxed) != 0
    }

    /// Sets the root frequency override and optionally sends it to the instrument.
    fn set_root_freq_override_note_no(&self, index: usize, send_tuning: bool) {
        let note_no = if index == 0 {
            0usize // No override
        } else {
            index + 53 // E.g. for middle C, index = 7, note_no = 60.
        };
        self.root_freq_override_note_no.store(note_no, Ordering::Relaxed);
        if send_tuning {
            self.tune();
        }
    }

    /// Sets whether initial rounding is to be overridden the next time tuning is sent.
    fn set_override_rounding_initial(&self, value: bool) {
        self.override_rounding_initial.store(value, Ordering::Relaxed);
    }

    /// Sets whether rounding rate is to be overridden the next time tuning is sent.
    fn set_override_rounding_rate(&self, value: bool) {
        self.override_rounding_rate.store(value, Ordering::Relaxed);
    }

    /// Sets the rounding rate, if it is to be overridden, the next time tuning is sent.
    fn set_rounding_rate(&self, rate: u8) {
        self.rounding_rate.store(rate, Ordering::Relaxed);
    }

    fn set_pitch_table(&self, pitch_table: u8) {
        PITCH_TABLE.store(pitch_table, Ordering::Relaxed);
    }

    fn on_tuning_updated(&self) {
        println!("tuner.on_tuning_updated");
        if !self.has_data() {
            println!("tuner.on_tuning_updated: no tuning data");
            // Could be tuning updated when an instrument preset is loaded
            // while PitchGrid is not connected.
            return;
        }
        // See comment in on_tuning_received.
        let send_again: bool;
        {
            let is_another_update_pending = self.is_another_update_pending.load(Ordering::Relaxed);
            println!("tuner.on_tuning_updated: is_another_update_pending = {is_another_update_pending}");
            if is_another_update_pending {
                self.is_another_update_pending.store(false, Ordering::Relaxed);
                send_again = true;
            } else {
                println!("tuner.on_tuning_updated: Setting is_already_updating to false");
                self.is_already_updating.store(false, Ordering::Relaxed);
                send_again = false;
            }
        }
        if send_again {
            self.send_tuning_update(true);
        }
    }

    /// Replaces the MIDI sender for testing.
    fn set_midi_sender(&self, sender: Box<dyn IMidiSender>) {
        *self.midi_sender.lock().unwrap() = sender;
    }

    fn pitch_table_index(&self) -> usize {
        // Return the index of the pitch_tables item that equals pitch_table.
        pitch_tables().iter().position(|&x| x == pitch_table()).unwrap_or(0)
    }
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

#[derive(Clone, Debug)]
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
#[derive(Clone)]
pub struct FormattedTuningParams {
    pub root_freq: String, pub stretch: String,
    pub skew: String, pub mode_offset: String, pub steps: String,
    pub mos_large_step_count: String, pub mos_small_step_count: String,
}

impl Default for FormattedTuningParams {
    fn default() -> Self {
        Self {
            root_freq: String::new(), stretch: String::new(),
            skew: String::new(), mode_offset: String::new(), steps: String::new(),
            mos_large_step_count: String::new(), mos_small_step_count: String::new(),
        }
    }
}

/// About the name Pitch Table.
/// The EaganMatrix Overlay Developer's Guide calls them tuning grids.
/// But naming is inconsistent in the Continuum User Guide.
/// The main documentation calls them pitch tables or custom grids, though there are also
/// scattered references to tuning grids.
/// We call them pitch tables, as that has the best chance of being understood by users.
///
/// Remains a static so that midi.rs can call pitch_table() without holding a tuner reference.
static PITCH_TABLE: AtomicU8 = AtomicU8::new(0);

static PITCH_TABLES: OnceLock<Vec<u8>> = OnceLock::new();