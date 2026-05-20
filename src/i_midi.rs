use std::error::Error;
use std::sync::{Arc, Mutex};
use crate::midi_ports::IIo;
use crate::port_strategy::PortStrategy;

/// A trait that defines the interface for managing MIDI devices and messages.
///
/// For the The `I` prefix, see `ITuner`s doc comment.
pub trait IMidi {
    fn add_init_download_completed_callback(
        &mut self,
        callback: Box<dyn Fn() + Send + Sync + 'static>,
    );

    fn add_init_download_started_callback(
        &mut self,
        callback: Box<dyn Fn() + Send + Sync + 'static>,
    );

    fn add_ports_connected_changed_callback(
        &mut self,
        callback: Box<dyn Fn() + Send + Sync + 'static>,
    );

    fn add_new_preset_selected_callback(
        &mut self,
        callback: Box<dyn Fn() + Send + Sync + 'static>,
    );

    fn add_receiving_data_started_callback(
        &mut self,
        callback: Box<dyn Fn() + Send + Sync + 'static>,
    );

    fn add_receiving_data_stopped_callback(
        &mut self,
        callback: Box<dyn Fn() + Send + Sync + 'static>,
    );

    fn add_tuning_updated_callback(&mut self, callback: Box<dyn Fn() + Send + Sync + 'static>);

    fn add_updating_tuning_callback(&mut self, callback: Box<dyn Fn() + Send + Sync + 'static>);

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
