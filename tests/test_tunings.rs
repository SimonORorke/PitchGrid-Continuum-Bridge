use pitchgrid_continuum::tuning_params::TuningParams;

/// Gets parameters for PitchGrid's "Mavila (16-TET, 16 keys/8ve)" tuning.
pub fn params_16_16() -> TuningParams {
    TuningParams::new(1, 261.62558, 0.99999994, 0.56250846,
                      6.8750014, 16, 5, 2)
}

/// Gets parameters for PitchGrid's "Dicoid 17-TET (7L 3s)" tuning.
pub fn params_17_17() -> TuningParams {
    TuningParams::new(1, 256.86972, 0.99999994, 0.7058833,
                      7.333337, 17, 7, 3)
}

/// Gets parameters for PitchGrid's "31-TET (19 keys per 8ve)" tuning.
pub fn params_31_19() -> TuningParams {
    TuningParams::new(1, 261.62558, 0.99999994, 0.5806459,
                      8.250002, 19, 5, 2)
}
