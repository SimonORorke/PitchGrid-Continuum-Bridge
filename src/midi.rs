use midir::{MidiOutput, MidiOutputConnection, MidiOutputPort};

pub struct MidiManager {
    output_connection: Option<MidiOutputConnection>,
    output_ports: Vec<MidiOutputPort>,
}

impl MidiManager {
    pub fn new() -> Self {
        Self {
            output_connection: None,
            output_ports: Vec::new(),
        }
    }

    pub fn connect_to_output_port(&mut self, index: usize) {
        self.disconnect_from_output_port();
        if let Some(port) = self.output_ports.get(index) {
            let midi_output = MidiOutput::new("My MIDI Output").unwrap();
            let port_name = midi_output.port_name(&port).unwrap();
            self.output_connection = midi_output.connect(port, &port_name).ok();
        }
    }

    fn disconnect_from_output_port(&mut self) {
        if let Some(connection) = self.output_connection.take() {
            connection.close();
        }
    }

    pub fn get_output_port_names(&mut self) -> Vec<String> {
        self.disconnect_from_output_port();
        let midi_output = MidiOutput::new("My MIDI Output").unwrap();
        self.output_ports = midi_output.ports().to_vec();
        let output_port_names: Vec<String> = self.output_ports.iter()
            .map(|port|
                midi_output.port_name(&port).unwrap()).collect();
        output_port_names
    }
}
