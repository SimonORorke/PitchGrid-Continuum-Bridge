use midir::{MidiOutput, MidiOutputConnection, MidiOutputPort, MidiOutputPorts};

pub struct MidiManager {
    midi_output: MidiOutput,
    midi_output_connection: Option<MidiOutputConnection>,
    output_port_index: usize,
}

impl MidiManager {
    pub fn new() -> Self {
        Self {
            midi_output: MidiOutput::new("My MIDI Output").unwrap(),
            midi_output_connection: None,
            output_port_index: 0,
        }
    }

    pub fn get_midi_output_names(&self) -> Vec<String> {
        let ports = self.midi_output.ports();
        let mut names = Vec::<String>::new();
        for port in ports {
            names.push(self.midi_output.port_name(&port).unwrap());
        }
        names
    }
}
