#[path = "mock_io.rs"] pub mod mock_io;

use std::error::Error;
use std::sync::{LazyLock, Mutex, MutexGuard};
use pitchgrid_continuum::i_midi_manager::IMidiManager;
use pitchgrid_continuum::midi_ports::IIo;
use pitchgrid_continuum::device_strategy::DeviceStrategy;
use mock_io::MockIo;
use pitchgrid_continuum::global::DeviceType;

pub fn mock_midi() -> MutexGuard<'static, MockMidiManager> {
    MOCK_MIDI.lock().unwrap_or_else(|e| e.into_inner())
}

pub static MOCK_MIDI: LazyLock<Mutex<MockMidiManager>> =
    LazyLock::new(|| Mutex::new(MockMidiManager::new_state()));

pub struct MockMidiManager {
    pub are_devices_connected: bool,

    pub close_count: u16,

    connect_device_err: Option<String>,
    pub connect_device_count: u16,
    pub connect_device_index: Option<usize>,
    pub connect_device_device_strategy: Option<Box<dyn DeviceStrategy>>,

    pub init_input_device_name: Option<String>,
    pub init_output_device_name: Option<String>,

    pub io_count: u16,
    pub io_device_strategy: Option<Box<dyn DeviceStrategy>>,

    pub is_output_device_connected: bool,
    pub is_receiving_data: bool,

    pub refresh_devices_count: u16,
    pub refresh_devices_device_name: Option<String>,
    pub refresh_devices_device_strategy: Option<Box<dyn DeviceStrategy>>,

    pub start_instrument_connection_monitor_count: u16,
    pub stop_instrument_connection_monitor_count: u16,
}

impl MockMidiManager {
    fn new_state() -> Self {
        MockMidiManager {
            are_devices_connected: false,

            close_count: 0,

            connect_device_err: None,
            connect_device_count: 0,
            connect_device_index: None,
            connect_device_device_strategy: None,

            init_input_device_name: None,
            init_output_device_name: None,

            io_count: 0,
            io_device_strategy: None,

            is_output_device_connected: false,
            is_receiving_data: false,

            refresh_devices_count: 0,
            refresh_devices_device_name: None,
            refresh_devices_device_strategy: None,

            start_instrument_connection_monitor_count: 0,
            stop_instrument_connection_monitor_count: 0,
        }
    }

    pub fn new(
        input_device_names: Vec<String>,
        output_device_names: Vec<String>,
        initial_input_device_name: &str,
        initial_output_device_name: &str,
    ) -> Box<dyn IMidiManager + Send> {
        *MOCK_MIDI.lock().unwrap_or_else(|e| e.into_inner()) = MockMidiManager::new_state();
        let mut input = MockIo::new(DeviceType::Input, input_device_names);
        input.set_device(initial_input_device_name);
        let mut output = MockIo::new(DeviceType::Output, output_device_names);
        output.set_device(initial_output_device_name);
        Box::new(MockMidiManagerImpl { mock_input: input, mock_output: output })
    }

    pub fn set_are_devices_connected(value: bool) {
        MOCK_MIDI.lock().unwrap_or_else(|e| e.into_inner()).are_devices_connected = value;
    }

    pub fn set_is_receiving_data(value: bool) {
        MOCK_MIDI.lock().unwrap_or_else(|e| e.into_inner()).is_receiving_data = value;
    }

    pub fn simulate_connect_device_err(msg: &str) {
        MOCK_MIDI.lock().unwrap_or_else(|e| e.into_inner()).connect_device_err =
            Some(msg.to_string());
    }
}

struct MockMidiManagerImpl {
    mock_input: MockIo,
    mock_output: MockIo,
}

impl IMidiManager for MockMidiManagerImpl {
    fn are_devices_connected(&self) -> bool {
        MOCK_MIDI.lock().unwrap_or_else(|e| e.into_inner()).are_devices_connected
    }

    fn close(&mut self) {
        MOCK_MIDI.lock().unwrap_or_else(|e| e.into_inner()).close_count += 1;
    }

    fn connect_device(
        &mut self,
        index: usize,
        device_strategy: &dyn DeviceStrategy,
    ) -> Result<(), Box<dyn Error>> {
        let mut state = MOCK_MIDI.lock().unwrap_or_else(|e| e.into_inner());
        state.connect_device_count += 1;
        state.connect_device_index = Some(index);
        state.connect_device_device_strategy = Some(device_strategy.clone_box());
        match device_strategy.device_type() {
            DeviceType::Input => {
                if state.is_output_device_connected {
                    state.are_devices_connected = true;
                }
            }
            DeviceType::Output => {
                state.is_output_device_connected = true;
            }
        }
        match &state.connect_device_err {
            Some(msg) =>
                Err(Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other, msg.clone())) as Box<dyn Error>),
            None => Ok(()),
        }
    }

    fn init(
        &mut self,
        input_device_name: &str,
        output_device_name: &str,
    ) {
        let mut state = MOCK_MIDI.lock().unwrap_or_else(|e| e.into_inner());
        state.init_input_device_name = Some(input_device_name.to_string());
        state.init_output_device_name = Some(output_device_name.to_string());
    }

    fn input(&self) -> &dyn IIo {
        &self.mock_input
    }

    fn io(&self, device_strategy: &dyn DeviceStrategy) -> &dyn IIo {
        {
            let mut state = MOCK_MIDI.lock().unwrap_or_else(|e| e.into_inner());
            state.io_count += 1;
            state.io_device_strategy = Some(device_strategy.clone_box());
        }
        device_strategy.io(self)
    }

    fn is_output_device_connected(&self) -> bool {
        MOCK_MIDI.lock().unwrap_or_else(|e| e.into_inner()).is_output_device_connected
    }

    fn is_receiving_data(&self) -> bool {
        MOCK_MIDI.lock().unwrap_or_else(|e| e.into_inner()).is_receiving_data
    }

    fn output(&self) -> &dyn IIo {
        &self.mock_output
    }

    fn refresh_devices(
        &mut self,
        device_name: &str,
        device_strategy: &dyn DeviceStrategy,
    ) {
        let mut state = MOCK_MIDI.lock().unwrap_or_else(|e| e.into_inner());
        state.refresh_devices_count += 1;
        state.refresh_devices_device_name = Some(device_name.to_string());
        state.refresh_devices_device_strategy = Some(device_strategy.clone_box());
        state.are_devices_connected = false;
        if *device_strategy.device_type() == DeviceType::Output {
            state.is_output_device_connected = false;
        }
    }

    fn start_instrument_connection_monitor(&mut self) {
        MOCK_MIDI.lock().unwrap_or_else(|e| e.into_inner())
            .start_instrument_connection_monitor_count += 1;
    }

    fn stop_instrument_connection_monitor(&mut self) {
        MOCK_MIDI.lock().unwrap_or_else(|e| e.into_inner())
            .stop_instrument_connection_monitor_count += 1;
    }
}
