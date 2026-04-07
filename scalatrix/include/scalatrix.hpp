/**
 * Scalatrix - A C++ library for microtonal scale generation and tuning
 * 
 * Core Concept: "A scale is a path on a 2D lattice"
 * 
 * Scalatrix implements this fundamental idea through the Scale::fromAffine method:
 * 1. Apply an affine transform to redistribute 2D integer lattice nodes in space
 * 2. Select all nodes that fall within the horizontal strip 0 ≤ y < 1 after transformation  
 * 3. Order these nodes by increasing x-coordinate to form the sequential scale path
 * 4. Normalize by constraining transforms so origin (0,0) maps to line segment (x=0, 0≤y<1)
 * 
 * This approach generates scales by "slicing" through the transformed lattice, creating
 * sequential paths that respect the underlying mathematical structure while supporting
 * arbitrary regular temperaments and tuning systems.
 */
#ifndef SCALATRIX_HPP
#define SCALATRIX_HPP

#include "scalatrix/affine_transform.hpp"
#include "scalatrix/lattice.hpp"
#include "scalatrix/node.hpp"
#include "scalatrix/scale.hpp"
#include "scalatrix/params.hpp"
#include "scalatrix/mos.hpp"
#include "scalatrix/pitchset.hpp"
#include "scalatrix/label_calculator.hpp"

#include <memory>

// ========================================================================================
// pg34 If you add, remove, or modify Rust functions in the C++ to Rust interface defined
// in mod ff1 in tuner.rs, you must update the corresponding C++ functions here.
// ========================================================================================
// Wrapper functions for CXX bridge
namespace scalatrix {
    inline std::unique_ptr<AffineTransform>  affine_from_three_dots(
            const Vector2d& a1, const Vector2d& a2, const Vector2d& a3,
            const Vector2d& b1, const Vector2d& b2, const Vector2d& b3) {
        return std::make_unique<AffineTransform>(affineFromThreeDots(a1, a2, a3, b1, b2, b3));
    }

    inline int get_mos_a(const MOS& mos) {
        return mos.a;
    }

    inline int get_mos_b(const MOS& mos) {
        return mos.b;
    }

    inline int get_mos_v_gen_x(const MOS& mos) {
        return mos.v_gen.x;
    }

    inline int get_mos_v_gen_y(const MOS& mos) {
        return mos.v_gen.y;
    }

    inline double get_node_pitch(const Node& node) {
        return node.pitch;
    }

    inline std::unique_ptr<std::vector<Node>> get_scale_nodes(const Scale& scale) {
        return std::make_unique<std::vector<Node>>(scale.getNodes());
    }

    inline std::unique_ptr<MOS> mos_from_g(int depth, int m, double g, double e, int repetitions) {
        return std::make_unique<MOS>(MOS::fromG(depth, m, g, e, repetitions));
    }

    inline std::unique_ptr<Scale> scale_from_affine(
            const AffineTransform& affine, const double base_freq,
            int num_nodes_to_generate, int root_index) {
        return std::make_unique<Scale>(Scale::fromAffine(affine, base_freq,
            num_nodes_to_generate, root_index));
    }

    inline std::unique_ptr<Vector2d> vector2d(double x, double y) {
        return std::make_unique<Vector2d>(Vector2d(x, y));
    }
}

#endif // SCALATRIX_HPP
