fn main() {
    slint_build::compile_with_config(
        "ui/main_window.slint",
        slint_build::CompilerConfiguration::new().with_library_paths(vivi_ui::import_paths()),
    )
    .unwrap();
    cxx_build::bridge("src/tuner.rs")  // returns a cc::Build
        // ========================================================================================
        // pg34 If you add, remove, or rename cpp files in the scalatrix/scv directory,
        // you must also update this, in order for the C++ code to be compiled.
        // Can any of these by easily identified as unnecessary and removed?
        // ========================================================================================
        .file("scalatrix/src/affine_transform.cpp")
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
        .include("scalatrix/include")
        .std("c++17")
        .compile("scalatrix");
    println!("cargo:rerun-if-changed=scalatrix");
    #[cfg(windows)]
    {
        // Generate a manifest to specify the application icon and the exe file properties.
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        let out_dir = std::env::var("OUT_DIR").unwrap();
        let version = app_info::VERSION;
        let description = app_info::APP_TITLE;
        let copyright = app_info::COPYRIGHT;
        let parts: Vec<u64> = version.split('.')
            .map(|s| s.parse().unwrap_or(0))
            .collect();
        let (major, minor, patch) = (
            parts.first().copied().unwrap_or(0),
            parts.get(1).copied().unwrap_or(0),
            parts.get(2).copied().unwrap_or(0),
        );
        // Use forward slashes so rc.exe accepts the path
        let icon_path = std::path::Path::new(&manifest_dir)
            .join("Midi port black on red 512.ico")
            .to_string_lossy()
            .replace('\\', "/");
        let rc = format!(
            // The pragma tells rc.exe to treat the file as UTF-8 — the same way it handles
            // strings in .NET projects, which we know works to not escape the apostrophe in
            // O'Rorke in the copyright string.
            "#pragma code_page(65001)\n\
             1 ICON \"{icon_path}\"\n\
             1 VERSIONINFO\n\
             FILEVERSION {major},{minor},{patch},0\n\
             PRODUCTVERSION {major},{minor},{patch},0\n\
             BEGIN\n\
               BLOCK \"StringFileInfo\"\n\
               BEGIN\n\
                 BLOCK \"040904B0\"\n\
                 BEGIN\n\
                   VALUE \"FileVersion\", \"{version}\"\n\
                   VALUE \"ProductVersion\", \"{version}\"\n\
                   VALUE \"ProductName\", \"{description}\"\n\
                   VALUE \"LegalCopyright\", \"{copyright}\"\n\
                 END\n\
               END\n\
               BLOCK \"VarFileInfo\"\n\
               BEGIN\n\
                 VALUE \"Translation\", 0x0409, 0x04B0\n\
               END\n\
             END\n"
        );
        let rc_path = std::path::Path::new(&out_dir).join("version.rc");
        std::fs::write(&rc_path, rc.as_bytes()).unwrap();
        let _ = embed_resource::compile(&rc_path, embed_resource::NONE);
    }
}
