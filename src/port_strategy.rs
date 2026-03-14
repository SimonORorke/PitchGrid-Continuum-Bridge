use slint::ModelRc;
use crate::{MainWindow, ComboBoxItem, SlintMessageType};
use crate::global::PortType;
use crate::midi::Midi;
use crate::midi_ports::MidiIo;
use crate::settings::Settings;

/// This trait is used to implement strategies that depend on whether a MIDI port is input or
/// output.
/// In the Model-View-Controller (MVC) pattern, it contains both view and controller methods.
/// See Controller's doc comment for more information on how the project implements MVC.
pub trait PortStrategy: Send + Sync {
    fn port_type(&self) -> &PortType;
    fn io<'a>(&self, midi: &'a Midi) -> &'a dyn MidiIo;

    /// Makes a clone of the current strategy that needs to be used when cross-threading.
    /// The code is the same for all strategies. But the compiler does not allow it to be
    /// implemented here as a default method.
    fn clone_box(&self) -> Box<dyn PortStrategy>;

    fn focus_port(&self, main_window: &MainWindow);
    fn port_setting<'a>(&self, settings: &'a Settings) -> &'a str;
    fn set_port_setting(&self, settings: &mut Settings, port_name: &str);
    fn show_connected_port_name(
        &self, main_window: &MainWindow, port_name: &str, message_type: SlintMessageType);
    fn set_ports_model(&self, main_window: &MainWindow, model: ModelRc<ComboBoxItem>);
    fn get_selected_port_index(&self, main_window: &MainWindow) -> i32;
    fn set_selected_port_index(&self, main_window: &MainWindow, index: i32);
    fn msg_cannot_connect(&self, port_name: &str) -> &str;
    fn msg_connect(&self) -> &str;
    fn msg_connected(&self, port_name: &str) -> &str;
    fn msg_not_selected(&self) -> &str;
    fn msg_refreshed_reconnect(&self) -> &str;
}

#[derive(Clone)]
pub struct InputStrategy {
}

impl InputStrategy {
    pub fn new() -> Self {
        Self {}
    }
}

impl PortStrategy for InputStrategy {
    fn port_type(&self) -> &PortType {
        &PortType::Input
    }

    fn io<'a>(&self, midi: &'a Midi) -> &'a dyn MidiIo {
        midi.input()
    }

    fn clone_box(&self) -> Box<dyn PortStrategy> {
        Box::new(self.clone())
    }

    fn focus_port(&self, main_window: &MainWindow) {
        main_window.invoke_input_focus();
    }

    fn port_setting<'a>(&self, settings: &'a Settings) -> &'a str {
        &settings.midi_input_port
    }

    fn set_port_setting(&self, settings: &mut Settings, port_name: &str) {
        settings.midi_input_port = port_name.into();
    }

    fn show_connected_port_name(&self, main_window: &MainWindow, port_name: &str,
                                message_type: SlintMessageType) {
        main_window.invoke_input_show_connected_port_name(port_name.into(), message_type);
    }

    fn set_ports_model(&self, main_window: &MainWindow, model: ModelRc<ComboBoxItem>) {
        main_window.set_input_ports_model(model);
    }

    fn get_selected_port_index(&self, main_window: &MainWindow) -> i32 {
        main_window.get_input_selected_port_index()
    }

    fn set_selected_port_index(&self, main_window: &MainWindow, index: i32) {
        main_window.set_input_selected_port_index(index);
    }

    fn msg_cannot_connect(&self, port_name: &str) -> &str {
        Box::leak(format!("Cannot connect MIDI input port {}. The port may be in use.",
                          port_name).into_boxed_str())
    }

    fn msg_connect(&self) -> &str {
        "Connect MIDI input port"
    }

    fn msg_connected(&self, port_name: &str) -> &str {
        Box::leak(format!("Connected MIDI input port {}", port_name).into_boxed_str())
    }

    fn msg_not_selected(&self) -> &str {
        "No MIDI input port selected."
    }

    fn msg_refreshed_reconnect(&self) -> &str {
        "Refreshed MIDI input ports. You must (re)connect."
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

impl PortStrategy for OutputStrategy {
    fn port_type(&self) -> &PortType {
        &PortType::Output
    }

    fn io<'a>(&self, midi: &'a Midi) -> &'a dyn MidiIo {
        midi.output()
    }

    fn clone_box(&self) -> Box<dyn PortStrategy> {
        Box::new(self.clone())
    }

    fn focus_port(&self, main_window: &MainWindow) {
        main_window.invoke_output_focus();
    }

    fn port_setting<'a>(&self, settings: &'a Settings) -> &'a str {
        &settings.midi_output_port
    }

    fn set_port_setting(&self, settings: &mut Settings, port_name: &str) {
        settings.midi_output_port = port_name.into();
    }

    fn show_connected_port_name(&self, main_window: &MainWindow, port_name: &str,
                                message_type: SlintMessageType) {
        main_window.invoke_output_show_connected_port_name(port_name.into(), message_type);
    }

    fn set_ports_model(&self, main_window: &MainWindow, model: ModelRc<ComboBoxItem>) {
        main_window.set_output_ports_model(model);
    }

    fn get_selected_port_index(&self, main_window: &MainWindow) -> i32 {
        main_window.get_output_selected_port_index()
    }

    fn set_selected_port_index(&self, main_window: &MainWindow, index: i32) {
        main_window.set_output_selected_port_index(index);
    }

    fn msg_cannot_connect(&self, port_name: &str) -> &str {
        Box::leak(format!("Cannot connect MIDI output port {}. The port may be in use.",
                          port_name).into_boxed_str())
    }

    fn msg_connect(&self) -> &str {
        "Connect MIDI output port"
    }

    fn msg_connected(&self, port_name: &str) -> &str {
        Box::leak(format!("Connected MIDI output port {}", port_name).into_boxed_str())
    }

    fn msg_not_selected(&self) -> &str {
        "No MIDI output port selected."
    }

    fn msg_refreshed_reconnect(&self) -> &str {
        "Refreshed MIDI output ports. You must (re)connect."
    }
}
