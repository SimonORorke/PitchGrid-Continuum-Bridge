use log::trace;
use slint::ModelRc;
use simonor::{DeviceType, IMidiManager, MidiDeviceStrategy, MidiInputStrategy, MidiOutputStrategy};
use simonor::{IIo, MidiDesktopSettings};
use crate::{ComboBoxItem, MainWindow, SlintMessageType};

/// This trait is used to implement UI strategies that depend on whether a MIDI device is input or
/// output.
pub trait UiDeviceStrategy: Send + Sync {
    fn device_type(&self) -> &DeviceType;
    fn midi_strategy(&self) -> &dyn MidiDeviceStrategy;
    fn io<'a>(&self, midi: &'a dyn IMidiManager) -> &'a dyn IIo;

    /// Makes a clone of the current strategy that needs to be used when cross-threading.
    /// The code is the same for all strategies. But the compiler does not allow it to be
    /// implemented here as a default method.
    fn clone_box(&self) -> Box<dyn UiDeviceStrategy>;

    fn focus_device(&self, main_window: &MainWindow);
    fn device_setting<'a>(&self, settings: &'a dyn MidiDesktopSettings) -> &'a str;
    fn set_device_setting(&self, settings: &mut dyn MidiDesktopSettings, device_name: &str);
    fn show_connected_device_name(
        &self, main_window: &MainWindow, device_name: &str, message_type: SlintMessageType);
    fn set_devices_model(&self, main_window: &MainWindow, model: ModelRc<ComboBoxItem>);
    fn get_selected_device_index(&self, main_window: &MainWindow) -> i32;
    fn set_selected_device_index(&self, main_window: &MainWindow, index: i32);
    fn msg_cannot_connect(&self, device_name: &str) -> String;
    fn msg_connect(&self) -> &str;
    fn msg_connected(&self, device_name: &str) -> String;
    fn msg_refreshed_reconnect(&self) -> &str;
    fn other_device_strategy(&self) -> Box<dyn UiDeviceStrategy>;
}

#[derive(Clone)]
pub struct UiInputStrategy {
    midi_strategy: Box<dyn MidiDeviceStrategy>,
}

impl UiInputStrategy {
    pub fn new() -> Self {
        Self { midi_strategy: Box::new(MidiInputStrategy::new()) }
    }
}

impl UiDeviceStrategy for UiInputStrategy {
    fn device_type(&self) -> &DeviceType {
        self.midi_strategy.device_type()
    }

    fn midi_strategy(&self) -> &dyn MidiDeviceStrategy {
        &*self.midi_strategy
    }

    fn io<'a>(&self, midi: &'a dyn IMidiManager) -> &'a dyn IIo {
        self.midi_strategy.io(midi)
    }

    fn clone_box(&self) -> Box<dyn UiDeviceStrategy> {
        Box::new(self.clone())
    }

    fn focus_device(&self, main_window: &MainWindow) {
        main_window.invoke_input_focus();
    }

    fn device_setting<'a>(&self, settings: &'a dyn MidiDesktopSettings) -> &'a str {
        self.midi_strategy.device_setting(settings)
    }

    fn set_device_setting(&self, settings: &mut dyn MidiDesktopSettings, device_name: &str) {
        self.midi_strategy.set_device_setting(settings, device_name);
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
        trace!("InputStrategy.get_selected_device_index: returning selected device index {}", index);
        index
    }

    fn set_selected_device_index(&self, main_window: &MainWindow, index: i32) {
        trace!("InputStrategy.set_selected_device_index: Setting selected device index to {}", index);
        main_window.set_input_selected_device_index(index);
    }

    fn msg_cannot_connect(&self, device_name: &str) -> String {
        self.midi_strategy.msg_cannot_connect(device_name)
    }

    fn msg_connect(&self) -> &str {
        self.midi_strategy.msg_connect()
    }

    fn msg_connected(&self, device_name: &str) -> String {
        self.midi_strategy.msg_connected(device_name)
    }

    fn msg_refreshed_reconnect(&self) -> &str {
        self.midi_strategy.msg_refreshed_reconnect()
    }

    fn other_device_strategy(&self) -> Box<dyn UiDeviceStrategy> {
        Box::new(UiOutputStrategy::new())
    }
}

#[derive(Clone)]
pub struct UiOutputStrategy {
    midi_strategy: Box<dyn MidiDeviceStrategy>,
}

impl UiOutputStrategy {
    pub fn new() -> Self {
        Self { midi_strategy: Box::new(MidiOutputStrategy::new()) }
    }
}

impl UiDeviceStrategy for UiOutputStrategy {
    fn device_type(&self) -> &DeviceType {
        self.midi_strategy.device_type()
    }

    fn midi_strategy(&self) -> &dyn MidiDeviceStrategy {
        &*self.midi_strategy
    }

    fn io<'a>(&self, midi: &'a dyn IMidiManager) -> &'a dyn IIo {
        self.midi_strategy.io(midi)
    }

    fn clone_box(&self) -> Box<dyn UiDeviceStrategy> {
        Box::new(self.clone())
    }

    fn focus_device(&self, main_window: &MainWindow) {
        main_window.invoke_output_focus();
    }

    fn device_setting<'a>(&self, settings: &'a dyn MidiDesktopSettings) -> &'a str {
        self.midi_strategy.device_setting(settings)
    }

    fn set_device_setting(&self, settings: &mut dyn MidiDesktopSettings, device_name: &str) {
        self.midi_strategy.set_device_setting(settings, device_name);
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
        trace!("OutputStrategy.get_selected_device_index: returning selected device index {}", index);
        index
    }

    fn set_selected_device_index(&self, main_window: &MainWindow, index: i32) {
        trace!("OutputStrategy.set_selected_device_index: Setting selected device index to {}", index);
        main_window.set_output_selected_device_index(index);
    }

    fn msg_cannot_connect(&self, device_name: &str) -> String {
        self.midi_strategy.msg_cannot_connect(device_name)
    }

    fn msg_connect(&self) -> &str {
        self.midi_strategy.msg_connect()
    }

    fn msg_connected(&self, device_name: &str) -> String {
        self.midi_strategy.msg_connected(device_name)
    }

    fn msg_refreshed_reconnect(&self) -> &str {
        self.midi_strategy.msg_refreshed_reconnect()
    }

    fn other_device_strategy(&self) -> Box<dyn UiDeviceStrategy> {
        Box::new(UiInputStrategy::new())
    }
}

impl Clone for Box<dyn UiDeviceStrategy> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}