use std::error::Error;
use std::sync::{Arc, Mutex};
use crate::midi_ports::IIo;
use crate::port_strategy::PortStrategy;

pub trait MidiCallbacks: Send + Sync {
    fn on_download_completed(&self);
    fn on_download_started(&self);
    fn on_new_preset_selected(&self);
    fn on_ports_connected_changed(&self);
    fn on_receiving_data_started(&self);
    fn on_receiving_data_stopped(&self);
    fn on_tuning_updated(&self);
    fn on_updating_tuning(&self);
}

/// A trait that defines the interface for managing MIDI devices and messages.
///
/// For the The `I` prefix, see `ITuner`s doc comment.
pub trait IMidi {
    fn are_ports_connected(&self) -> bool;

    fn close(&mut self);

    fn connect_port(
        &mut self,
        index: usize,
        port_strategy: &dyn PortStrategy,
    ) -> Result<(), Box<dyn Error>>;

    fn init(
        &mut self,
        input_device_name: &str,
        output_device_name: &str,
        callbacks: Arc<dyn MidiCallbacks>,
    ) -> Result<(), Box<dyn Error>>;

    fn input(&self) -> &dyn IIo;

    fn io(&self, port_strategy: &dyn PortStrategy) -> &dyn IIo;

    fn has_downloaded_init_data(&self) -> bool;

    fn is_output_port_connected(&self) -> bool;

    fn is_receiving_data(&self) -> bool;

    fn output(&self) -> &dyn IIo;

    fn refresh_devices(
        &mut self,
        device_name: &str,
        port_strategy: &dyn PortStrategy,
    ) -> Result<(), Box<dyn Error>>;

    fn start_instrument_connection_monitor(&mut self);

    fn stop_instrument_connection_monitor(&mut self);
}

pub type SharedMidi = Arc<Mutex<Box<dyn IMidi + Send>>>;
