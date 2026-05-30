use slint::ModelRc;
use crate::{MainWindow, ComboBoxItem, SlintMessageType};
use crate::global::DeviceType;
use crate::i_midi::IMidi;
use crate::midi_ports::IIo;
use crate::i_settings::ISettings;

/// This trait is used to implement strategies that depend on whether a MIDI device is input or
/// output.
/// In the Model-View-Controller (MVC) pattern, it contains both view and controller methods.
/// See `Controller`'s doc comment for more information on how the project implements MVC.
pub trait DeviceStrategy: Send + Sync {
    fn device_type(&self) -> &DeviceType;
    fn io<'a>(&self, midi: &'a dyn IMidi) -> &'a dyn IIo;

    /// Makes a clone of the current strategy that needs to be used when cross-threading.
    /// The code is the same for all strategies. But the compiler does not allow it to be
    /// implemented here as a default method.
    fn clone_box(&self) -> Box<dyn DeviceStrategy>;

    fn focus_device(&self, main_window: &MainWindow);
    fn device_setting<'a>(&self, settings: &'a dyn ISettings) -> &'a str;
    fn set_device_setting(&self, settings: &mut dyn ISettings, device_name: &str);
    fn show_connected_device_name(
        &self, main_window: &MainWindow, device_name: &str, message_type: SlintMessageType);
    fn set_devices_model(&self, main_window: &MainWindow, model: ModelRc<ComboBoxItem>);
    fn get_selected_device_index(&self, main_window: &MainWindow) -> i32;
    fn set_selected_device_index(&self, main_window: &MainWindow, index: i32);
    fn msg_cannot_connect(&self, device_name: &str) -> &str;
    fn msg_connect(&self) -> &str;
    fn msg_connected(&self, device_name: &str) -> &str;
    fn msg_not_selected(&self) -> &str;
    fn msg_refreshed_reconnect(&self) -> &str;
    fn other_device_strategy(&self) -> Box<dyn DeviceStrategy>;
}

#[derive(Clone)]
pub struct InputStrategy {
}

impl InputStrategy {
    pub fn new() -> Self {
        Self {}
    }
}

impl DeviceStrategy for InputStrategy {
    fn device_type(&self) -> &DeviceType {
        &DeviceType::Input
    }

    fn io<'a>(&self, midi: &'a dyn IMidi) -> &'a dyn IIo {
        midi.input()
    }

    fn clone_box(&self) -> Box<dyn DeviceStrategy> {
        Box::new(self.clone())
    }

    fn focus_device(&self, main_window: &MainWindow) {
        main_window.invoke_input_focus();
    }

    fn device_setting<'a>(&self, settings: &'a dyn ISettings) -> &'a str {
        settings.midi_input_device()
    }

    fn set_device_setting(&self, settings: &mut dyn ISettings, device_name: &str) {
        settings.set_midi_input_device(device_name);
    }

    fn show_connected_device_name(&self, main_window: &MainWindow, device_name: &str,
                                message_type: SlintMessageType) {
        main_window.invoke_input_show_connected_device_name(device_name.into(), message_type);
    }

    fn set_devices_model(&self, main_window: &MainWindow, model: ModelRc<ComboBoxItem>) {
        main_window.set_input_devices_model(model);
    }

    fn get_selected_device_index(&self, main_window: &MainWindow) -> i32 {
        let index = main_window.get_input_selected_device_index();
        // println!("InputStrategy.get_selected_device_index: returning selected device index {}", index);
        index
    }

    fn set_selected_device_index(&self, main_window: &MainWindow, index: i32) {
        // println!("InputStrategy.set_selected_device_index: Setting selected device index to {}", index);
        main_window.set_input_selected_device_index(index);
    }

    fn msg_cannot_connect(&self, device_name: &str) -> &str {
        Box::leak(format!("Cannot connect MIDI input device {}. The device may be in use.",
                          device_name).into_boxed_str())
    }

    fn msg_connect(&self) -> &str {
        "Connect MIDI input device"
    }

    fn msg_connected(&self, device_name: &str) -> &str {
        Box::leak(format!("Connected MIDI input device {}", device_name).into_boxed_str())
    }

    fn msg_not_selected(&self) -> &str {
        "No MIDI input device selected."
    }

    fn msg_refreshed_reconnect(&self) -> &str {
        "Refreshed MIDI input devices. You must (re)connect."
    }

    fn other_device_strategy(&self) -> Box<dyn DeviceStrategy> {
        Box::new(OutputStrategy::new())
    }
}

#[derive(Clone)]
pub struct OutputStrategy {
}

impl OutputStrategy {
    pub fn new() -> Self {
        Self {}
    }
}

impl DeviceStrategy for OutputStrategy {
    fn device_type(&self) -> &DeviceType {
        &DeviceType::Output
    }

    fn io<'a>(&self, midi: &'a dyn IMidi) -> &'a dyn IIo {
        midi.output()
    }

    fn clone_box(&self) -> Box<dyn DeviceStrategy> {
        Box::new(self.clone())
    }

    fn focus_device(&self, main_window: &MainWindow) {
        main_window.invoke_output_focus();
    }

    fn device_setting<'a>(&self, settings: &'a dyn ISettings) -> &'a str {
        settings.midi_output_device()
    }

    fn set_device_setting(&self, settings: &mut dyn ISettings, device_name: &str) {
        settings.set_midi_output_device(device_name);
    }

    fn show_connected_device_name(&self, main_window: &MainWindow, device_name: &str,
                                message_type: SlintMessageType) {
        main_window.invoke_output_show_connected_device_name(device_name.into(), message_type);
    }

    fn set_devices_model(&self, main_window: &MainWindow, model: ModelRc<ComboBoxItem>) {
        main_window.set_output_devices_model(model);
    }

    fn get_selected_device_index(&self, main_window: &MainWindow) -> i32 {
        let index = main_window.get_output_selected_device_index();
        // println!("OutputStrategy.get_selected_device_index: returning selected device index {}", index);
        index
    }

    fn set_selected_device_index(&self, main_window: &MainWindow, index: i32) {
        // println!("OutputStrategy.set_selected_device_index: Setting selected device index to {}", index);
        main_window.set_output_selected_device_index(index);
    }

    fn msg_cannot_connect(&self, device_name: &str) -> &str {
        Box::leak(format!("Cannot connect MIDI output device {}. The device may be in use.",
                          device_name).into_boxed_str())
    }

    fn msg_connect(&self) -> &str {
        "Connect MIDI output device"
    }

    fn msg_connected(&self, device_name: &str) -> &str {
        Box::leak(format!("Connected MIDI output device {}", device_name).into_boxed_str())
    }

    fn msg_not_selected(&self) -> &str {
        "No MIDI output device selected."
    }

    fn msg_refreshed_reconnect(&self) -> &str {
        "Refreshed MIDI output devices. You must (re)connect."
    }

    fn other_device_strategy(&self) -> Box<dyn DeviceStrategy> {
        Box::new(InputStrategy::new())
    }
}
