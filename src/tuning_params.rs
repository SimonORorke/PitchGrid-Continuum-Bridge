use std::cmp::max;
use std::sync::OnceLock;
use cxx::UniquePtr;
use round::round;

/// Tuning parameters received from PitchGrid.
#[derive(Clone, Debug)]
pub struct TuningParams {
    mode: i32,
    root_freq: f32,
    stretch: f32,
    skew: f32,
    mode_offset: f32,
    steps: i32,
    mos_a: i32,
    mos_b: i32,
    root_freq_override: Option<f32>,
}

impl TuningParams {
    pub fn new(mode: i32, root_freq: f32, stretch: f32, skew: f32,
               mode_offset: f32, steps: i32, mos_a: i32, mos_b: i32) -> Self {
        Self { mode, root_freq, stretch, skew, mode_offset, steps, mos_a, mos_b,
               root_freq_override: None }
    }

    /// This is not a meaningful default. It's just here so we don't have to encapsulate
    /// TuningParams instances in Options.
    pub fn default() -> Self {
        Self { mode: 0, root_freq: 0.0, stretch: 0.0, skew: 0.0,
            mode_offset: 0.0, steps: 0, mos_a: 0, mos_b: 0, root_freq_override: None }
    }

    pub fn mode(&self) -> i32 { self.mode }
    pub fn root_freq(&self) -> f32 { self.root_freq }
    pub fn stretch(&self) -> f32 { self.stretch }
    pub fn skew(&self) -> f32 { self.skew }
    pub fn mode_offset(&self) -> f32 { self.mode_offset }
    pub fn steps(&self) -> i32 { self.steps }
    pub fn mos_a(&self) -> i32 { self.mos_a }
    pub fn mos_b(&self) -> i32 { self.mos_b }
    pub fn root_freq_override(&self) -> Option<f32> { self.root_freq_override }

    pub fn default_pitch_keys<'a>() -> &'a Vec<f32> {
        DEFAULT_KEY_PITCHES.get_or_init(|| create_default_key_pitches())
    }

    /// Calculates and returns the pitch required for each key in the MIDI range,
    /// given the tuning parameters.
    pub fn calculate_key_pitches(&mut self, root_freq_override_note_no: usize) -> Vec<f32> {
        let root_freq = if root_freq_override_note_no == 0 {
            self.root_freq_override = None;
            self.root_freq
        } else {
            let pitch = Self::default_pitch_keys()[root_freq_override_note_no];
            self.root_freq_override = Some(pitch);
            pitch
        };
        let steps = max(1, self.steps);
        let mos = mos_from_tuning_params(self);
        let a1 = ffi::vector2d(0.0, 0.0);
        let a2 = ffi::vector2d(
            ffi::get_mos_v_gen_x(&mos) as f64, ffi::get_mos_v_gen_y(&mos) as f64);
        let a3 = ffi::vector2d(
            ffi::get_mos_a(&mos) as f64, ffi::get_mos_b(&mos) as f64);
        let b1 = ffi::vector2d(
            0.0, (self.mode_offset as f64 + 0.5) / steps as f64);
        let b2 = ffi::vector2d(
            (self.skew * self.stretch) as f64,
            (self.mode_offset as f64 + 1.5) / steps as f64);
        let b3 = ffi::vector2d(
            self.stretch as f64,
            (self.mode_offset as f64 + 0.5) / steps as f64);
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

    /// Formats the tuning parameters for display.
    pub fn format_tuning_params(&self) -> FormattedTuningParams {
        let root_freq = self.root_freq_override.unwrap_or(self.root_freq);
        let mos = mos_from_tuning_params(self);
        FormattedTuningParams {
            root_freq: format!("{} Hz", round(root_freq as f64, 3)),
            // The stretch parameter is in octaves, so we need to multiply by 1200 to get the
            // number of cents to display.
            stretch: format!("{} ct", (self.stretch * 1200.0).round()),
            skew: format!("{}", round(self.skew as f64, 5)),
            mode_offset: format!("{}", round(self.mode_offset as f64, 5)),
            steps: format!("{}", self.steps),
            mos_large_step_count: format!("{}", ffi::get_mos_large_step_count(&mos)),
            mos_small_step_count: format!("{}", ffi::get_mos_small_step_count(&mos)),
        }
    }
}

/// Returns the default key pitches in Hz, where the default scale is 12-TET
/// at standard concert pitch A=440.
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

static DEFAULT_KEY_PITCHES: OnceLock<Vec<f32>> = OnceLock::new();

fn mos_from_tuning_params(tuning_params: &TuningParams) -> UniquePtr<ffi::MOS> {
    ffi::mos_from_params(
        tuning_params.mos_a(),
        tuning_params.mos_b(),
        tuning_params.mode(),
        tuning_params.stretch() as f64,
        tuning_params.skew() as f64)
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
