use std::error::Error;
use std::sync::{LazyLock, Mutex, MutexGuard};
use pitchgrid_continuum::i_settings::ISettings;
use pitchgrid_continuum::path_finder::PathFinder;

pub fn mock_settings() -> MutexGuard<'static, MockSettings> {
    MOCK_SETTINGS.lock().unwrap_or_else(|e| e.into_inner())
}

pub static MOCK_SETTINGS: LazyLock<Mutex<MockSettings>> =
    LazyLock::new(|| Mutex::new(MockSettings::new_state()));

pub struct MockSettings {
    pub main_window_x: i32,
    pub main_window_y: i32,
    pub midi_input_device: String,
    pub midi_output_device: String,
    pub osc_listening_port: u16,
    pub pitch_table: u8,
    pub override_rounding_initial: bool,
    pub override_rounding_rate: bool,
    pub rounding_rate: u8,

    read_from_file_err: Option<String>,
    write_to_file_err: Option<String>,
}

impl MockSettings {
    fn new_state() -> Self {
        MockSettings {
            main_window_x: 0,
            main_window_y: 0,
            midi_input_device: String::new(),
            midi_output_device: String::new(),
            osc_listening_port: 0,
            pitch_table: 0,
            override_rounding_initial: true,
            override_rounding_rate: true,
            rounding_rate: 127,
            read_from_file_err: None,
            write_to_file_err: None,
        }
    }

    pub fn new() -> Self {
        *MOCK_SETTINGS.lock().unwrap_or_else(|e| e.into_inner()) = MockSettings::new_state();
        MockSettings::new_state()
    }

    pub fn simulate_read_from_file_err(msg: &str) {
        MOCK_SETTINGS.lock().unwrap_or_else(|e| e.into_inner()).read_from_file_err =
            Some(msg.to_string());
    }

    pub fn simulate_write_to_file_err(msg: &str) {
        MOCK_SETTINGS.lock().unwrap_or_else(|e| e.into_inner()).write_to_file_err =
            Some(msg.to_string());
    }
}

impl ISettings for MockSettings {
    fn main_window_x(&self) -> i32 {
        MOCK_SETTINGS.lock().unwrap_or_else(|e| e.into_inner()).main_window_x
    }

    fn set_main_window_x(&mut self, value: i32) {
        MOCK_SETTINGS.lock().unwrap_or_else(|e| e.into_inner()).main_window_x = value;
    }

    fn main_window_y(&self) -> i32 {
        MOCK_SETTINGS.lock().unwrap_or_else(|e| e.into_inner()).main_window_y
    }

    fn set_main_window_y(&mut self, value: i32) {
        MOCK_SETTINGS.lock().unwrap_or_else(|e| e.into_inner()).main_window_y = value;
    }

    fn midi_input_device(&self) -> &str {
        Box::leak(MOCK_SETTINGS.lock().unwrap_or_else(|e| e.into_inner())
            .midi_input_device.clone().into_boxed_str())
    }

    fn set_midi_input_device(&mut self, value: &str) {
        MOCK_SETTINGS.lock().unwrap_or_else(|e| e.into_inner()).midi_input_device =
            value.to_string();
    }

    fn midi_output_device(&self) -> &str {
        Box::leak(MOCK_SETTINGS.lock().unwrap_or_else(|e| e.into_inner())
            .midi_output_device.clone().into_boxed_str())
    }

    fn set_midi_output_device(&mut self, value: &str) {
        MOCK_SETTINGS.lock().unwrap_or_else(|e| e.into_inner()).midi_output_device =
            value.to_string();
    }

    fn osc_listening_port(&self) -> u16 {
        MOCK_SETTINGS.lock().unwrap_or_else(|e| e.into_inner()).osc_listening_port
    }

    fn set_osc_listening_port(&mut self, value: u16) {
        MOCK_SETTINGS.lock().unwrap_or_else(|e| e.into_inner()).osc_listening_port = value;
    }

    fn pitch_table(&self) -> u8 {
        MOCK_SETTINGS.lock().unwrap_or_else(|e| e.into_inner()).pitch_table
    }

    fn set_pitch_table(&mut self, value: u8) {
        MOCK_SETTINGS.lock().unwrap_or_else(|e| e.into_inner()).pitch_table = value;
    }

    fn override_rounding_initial(&self) -> bool {
        MOCK_SETTINGS.lock().unwrap_or_else(|e| e.into_inner()).override_rounding_initial
    }

    fn set_override_rounding_initial(&mut self, value: bool) {
        MOCK_SETTINGS.lock().unwrap_or_else(|e| e.into_inner()).override_rounding_initial = value;
    }

    fn override_rounding_rate(&self) -> bool {
        MOCK_SETTINGS.lock().unwrap_or_else(|e| e.into_inner()).override_rounding_rate
    }

    fn set_override_rounding_rate(&mut self, value: bool) {
        MOCK_SETTINGS.lock().unwrap_or_else(|e| e.into_inner()).override_rounding_rate = value;
    }

    fn rounding_rate(&self) -> u8 {
        MOCK_SETTINGS.lock().unwrap_or_else(|e| e.into_inner()).rounding_rate
    }

    fn set_rounding_rate(&mut self, value: u8) {
        MOCK_SETTINGS.lock().unwrap_or_else(|e| e.into_inner()).rounding_rate = value;
    }

    fn read_from_file(&mut self) -> Result<(), Box<dyn Error>> {
        match MOCK_SETTINGS.lock().unwrap_or_else(|e| e.into_inner()).read_from_file_err.clone() {
            None => Ok(()),
            Some(msg) => Err(msg.into()),
        }
    }

    fn write_to_file(&mut self) -> Result<(), Box<dyn Error>> {
        match MOCK_SETTINGS.lock().unwrap_or_else(|e| e.into_inner()).write_to_file_err.clone() {
            None => Ok(()),
            Some(msg) => Err(msg.into()),
        }
    }

    fn set_system_path_finder(&mut self, _path_finder: Box<dyn PathFinder>) {}
}
