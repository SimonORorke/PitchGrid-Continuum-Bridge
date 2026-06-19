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
/// For the The `I` prefix, see `ITuner`s doc comment.
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
