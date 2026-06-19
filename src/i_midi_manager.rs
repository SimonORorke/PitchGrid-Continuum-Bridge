use std::error::Error;
use std::sync::{Arc, Mutex};
use crate::midi_ports::IIo;
use crate::device_strategy::DeviceStrategy;

pub trait MidiCallbacks: Send + Sync {
    fn on_download_completed(&self);
    fn on_download_started(&self);
    fn on_new_preset_selected(&self);
    fn on_devices_connected_changed(&self);
    fn on_receiving_data_started(&self);
    fn on_receiving_data_stopped(&self);
    fn on_tuning_updated(&self);
    fn on_updating_tuning(&self);
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
        callbacks: Arc<dyn MidiCallbacks>,
    );

    fn input(&self) -> &dyn IIo;

    fn io(&self, device_strategy: &dyn DeviceStrategy) -> &dyn IIo;

    fn has_downloaded_init_data(&self) -> bool;

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
/// disconnects it) and the `MidiSender` (which writes to it). Replaces the former
/// `OUTPUT_CONNECTION` global.
pub type SharedOutput = Arc<Mutex<Option<midir::MidiOutputConnection>>>;

/// The seam by which the `Tuner` tells the protocol layer that it is about to send a tuning update,
/// so that layer can mark a tuning as in-flight (and notify the UI). Implemented in 3b by
/// `MidiState`; 3c will move the implementation to the `ContinuumProtocol`.
pub trait TuningUpdateSignaller: Send + Sync {
    fn on_updating_tuning(&self);
}

/// A no-op `TuningUpdateSignaller`, the `Tuner`'s default until the real one is wired in (see
/// `Controller::new`). Mirrors `NullMidiSender`; keeps the standalone `Tuner` tests free of any
/// MIDI/protocol wiring.
pub struct NullTuningSignaller;

impl TuningUpdateSignaller for NullTuningSignaller {
    fn on_updating_tuning(&self) {}
}
