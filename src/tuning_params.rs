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
}

impl TuningParams {
    pub fn new(mode: i32, root_freq: f32, stretch: f32, skew: f32,
               mode_offset: f32, steps: i32, mos_a: i32, mos_b: i32) -> Self {
        Self { mode, root_freq, stretch, skew, mode_offset, steps, mos_a, mos_b }
    }

    /// This is not a meaningful default. It's just here so we don't have to encapsulate
    /// TuningParams instances in Options.
    pub fn default() -> Self {
        Self { mode: 0, root_freq: 0.0, stretch: 0.0, skew: 0.0,
            mode_offset: 0.0, steps: 0, mos_a: 0, mos_b: 0 }
    }

    pub fn mode(&self) -> i32 {
        self.mode
    }
    pub fn root_freq(&self) -> f32 {
        self.root_freq
    }
    pub fn stretch(&self) -> f32 {
        self.stretch
    }
    pub fn skew(&self) -> f32 {
        self.skew
    }
    pub fn mode_offset(&self) -> f32 {
        self.mode_offset
    }
    pub fn steps(&self) -> i32 {
        self.steps
    }
    pub fn mos_a(&self) -> i32 {
        self.mos_a
    }
    pub fn mos_b(&self) -> i32 {
        self.mos_b
    }
}
