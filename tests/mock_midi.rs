#[path = "mock_io.rs"] pub mod mock_io;

use std::cell::RefCell;
use std::error::Error;
use std::sync::Arc;
use pitchgrid_continuum::i_midi::IMidi;
use pitchgrid_continuum::midi_ports::{IIo};
use pitchgrid_continuum::port_strategy::PortStrategy;
use mock_io::MockIo;
use pitchgrid_continuum::global::PortType;

/// Returns a clone of the current `MidiState`.
pub fn midi_state() -> MidiState {
    MIDI_STATE.with(|s| s.borrow().clone())
}

pub struct MockMidi {
    mock_input: MockIo,
    mock_output: MockIo,
}

impl MockMidi {
    pub fn new(input_device_names: Vec<String>, output_device_names: Vec<String>,
        initial_input_device_name: &str, initial_output_device_name: &str) -> Self {
        MIDI_STATE.replace(MidiState::new());
        let mut input = MockIo::new(PortType::Input, input_device_names);
        input.set_device(initial_input_device_name);
        let mut output = MockIo::new(PortType::Output, output_device_names);
        output.set_device(initial_output_device_name);
        MockMidi {
            mock_input: input,
            mock_output: output,
        }
    }

    pub fn simulate_init_err(msg: &str) {
        MIDI_STATE.with_borrow_mut(|s| s.init_result =
            Err(Arc::new(std::io::Error::new(std::io::ErrorKind::Other, msg))));
    }
}

impl IMidi for MockMidi {
    #[allow(dead_code)]
    fn add_init_download_completed_callback(
        &mut self,
        _callback: Box<dyn Fn() + Send + Sync + 'static>,
    ) {
        MIDI_STATE.with_borrow_mut(|s| {
            s.add_init_download_completed_callback_count += 1;
        });
    }

    #[allow(dead_code)]
    fn add_init_download_started_callback(
        &mut self,
        _callback: Box<dyn Fn() + Send + Sync + 'static>,
    ) {
        MIDI_STATE.with_borrow_mut(|s| {
            s.add_init_download_started_callback_count += 1;
        });
    }

    #[allow(dead_code)]
    fn add_ports_connected_changed_callback(
        &mut self,
        _callback: Box<dyn Fn() + Send + Sync + 'static>,
    ) {
        MIDI_STATE.with_borrow_mut(|s| {
            s.add_ports_connected_changed_callback_count += 1;
        });
    }

    #[allow(dead_code)]
    fn add_new_preset_selected_callback(
        &mut self,
        _callback: Box<dyn Fn() + Send + Sync + 'static>,
    ) {
        MIDI_STATE.with_borrow_mut(|s| {
            s.add_new_preset_selected_callback_count += 1;
        });
    }

    #[allow(dead_code)]
    fn add_receiving_data_started_callback(
        &mut self,
        _callback: Box<dyn Fn() + Send + Sync + 'static>,
    ) {
        MIDI_STATE.with_borrow_mut(|s| {
            s.add_receiving_data_started_callback_count += 1;
        });
    }

    #[allow(dead_code)]
    fn add_receiving_data_stopped_callback(
        &mut self,
        _callback: Box<dyn Fn() + Send + Sync + 'static>,
    ) {
        MIDI_STATE.with_borrow_mut(|s| {
            s.add_receiving_data_stopped_callback_count += 1;
        });
    }

    #[allow(dead_code)]
    fn add_tuning_updated_callback(&mut self, _callback: Box<dyn Fn() + Send + Sync + 'static>) {
        MIDI_STATE.with_borrow_mut(|s| {
            s.add_tuning_updated_callback_count += 1;
        });
    }

    #[allow(dead_code)]
    fn add_updating_tuning_callback(&mut self, _callback: Box<dyn Fn() + Send + Sync + 'static>) {
        MIDI_STATE.with_borrow_mut(|s| {
            s.add_updating_tuning_callback_count += 1;
        });
    }

    #[allow(dead_code)]
    fn are_ports_connected(&self) -> bool {
        MIDI_STATE.with(|s| s.borrow().are_ports_connected)
    }

