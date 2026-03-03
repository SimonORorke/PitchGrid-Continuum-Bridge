use slint::ModelRc;
use crate::{MainWindow, ComboBoxItem, MessageType};
use crate::midi::{ConnectionTo, Midi, PortType};
use crate::midi_ports::MidiIo;
use crate::settings::Settings;

pub trait PortStrategy: Send + Sync {
    fn connection_to(&self) -> &ConnectionTo;
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
        &self, main_window: &MainWindow, port_name: &str, message_type: MessageType);
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
pub struct EditorInputStrategy {
}

impl EditorInputStrategy {
    pub fn new() -> Self {
        Self {}
    }
}

impl PortStrategy for EditorInputStrategy {
    fn connection_to(&self) -> &ConnectionTo {
        &ConnectionTo::Editor
    }

    fn port_type(&self) -> &PortType {
        &PortType::Input
    }

    fn io<'a>(&self, midi: &'a Midi) -> &'a dyn MidiIo {
        midi.editor_input()
    }

    fn clone_box(&self) -> Box<dyn PortStrategy> {
        Box::new(self.clone())
    }

    fn focus_port(&self, main_window: &MainWindow) {
        main_window.invoke_editor_input_focus();
    }

    fn port_setting<'a>(&self, settings: &'a Settings) -> &'a str {
        &settings.editor_midi_input_port
    }

    fn set_port_setting(&self, settings: &mut Settings, port_name: &str) {
        settings.editor_midi_input_port = port_name.into();
    }

    fn show_connected_port_name(&self, main_window: &MainWindow, port_name: &str,
                                message_type: MessageType) {
        main_window.invoke_editor_input_show_connected_port_name(port_name.into(), message_type);
    }

    fn set_ports_model(&self, main_window: &MainWindow, model: ModelRc<ComboBoxItem>) {
        main_window.set_editor_input_ports_model(model);
    }

    fn get_selected_port_index(&self, main_window: &MainWindow) -> i32 {
        main_window.get_editor_input_selected_port_index()
    }

    fn set_selected_port_index(&self, main_window: &MainWindow, index: i32) {
        main_window.set_editor_input_selected_port_index(index);
    }

    fn msg_cannot_connect(&self, port_name: &str) -> &str {
        Box::leak(format!("Cannot connect editor MIDI input port {}. The port may be in use.",
                          port_name).into_boxed_str())
    }

    fn msg_connect(&self) -> &str {
        "Connect editor MIDI input port"
    }

    fn msg_connected(&self, port_name: &str) -> &str {
        Box::leak(format!("Connected editor MIDI input port {}", port_name).into_boxed_str())
    }

    fn msg_not_selected(&self) -> &str {
        "No editor MIDI input port selected."
    }

    fn msg_refreshed_reconnect(&self) -> &str {
        "Refreshed editor MIDI input ports. You must (re)connect."
    }
}

#[derive(Clone)]
pub struct EditorOutputStrategy {
}

impl EditorOutputStrategy {
    pub fn new() -> Self {
        Self {}
    }
}

impl PortStrategy for EditorOutputStrategy {
    fn connection_to(&self) -> &ConnectionTo {
        &ConnectionTo::Editor
    }

    fn port_type(&self) -> &PortType {
        &PortType::Output
    }

    fn io<'a>(&self, midi: &'a Midi) -> &'a dyn MidiIo {
        midi.editor_output()
    }

    fn clone_box(&self) -> Box<dyn PortStrategy> {
        Box::new(self.clone())
    }

    fn focus_port(&self, main_window: &MainWindow) {
        main_window.invoke_editor_output_focus();
    }

    fn port_setting<'a>(&self, settings: &'a Settings) -> &'a str {
        &settings.editor_midi_output_port
    }

    fn set_port_setting(&self, settings: &mut Settings, port_name: &str) {
        settings.editor_midi_output_port = port_name.into();
    }

    fn show_connected_port_name(&self, main_window: &MainWindow, port_name: &str,
                                message_type: MessageType) {
        main_window.invoke_editor_output_show_connected_port_name(port_name.into(), message_type);
    }

    fn set_ports_model(&self, main_window: &MainWindow, model: ModelRc<ComboBoxItem>) {
        main_window.set_editor_output_ports_model(model);
    }

    fn get_selected_port_index(&self, main_window: &MainWindow) -> i32 {
        main_window.get_editor_output_selected_port_index()
    }

    fn set_selected_port_index(&self, main_window: &MainWindow, index: i32) {
        main_window.set_editor_output_selected_port_index(index);
    }

    fn msg_cannot_connect(&self, port_name: &str) -> &str {
        Box::leak(format!("Cannot connect editor MIDI output port {}. The port may be in use.",
                          port_name).into_boxed_str())
    }

    fn msg_connect(&self) -> &str {
        "Connect editor MIDI output port"
    }

    fn msg_connected(&self, port_name: &str) -> &str {
        Box::leak(format!("Connected editor MIDI output port {}", port_name).into_boxed_str())
    }

    fn msg_not_selected(&self) -> &str {
        "No editor MIDI output port selected."
    }

    fn msg_refreshed_reconnect(&self) -> &str {
        "Refreshed editor MIDI output ports. You must (re)connect."
    }
}

