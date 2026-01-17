// Prevent console window in addition to Slint window in Windows release builds when, e.g., starting the app via file manager. Ignored on other platforms.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

slint::include_modules!();

fn main() {
    // use slint::Model; // For for loops on Slint components.
    let main_window = MainWindow::new().unwrap();
    main_window.set_window_title("PitchGrid-Continuum Companion".into());

    main_window.on_midi_input_changed(|index| {
        println!("MIDI input changed to {}", index);
    });

    main_window.run().unwrap();
}