    #[allow(dead_code)]
    fn close(&mut self) {
        MIDI_STATE.with_borrow_mut(|s| {
            s.close_count += 1;
        });
    }

    #[allow(dead_code)]
    fn connect_port(
        &mut self,
        index: usize,
        port_strategy: &dyn PortStrategy,
    ) -> Result<(), Box<dyn Error>> {
        MIDI_STATE.with_borrow_mut(|s| {
            s.connect_port_count += 1;
            s.connect_port_index = Some(index);
            s.connect_port_port_strategy = Some(port_strategy.clone_box());
            match port_strategy.port_type() {
                PortType::Input => {
                    if s.is_output_port_connected {
                        s.are_ports_connected = true;
                    }
                }
                PortType::Output => {
                    s.is_output_port_connected = true;
                }
            }
        });
        Ok(())
    }

    #[allow(dead_code)]
    fn init(
        &mut self,
        input_device_name: &str,
        output_device_name: &str,
    ) -> Result<(), Box<dyn Error>> {
        match MIDI_STATE.with(|s| s.borrow().init_result.clone()) {
            Ok(()) => {
                MIDI_STATE.with_borrow_mut(|s| {
                    s.init_input_device_name = Some(input_device_name.to_string());
                    s.init_output_device_name = Some(output_device_name.to_string());
                });
                Ok(())
            }
            Err(e) => Err(e.to_string().into()),
        }
    }

    #[allow(dead_code)]
    fn input(&self) -> &dyn IIo {
        &self.mock_input
    }

    #[allow(dead_code)]
    fn io(&self, port_strategy: &dyn PortStrategy) -> &dyn IIo {
        MIDI_STATE.with_borrow_mut(|s| {
            s.io_count += 1;
            s.io_port_strategy = Some(port_strategy.clone_box());
        });
        port_strategy.io(self)
    }

    #[allow(dead_code)]
    fn has_downloaded_init_data(&self) -> bool {
        MIDI_STATE.with_borrow_mut(|s| {
            s.has_downloaded_init_data_count += 1;
        });
        MIDI_STATE.with(|s| s.borrow().has_downloaded_init_data_result)
    }

    #[allow(dead_code)]
    fn is_output_port_connected(&self) -> bool {
        MIDI_STATE.with(|s| s.borrow().is_output_port_connected)
    }

    #[allow(dead_code)]
    fn is_receiving_data(&self) -> bool {
        MIDI_STATE.with(|s| s.borrow().is_receiving_data)
    }

    #[allow(dead_code)]
    fn output(&self) -> &dyn IIo {
        &self.mock_output
    }

    #[allow(dead_code)]
    fn refresh_devices(
        &mut self,
        device_name: &str,
        port_strategy: &dyn PortStrategy,
    ) -> Result<(), Box<dyn Error>> {
        MIDI_STATE.with_borrow_mut(|s| {
            s.refresh_devices_count += 1;
            s.refresh_devices_device_name = Some(device_name.to_string());
            s.refresh_devices_port_strategy = Some(port_strategy.clone_box());
            s.are_ports_connected = false;
            if *port_strategy.port_type() == PortType::Output {
                s.is_output_port_connected = false;
            }
        });
        Ok(())
    }

    #[allow(dead_code)]
    fn start_instrument_connection_monitor(&mut self) {
        MIDI_STATE.with_borrow_mut(|s| {
            s.start_instrument_connection_monitor_count += 1;
        });
    }

    #[allow(dead_code)]
    fn stop_instrument_connection_monitor(&mut self) {
        MIDI_STATE.with_borrow_mut(|s| {
            s.stop_instrument_connection_monitor_count += 1;
        });
    }
}

pub struct MidiState {
    pub add_init_download_completed_callback_count: u16,
    pub add_init_download_started_callback_count: u16,
    pub add_ports_connected_changed_callback_count: u16,
    pub add_new_preset_selected_callback_count: u16,
    pub add_receiving_data_started_callback_count: u16,
    pub add_receiving_data_stopped_callback_count: u16,
    pub add_tuning_updated_callback_count: u16,
    pub add_updating_tuning_callback_count: u16,

    pub are_ports_connected: bool,

    pub close_count: u16,

