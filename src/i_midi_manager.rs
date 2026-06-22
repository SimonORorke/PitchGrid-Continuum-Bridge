use std::error::Error;
use std::sync::{Arc, Mutex};
use crate::midi_ports::IIo;
use crate::device_strategy::DeviceStrategy;

/// The raw seam by which the generic `MidiManager` reports inbound MIDI and connection-lifecycle
/// events to whatever interprets them — here, the `ContinuumProtocol`. The manager raises raw
/// messages and generic lifecycle events and has no knowledge of the Continuum protocol; this is
/// the library boundary, where a different application would supply a different listener.
pub trait MidiInputListener: Send + Sync {
    /// A raw inbound MIDI message.
    fn on_message(&self, message: &[u8]);
    /// The first message has arrived since monitoring started.
    fn on_receiving_data_started(&self);
    /// Data has stopped arriving (or never started).
    fn on_receiving_data_stopped(&self);
    /// Both input and output devices have just become connected, or one has dropped.
    fn on_devices_connected_changed(&self);
}

/// A trait that defines the interface for managing MIDI devices and messages.
///
/// For the `I` prefix, see `ITuner`s doc comment.
pub trait IMidiManager {
    fn are_devices_connected(&self) -> bool;

    fn close(&mut self);

    fn connect_device(
        &mut self,
        index: usize,
        device_strategy: &dyn DeviceStrategy,
    ) -> Result<(), Box<dyn Error>>;

    fn init(
        &mut self,
        input_device_name: &str,
        output_device_name: &str,
    );

    fn input(&self) -> &dyn IIo;

    fn io(&self, device_strategy: &dyn DeviceStrategy) -> &dyn IIo;

    /// Whether both MIDI devices are connected *and* data is being received from the instrument.
    /// Provided so a caller holding the manager lock can test both conditions with a single lock
    /// rather than locking the manager twice. Has a default impl; implementors need not override it.
    fn is_connected_and_receiving(&self) -> bool {
        self.are_devices_connected() && self.is_receiving_data()
    }

    fn is_output_device_connected(&self) -> bool;

    fn is_receiving_data(&self) -> bool;

    fn output(&self) -> &dyn IIo;

    fn refresh_devices(
        &mut self,
        device_name: &str,
        device_strategy: &dyn DeviceStrategy,
    );

    fn start_instrument_connection_monitor(&mut self);

    fn stop_instrument_connection_monitor(&mut self);
}

pub type SharedMidiManager = Arc<Mutex<Box<dyn IMidiManager + Send>>>;

/// The instrument's MIDI output connection, shared between the `MidiManager` (which connects and
/// disconnects it) and the `MidiSender` (which writes to it).
pub type SharedOutput = Arc<Mutex<Option<midir::MidiOutputConnection>>>;