#[derive(Clone)]
pub struct InstrumentInputStrategy {
}

impl InstrumentInputStrategy {
    pub fn new() -> Self {
        Self {}
    }
}

impl PortStrategy for InstrumentInputStrategy {
    fn connection_to(&self) -> &ConnectionTo {
        &ConnectionTo::Instru
    }

    fn port_type(&self) -> &PortType {
        &PortType::Input
    }

    fn io<'a>(&self, midi: &'a Midi) -> &'a dyn MidiIo {
        midi.instru_input()
    }

    fn clone_box(&self) -> Box<dyn PortStrategy> {
        Box::new(self.clone())
    }

    fn focus_port(&self, main_window: &MainWindow) {
        main_window.invoke_instru_input_focus();
    }

    fn port_setting<'a>(&self, settings: &'a Settings) -> &'a str {
        &settings.instrument_midi_input_port
    }

    fn set_port_setting(&self, settings: &mut Settings, port_name: &str) {
        settings.instrument_midi_input_port = port_name.into();
    }

    fn show_connected_port_name(&self, main_window: &MainWindow, port_name: &str,
                                message_type: MessageType) {
        main_window.invoke_instru_input_show_connected_port_name(port_name.into(), message_type);
    }

    fn set_ports_model(&self, main_window: &MainWindow, model: ModelRc<ComboBoxItem>) {
        main_window.set_instru_input_ports_model(model);
    }

    fn get_selected_port_index(&self, main_window: &MainWindow) -> i32 {
        main_window.get_instru_input_selected_port_index()
    }

    fn set_selected_port_index(&self, main_window: &MainWindow, index: i32) {
        main_window.set_instru_input_selected_port_index(index);
    }

    fn msg_cannot_connect(&self, port_name: &str) -> &str {
        Box::leak(format!("Cannot connect instrument MIDI input port {}. The port may be in use.",
                          port_name).into_boxed_str())
    }

    fn msg_connect(&self) -> &str {
        "Connect instrument MIDI input port"
    }

    fn msg_connected(&self, port_name: &str) -> &str {
        Box::leak(format!("Connected instrument MIDI input port {}", port_name).into_boxed_str())
    }

    fn msg_not_selected(&self) -> &str {
        "No instrument MIDI input port selected."
    }

    fn msg_refreshed_reconnect(&self) -> &str {
        "Refreshed instrument MIDI input ports. You must (re)connect."
    }
}

#[derive(Clone)]
pub struct InstrumentOutputStrategy {
}

impl InstrumentOutputStrategy {
    pub fn new() -> Self {
        Self {}
    }
}

impl PortStrategy for InstrumentOutputStrategy {
    fn connection_to(&self) -> &ConnectionTo {
        &ConnectionTo::Instru
    }

    fn port_type(&self) -> &PortType {
        &PortType::Output
    }

    fn io<'a>(&self, midi: &'a Midi) -> &'a dyn MidiIo {
        midi.instru_output()
    }

    fn clone_box(&self) -> Box<dyn PortStrategy> {
        Box::new(self.clone())
    }

    fn focus_port(&self, main_window: &MainWindow) {
        main_window.invoke_instru_output_focus();
    }

    fn port_setting<'a>(&self, settings: &'a Settings) -> &'a str {
        &settings.instrument_midi_output_port
    }

    fn set_port_setting(&self, settings: &mut Settings, port_name: &str) {
        settings.instrument_midi_output_port = port_name.into();
    }

    fn show_connected_port_name(&self, main_window: &MainWindow, port_name: &str,
                                message_type: MessageType) {
        main_window.invoke_instru_output_show_connected_port_name(port_name.into(), message_type);
    }

    fn set_ports_model(&self, main_window: &MainWindow, model: ModelRc<ComboBoxItem>) {
        main_window.set_instru_output_ports_model(model);
    }

    fn get_selected_port_index(&self, main_window: &MainWindow) -> i32 {
        main_window.get_instru_output_selected_port_index()
    }

    fn set_selected_port_index(&self, main_window: &MainWindow, index: i32) {
        main_window.set_instru_output_selected_port_index(index);
    }

    fn msg_cannot_connect(&self, port_name: &str) -> &str {
        Box::leak(format!("Cannot connect instrument MIDI output port {}. The port may be in use.",
                          port_name).into_boxed_str())
    }

    fn msg_connect(&self) -> &str {
        "Connect instrument MIDI output port"
    }

    fn msg_connected(&self, port_name: &str) -> &str {
        Box::leak(format!("Connected instrument MIDI output port {}", port_name).into_boxed_str())
    }

    fn msg_not_selected(&self) -> &str {
        "No instrument MIDI output port selected."
    }

    fn msg_refreshed_reconnect(&self) -> &str {
        "Refreshed instrument MIDI output ports. You must (re)connect."
    }
}
