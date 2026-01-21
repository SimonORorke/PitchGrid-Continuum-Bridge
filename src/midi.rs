use midir::{MidiOutput, MidiOutputConnection, MidiOutputPort};

pub struct MidiManager {
    midi_output: MidiOutput,
    midi_output_connection: Option<MidiOutputConnection>,
    output_port_index: usize,
    output_ports: Vec<MidiOutputPort>,
}

impl MidiManager {
    pub fn new() -> Self {
        Self {
            midi_output: MidiOutput::new("My MIDI Output").unwrap(),
            midi_output_connection: None,
            output_port_index: 0,
            output_ports: Vec::new(),
        }
    }

    pub fn get_output_port_names(&mut self) -> Vec<String> {
        self.output_ports = self.midi_output.ports().to_vec();
        let output_port_names: Vec<String> = self.output_ports.iter()
            .map(|port| 
                self.midi_output.port_name(&port).unwrap()).collect();
        output_port_names
    }
}
