use std::cmp::max;

#[cxx::bridge(namespace = "scalatrix")]
mod ffi {
    unsafe extern "C++" {
        include!("scalatrix.hpp");

        type MOS;
        fn mosFromG(depth: i32, m: i32, g: f64, e: f64, repetitions: i32) -> UniquePtr<MOS>;

        /// Getter for nL field
        fn get_nL(mos: &MOS) -> i32;

        /// Getter for nS field
        fn get_nS(mos: &MOS) -> i32;
    }
}

/// Update tuning parameters from the OSC message.
///     Args:
///         depth: MOS depth (generation)
///         mode: Mode index
///         root_freq: Root frequency in Hz
///         stretch: Stretch factor
///         skew: Skew factor
///         mode_offset: Mode offset
///         steps: Number of steps per period
pub fn update_tuning(depth: i32, mode: i32, root_freq: f32, stretch: f32,
                     skew: f32, mode_offset: i32, steps: i32) {
    // Recalculate MOS and scale degrees
    calculate_mos(max(1, depth), mode, root_freq, stretch, skew, mode_offset, max(1, steps));
}

/// Calculate MOS from current tuning parameters.
#[allow(unused)]
fn calculate_mos(depth: i32, mode: i32, root_freq: f32, stretch: f32,
                 skew: f32, mode_offset: i32, steps: i32) {
    // Create MOS using fromG (depth, mode, skew, stretch, repetitions)
    let mos = ffi::mosFromG(
        depth,
        mode,
        skew as f64,
        stretch as f64,
        1);
    // For now, use chromatic mapping (all notes in the EDO).
    let scale_degrees = (0..steps).collect::<Vec<i32>>();
    // Get L and s count directly from MOS
    // nL = number of large steps, nS = number of small steps
    let l_count = ffi::get_nL(&mos);
    let s_count = ffi::get_nS(&mos);
}