    pub connect_port_count: u16,
    pub connect_port_index: Option<usize>,
    pub connect_port_port_strategy: Option<Box<dyn PortStrategy>>,

    pub has_downloaded_init_data_count: u16,
    pub has_downloaded_init_data_result: bool,

    pub init_result: Result<(), Arc<dyn Error>>,
    pub init_input_device_name: Option<String>,
    pub init_output_device_name: Option<String>,

    pub io_count: u16,
    pub io_port_strategy: Option<Box<dyn PortStrategy>>,

    pub is_output_port_connected: bool,
    pub is_receiving_data: bool,

    pub refresh_devices_count: u16,
    pub refresh_devices_device_name: Option<String>,
    pub refresh_devices_port_strategy: Option<Box<dyn PortStrategy>>,

    pub start_instrument_connection_monitor_count: u16,
    pub stop_instrument_connection_monitor_count: u16,
}

impl MidiState {
    pub fn new() -> Self {
        MidiState {
            add_init_download_completed_callback_count: 0,
            add_init_download_started_callback_count: 0,
            add_ports_connected_changed_callback_count: 0,
            add_new_preset_selected_callback_count: 0,
            add_receiving_data_started_callback_count: 0,
            add_receiving_data_stopped_callback_count: 0,
            add_tuning_updated_callback_count: 0,
            add_updating_tuning_callback_count: 0,

            are_ports_connected: false,

            close_count: 0,

            connect_port_count: 0,
            connect_port_index: None,
            connect_port_port_strategy: None,

            has_downloaded_init_data_count: 0,
            has_downloaded_init_data_result: false,

            init_result: Ok(()),
            init_input_device_name: None,
            init_output_device_name: None,

            io_count: 0,
            io_port_strategy: None,

            is_output_port_connected: false,
            is_receiving_data: false,

            refresh_devices_count: 0,
            refresh_devices_device_name: None,
            refresh_devices_port_strategy: None,

            start_instrument_connection_monitor_count: 0,
            stop_instrument_connection_monitor_count: 0,
        }
    }
}

impl Clone for MidiState {
    fn clone(&self) -> Self {
        MidiState {
            add_init_download_completed_callback_count: self.add_init_download_completed_callback_count,
            add_init_download_started_callback_count: self.add_init_download_started_callback_count,
            add_ports_connected_changed_callback_count: self.add_ports_connected_changed_callback_count,
            add_new_preset_selected_callback_count: self.add_new_preset_selected_callback_count,
            add_receiving_data_started_callback_count: self.add_receiving_data_started_callback_count,
            add_receiving_data_stopped_callback_count: self.add_receiving_data_stopped_callback_count,
            add_tuning_updated_callback_count: self.add_tuning_updated_callback_count,
            add_updating_tuning_callback_count: self.add_updating_tuning_callback_count,

            are_ports_connected: self.are_ports_connected,

            close_count: self.close_count,

            connect_port_count: self.connect_port_count,
            connect_port_index: self.connect_port_index,
            connect_port_port_strategy: self.connect_port_port_strategy.as_ref().map(|s| s.clone_box()),

            has_downloaded_init_data_count: self.has_downloaded_init_data_count,
            has_downloaded_init_data_result: self.has_downloaded_init_data_result,

            init_result: self.init_result.clone(),
            init_input_device_name: self.init_input_device_name.clone(),
            init_output_device_name: self.init_output_device_name.clone(),

            io_count: self.io_count,
            io_port_strategy: self.io_port_strategy.as_ref().map(|s| s.clone_box()),

            is_output_port_connected: self.is_output_port_connected,
            is_receiving_data: self.is_receiving_data,

            refresh_devices_count: self.refresh_devices_count,
            refresh_devices_device_name: self.refresh_devices_device_name.clone(),
            refresh_devices_port_strategy: self.refresh_devices_port_strategy.as_ref().map(|s| s.clone_box()),

            start_instrument_connection_monitor_count: self.start_instrument_connection_monitor_count,
            stop_instrument_connection_monitor_count: self.stop_instrument_connection_monitor_count,
        }
    }
}

thread_local! {
    static MIDI_STATE: RefCell<MidiState> = RefCell::new(MidiState::new());
}
