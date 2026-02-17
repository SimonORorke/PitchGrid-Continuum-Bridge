use std::cmp::max;

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
    let note_pitches = calculate_note_pitches(
        max(1, depth), mode, root_freq, stretch, skew, mode_offset, max(1, steps));
    for (i, pitch) in note_pitches.iter().enumerate() {
        println!("note {}: {}", i, pitch);
    }
}

/// Calculates and returns the pitch of each note in the MIDI range,
/// given the tuning parameters.
fn calculate_note_pitches(depth: i32, mode: i32, root_freq: f32, stretch: f32,
                 skew: f32, mode_offset: i32, steps: i32) -> Vec<f64> {
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
    scale_nodes.iter().map(|node| ffi::get_node_pitch(&node)).collect()
}