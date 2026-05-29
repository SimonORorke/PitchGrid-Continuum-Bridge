#[path = "mock_io.rs"] pub mod mock_io;

use std::cell::RefCell;
use std::error::Error;
use std::sync::Arc;
use pitchgrid_continuum::i_midi::{IMidi, MidiCallbacks};
use pitchgrid_continuum::midi_ports::{IIo};
use pitchgrid_continuum::device_strategy::DeviceStrategy;
use mock_io::MockIo;
use pitchgrid_continuum::global::DeviceType;

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
        let mut input = MockIo::new(DeviceType::Input, input_device_names);
        input.set_device(initial_input_device_name);
        let mut output = MockIo::new(DeviceType::Output, output_device_names);
        output.set_device(initial_output_device_name);
        MockMidi {
            mock_input: input,
            mock_output: output,
        }
    }

    pub fn set_are_devices_connected(value: bool) {
        MIDI_STATE.with_borrow_mut(|s| s.are_devices_connected = value);
    }

    pub fn set_is_receiving_data(value: bool) {
        MIDI_STATE.with_borrow_mut(|s| s.is_receiving_data = value);
    }

    pub fn set_simulate_devices_connected_changed(value: bool) {
        MIDI_STATE.with_borrow_mut(|s| s.simulate_devices_connected_changed = value);
    }

    pub fn simulate_init_err(msg: &str) {
        MIDI_STATE.with_borrow_mut(|s| s.init_result =
            Err(Arc::new(std::io::Error::new(std::io::ErrorKind::Other, msg))));
    }

    pub fn simulate_download_completed() {
        let callbacks = MIDI_STATE.with_borrow_mut(|s| {
            s.has_downloaded_init_data_result = true;
            s.callbacks.clone().unwrap()
        });
        callbacks.on_download_completed();
    }

    pub fn simulate_download_started() {
        MIDI_STATE.with(|s| s.borrow().callbacks.as_ref().unwrap().on_download_started());
    }

    pub fn simulate_new_preset_selected() {
        MIDI_STATE.with(|s| s.borrow().callbacks.as_ref().unwrap().on_new_preset_selected());
    }

    pub fn simulate_receiving_data_started() {
        MIDI_STATE.with(|s| s.borrow().callbacks.as_ref().unwrap().on_receiving_data_started());
    }

    #[allow(dead_code)]
    pub fn simulate_receiving_data_stopped() {
        MIDI_STATE.with(|s| s.borrow().callbacks.as_ref().unwrap().on_receiving_data_stopped());
    }

    pub fn simulate_tuning_updated() {
        MIDI_STATE.with(|s| s.borrow().callbacks.as_ref().unwrap().on_tuning_updated());
    }

    pub fn simulate_updating_tuning() {
        MIDI_STATE.with(|s| s.borrow().callbacks.as_ref().unwrap().on_updating_tuning());
    }
}

impl IMidi for MockMidi {
    fn are_devices_connected(&self) -> bool {
        MIDI_STATE.with(|s| s.borrow().are_devices_connected)
    }

    fn close(&mut self) {
        MIDI_STATE.with_borrow_mut(|s| {
            s.close_count += 1;
        });
    }

    fn connect_device(
        &mut self,
        index: usize,
        device_strategy: &dyn DeviceStrategy,
    ) -> Result<(), Box<dyn Error>> {
        MIDI_STATE.with_borrow_mut(|s| {
            s.connect_device_count += 1;
            s.connect_device_index = Some(index);
            s.connect_device_device_strategy = Some(device_strategy.clone_box());
            match device_strategy.device_type() {
                DeviceType::Input => {
                    if s.is_output_device_connected {
                        s.are_devices_connected = true;
                    }
                }
                DeviceType::Output => {
                    s.is_output_device_connected = true;
                }
            }
            if s.simulate_devices_connected_changed {
                s.callbacks.as_ref().unwrap().on_devices_connected_changed();
            }
        });
        Ok(())
    }

    fn init(
        &mut self,
        input_device_name: &str,
        output_device_name: &str,
        callbacks: Arc<dyn MidiCallbacks>,
    ) -> Result<(), Box<dyn Error>> {
        match MIDI_STATE.with(|s| s.borrow().init_result.clone()) {
            Ok(()) => {
                MIDI_STATE.with_borrow_mut(|s| {
                    s.callbacks = Some(callbacks);
                    s.init_input_device_name = Some(input_device_name.to_string());
                    s.init_output_device_name = Some(output_device_name.to_string());
                });
                Ok(())
            }
            Err(e) => Err(e.to_string().into()),
        }
    }

    fn input(&self) -> &dyn IIo {
        &self.mock_input
    }

    fn io(&self, device_strategy: &dyn DeviceStrategy) -> &dyn IIo {
        MIDI_STATE.with_borrow_mut(|s| {
            s.io_count += 1;
            s.io_device_strategy = Some(device_strategy.clone_box());
        });
        device_strategy.io(self)
    }

    fn has_downloaded_init_data(&self) -> bool {
        MIDI_STATE.with_borrow_mut(|s| {
            s.has_downloaded_init_data_count += 1;
        });
        MIDI_STATE.with(|s| s.borrow().has_downloaded_init_data_result)
    }

