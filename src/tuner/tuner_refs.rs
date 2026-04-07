use std::sync::{Arc, Mutex, OnceLock};
use crate::tuning_params::{TuningParams,};

pub type SharedTuningParams = Arc<Mutex<TuningParams>>;

static DEFAULT_KEY_PITCHES: OnceLock<Vec<f32>> = OnceLock::new();
static KEYS: OnceLock<Mutex<Vec<super::Key>>> = OnceLock::new();
static PARAMS: OnceLock<SharedTuningParams> = OnceLock::new();
static PITCH_TABLE_NOS: OnceLock<Vec<u8>> = OnceLock::new();
static ROOT_FREQ_OVERRIDE: OnceLock<Arc<Mutex<f32>>> = OnceLock::new();

pub(super) fn default_pitch_keys<'a>() -> &'a Vec<f32> {
    DEFAULT_KEY_PITCHES.get_or_init(|| create_default_key_pitches())
}

pub(super) fn keys_clone() -> Vec<super::Key> {
    KEYS.get_or_init(|| Mutex::new(vec![])).lock().unwrap().clone()
}

pub(super) fn set_keys(keys: Vec<super::Key>) {
    *KEYS.get_or_init(|| Mutex::new(vec![])).lock().unwrap() = keys;
}

pub(super) fn params_clone() -> SharedTuningParams {
    let params =
        PARAMS.get_or_init(|| Arc::new(Mutex::new(TuningParams::default())));
    Arc::clone(params)
}

pub(super) fn pitch_table_nos<'a>() -> &'a Vec<u8> {
    PITCH_TABLE_NOS.get_or_init(|| (80..88).collect())
}

pub(super) fn root_freq_override<'a>() -> &'a Arc<Mutex<f32>> {
    ROOT_FREQ_OVERRIDE.get_or_init(|| Arc::new(Mutex::new(0.0)))
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
