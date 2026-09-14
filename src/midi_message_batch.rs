use midly::{MidiMessage, live::LiveEvent};

/// A batch of MIDI messages that can be accumulated and then released to be sent.
pub struct MidiMessageBatch {
    messages: Vec<Box<[u8]>>,
    pub print_messages_on_adding : bool,
}

impl MidiMessageBatch {
    pub fn new(print_messages_on_adding: bool) -> Self {
        MidiMessageBatch {
            messages: vec!(),
            print_messages_on_adding,
        }
    }

    /// Adds a MIDI control change message to the batch.
    /// Parameter `channel` is 1-based.
    pub fn add_control_change(&mut self, channel: u8, cc_no: u8, value: u8) {
        self.add_channel_message(
            channel,
            MidiMessage::Controller {
                controller: cc_no.into(),
                value: value.into(),
            },
        );
        if self.print_messages_on_adding {
            println!("tx ch{} cc{} {}", channel, cc_no, value);
        }
    }

    /// Adds a MIDI note aftertouch (pressure) message to the batch.
    /// Parameter `channel` is 1-based.
    pub fn add_note_aftertouch(&mut self, channel: u8, key: u8, pressure: u8) {
        self.add_channel_message(
            channel,
            MidiMessage::Aftertouch {
                key: key.into(),
                vel: pressure.into(),
            },
        );
        if self.print_messages_on_adding {
            println!("tx ch{} pPres{} {}", channel, key, pressure);
        }
    }

    /// Returns all messages and clears the batch.
    pub fn release_to_send(&mut self) -> Vec<Box<[u8]>> {
        self.messages.drain(..).collect()
    }

    /// Adds a MIDI channel message to the batch.
    /// Parameter `channel` is 1-based.
    fn add_channel_message(&mut self, channel: u8, message: MidiMessage) {
        let live_event = LiveEvent::Midi {
            channel: (channel - 1).into(), // 0-based channel number.
            message,
        };
        let mut buf = Vec::new();
        live_event.write(&mut buf).unwrap();
        self.add_message(&buf[..]);
    }

    /// Adds a MIDI message to the batch.
    fn add_message(&mut self, message: &[u8]) {
        self.messages.push(message.to_vec().into_boxed_slice());
    }
}