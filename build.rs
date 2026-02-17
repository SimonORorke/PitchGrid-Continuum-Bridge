fn main() {
    slint_build::compile_with_config(
        "ui/main_window.slint",
        slint_build::CompilerConfiguration::new().with_library_paths(vivi_ui::import_paths()),
    )
    .unwrap();
    cxx_build::bridge("src/tuner.rs")  // returns a cc::Build
        .file("scalatrix/src/affine_transform.cpp")
        .file("scalatrix/src/label_calculator.cpp")
        .file("scalatrix/src/lattice.cpp")
        .file("scalatrix/src/linear_solver.cpp")
        .file("scalatrix/src/main.cpp")
        .file("scalatrix/src/mos.cpp")
        .file("scalatrix/src/node.cpp")
        .file("scalatrix/src/params.cpp")
        .file("scalatrix/src/pitchset.cpp")
        // .file("scalatrix/src/python_bindings.cpp")
        .file("scalatrix/src/scale.cpp")
        .include("scalatrix/include")
        .std("c++17")
        .compile("scalatrix");
    println!("cargo:rerun-if-changed=scalatrix");
}
