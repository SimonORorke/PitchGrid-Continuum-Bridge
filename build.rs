fn main() {
    slint_build::compile_with_config(
        "ui/main_window.slint",
        slint_build::CompilerConfiguration::new().with_library_paths(vivi_ui::import_paths()),
    )
    .unwrap();
    let mut builder = cxx_build::bridge("src/tuning_params.rs"); // returns a cc::Build
    // If you add, remove, or rename cpp files in the scalatrix/src directory,
    // you must also update this, in order for the C++ code to be compiled.
    builder
        .file("scalatrix/src/affine_transform.cpp")
        .file("scalatrix/src/consonance.cpp")
        .file("scalatrix/src/label_calculator.cpp")
        .file("scalatrix/src/lattice.cpp")
        .file("scalatrix/src/linear_solver.cpp")
        .file("scalatrix/src/main.cpp")
        .file("scalatrix/src/mos.cpp")
        .file("scalatrix/src/node.cpp")
        .file("scalatrix/src/params.cpp")
        .file("scalatrix/src/pitchset.cpp")
        // .file("scalatrix/src/python_bindings.cpp") // Causes compiler error.
        .file("scalatrix/src/scale.cpp")
        .file("scalatrix/src/spectrum.cpp")
        .include("scalatrix/include")
        .std("c++17");

    // Check the target platform instead of using a build directive,
    // in case the build script is run on a non-macOS platform.
    let is_macos = std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos");
    if is_macos {
        builder.file("src/mac_lifecycle.mm");
        println!("cargo:rustc-link-lib=framework=Cocoa");
        println!("cargo:rerun-if-changed=src/mac_lifecycle.mm");
    }

    builder.compile("scalatrix");
    println!("cargo:rerun-if-changed=scalatrix");

    // Check the target platform instead of using a build directive,
    // in case the build script is run on a non-Windows platform.
    let is_windows = std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows");
    if is_windows {
        // Generate a manifest to specify the application icon and the exe file properties.
        let manifest_dir = std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
        let out_dir = std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap());
        let icon_path = std::path::PathBuf::from("ui/images/Midi port black on red 512.ico");
        let version = app_info::VERSION;
        let product_name = app_info::APP_TITLE;
        let copyright = app_info::COPYRIGHT;
        let rc_path = simonor::BuildHelper::configure_app_icon_and_exe_file_properties(
            &manifest_dir, &out_dir, &icon_path, version, product_name, copyright);
        let _ = embed_resource::compile(&rc_path, embed_resource::NONE);
    }
}