    fn is_output_device_connected(&self) -> bool {
        MIDI_STATE.with(|s| s.borrow().is_output_device_connected)
    }

    fn is_receiving_data(&self) -> bool {
        MIDI_STATE.with(|s| s.borrow().is_receiving_data)
    }

    fn output(&self) -> &dyn IIo {
        &self.mock_output
    }

    fn refresh_devices(
        &mut self,
        device_name: &str,
        device_strategy: &dyn DeviceStrategy,
    ) -> Result<(), Box<dyn Error>> {
        MIDI_STATE.with_borrow_mut(|s| {
            s.refresh_devices_count += 1;
            s.refresh_devices_device_name = Some(device_name.to_string());
            s.refresh_devices_device_strategy = Some(device_strategy.clone_box());
            s.are_devices_connected = false;
            if *device_strategy.device_type() == DeviceType::Output {
                s.is_output_device_connected = false;
            }
            if s.simulate_devices_connected_changed {
                s.callbacks.as_ref().unwrap().on_devices_connected_changed();
            }
        });
        Ok(())
    }

    fn start_instrument_connection_monitor(&mut self) {
        MIDI_STATE.with_borrow_mut(|s| {
            s.start_instrument_connection_monitor_count += 1;
        });
    }

    fn stop_instrument_connection_monitor(&mut self) {
        MIDI_STATE.with_borrow_mut(|s| {
            s.stop_instrument_connection_monitor_count += 1;
        });
    }
}

pub struct MidiState {
    pub callbacks: Option<Arc<dyn MidiCallbacks>>,

    pub are_devices_connected: bool,

    pub close_count: u16,

    pub connect_device_count: u16,
    pub connect_device_index: Option<usize>,
    pub connect_device_device_strategy: Option<Box<dyn DeviceStrategy>>,

    pub has_downloaded_init_data_count: u16,
    pub has_downloaded_init_data_result: bool,

    init_result: Result<(), Arc<dyn Error>>,
    pub init_input_device_name: Option<String>,
    pub init_output_device_name: Option<String>,

    pub io_count: u16,
    pub io_device_strategy: Option<Box<dyn DeviceStrategy>>,

    pub is_output_device_connected: bool,
    pub is_receiving_data: bool,

    pub refresh_devices_count: u16,
    pub refresh_devices_device_name: Option<String>,
    pub refresh_devices_device_strategy: Option<Box<dyn DeviceStrategy>>,

    pub simulate_devices_connected_changed: bool,

    pub start_instrument_connection_monitor_count: u16,
    pub stop_instrument_connection_monitor_count: u16,
}

impl MidiState {
    pub fn new() -> Self {
        MidiState {
            callbacks: None,

            are_devices_connected: false,

            close_count: 0,

            connect_device_count: 0,
            connect_device_index: None,
            connect_device_device_strategy: None,

            has_downloaded_init_data_count: 0,
            has_downloaded_init_data_result: false,

            init_result: Ok(()),
            init_input_device_name: None,
            init_output_device_name: None,

            io_count: 0,
            io_device_strategy: None,

            is_output_device_connected: false,
            is_receiving_data: false,

            refresh_devices_count: 0,
            refresh_devices_device_name: None,
            refresh_devices_device_strategy: None,

            simulate_devices_connected_changed: false,
            start_instrument_connection_monitor_count: 0,
            stop_instrument_connection_monitor_count: 0,
        }
    }
}

impl Clone for MidiState {
    fn clone(&self) -> Self {
        MidiState {
            callbacks: self.callbacks.clone(),

            are_devices_connected: self.are_devices_connected,

            close_count: self.close_count,

            connect_device_count: self.connect_device_count,
            connect_device_index: self.connect_device_index,
            connect_device_device_strategy: self.connect_device_device_strategy.as_ref().map(|s| s.clone_box()),

            has_downloaded_init_data_count: self.has_downloaded_init_data_count,
            has_downloaded_init_data_result: self.has_downloaded_init_data_result,

            init_result: self.init_result.clone(),
            init_input_device_name: self.init_input_device_name.clone(),
            init_output_device_name: self.init_output_device_name.clone(),

            io_count: self.io_count,
            io_device_strategy: self.io_device_strategy.as_ref().map(|s| s.clone_box()),

            is_output_device_connected: self.is_output_device_connected,
            is_receiving_data: self.is_receiving_data,

            refresh_devices_count: self.refresh_devices_count,
            refresh_devices_device_name: self.refresh_devices_device_name.clone(),
            refresh_devices_device_strategy:
                self.refresh_devices_device_strategy.as_ref().map(|s| s.clone_box()),

            simulate_devices_connected_changed: self.simulate_devices_connected_changed,

            start_instrument_connection_monitor_count: self.start_instrument_connection_monitor_count,
            stop_instrument_connection_monitor_count: self.stop_instrument_connection_monitor_count,
        }
    }
}

thread_local! {
    static MIDI_STATE: RefCell<MidiState> = RefCell::new(MidiState::new());
}
