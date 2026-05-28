use std::cell::RefCell;
use std::error::Error;
use std::sync::Arc;
use pitchgrid_continuum::i_settings::ISettings;
use pitchgrid_continuum::path_finder::PathFinder;

/// Returns a clone of the current `SettingsState`.
pub fn settings_state() -> SettingsState {
    SETTINGS_STATE.with(|s| s.borrow().clone())
}

pub struct MockSettings {}

impl MockSettings {
    pub fn new() -> Self {
        SETTINGS_STATE.replace(SettingsState::new());
        MockSettings {}
    }

    /// As `new()` resets state, this method takes `&mut self`, making it clear that is operates
    /// on the instance and must be called after construction.
    pub fn simulate_read_from_file_err(&mut self, msg: &str) {
        SETTINGS_STATE.with_borrow_mut(|s| s.read_from_file_result =
            Err(Arc::new(std::io::Error::new(std::io::ErrorKind::Other, msg))));
    }

    /// As `new()` resets state, this method takes `&mut self`, making it clear that is operates
    /// on the instance and must be called after construction.
    pub fn simulate_write_to_file_err(&mut self, msg: &str) {
        SETTINGS_STATE.with_borrow_mut(|s| s.write_to_file_result =
            Err(Arc::new(std::io::Error::new(std::io::ErrorKind::Other, msg))));
    }
}

impl ISettings for MockSettings {
    fn main_window_x(&self) -> i32 {
        SETTINGS_STATE.with(|s| s.borrow().main_window_x)
    }

    fn set_main_window_x(&mut self, value: i32) {
        SETTINGS_STATE.with_borrow_mut(|s| s.main_window_x = value);
    }

    fn main_window_y(&self) -> i32 {
        SETTINGS_STATE.with(|s| s.borrow().main_window_y)
    }

    fn set_main_window_y(&mut self, value: i32) {
        SETTINGS_STATE.with_borrow_mut(|s| s.main_window_y = value);
    }

    fn midi_input_device(&self) -> &str {
        Box::leak(SETTINGS_STATE.with(|s| s.borrow().midi_input_device.clone()).into_boxed_str())
    }

    fn set_midi_input_device(&mut self, value: &str) {
        SETTINGS_STATE.with_borrow_mut(|s| s.midi_input_device = value.to_string());
    }

    fn midi_output_device(&self) -> &str {
        Box::leak(SETTINGS_STATE.with(|s| s.borrow().midi_output_device.clone()).into_boxed_str())
    }

    fn set_midi_output_device(&mut self, value: &str) {
        SETTINGS_STATE.with_borrow_mut(|s| s.midi_output_device = value.to_string());
    }

    fn osc_listening_port(&self) -> u16 {
        SETTINGS_STATE.with(|s| s.borrow().osc_listening_port)
    }

    fn set_osc_listening_port(&mut self, value: u16) {
        SETTINGS_STATE.with_borrow_mut(|s| s.osc_listening_port = value);
    }

    fn pitch_table(&self) -> u8 {
        SETTINGS_STATE.with(|s| s.borrow().pitch_table)
    }

    fn set_pitch_table(&mut self, value: u8) {
        SETTINGS_STATE.with_borrow_mut(|s| s.pitch_table = value);
    }

    fn override_rounding_initial(&self) -> bool {
        SETTINGS_STATE.with(|s| s.borrow().override_rounding_initial)
    }

    fn set_override_rounding_initial(&mut self, value: bool) {
        SETTINGS_STATE.with_borrow_mut(|s| s.override_rounding_initial = value);
    }

    fn override_rounding_rate(&self) -> bool {
        SETTINGS_STATE.with(|s| s.borrow().override_rounding_rate)
    }

    fn set_override_rounding_rate(&mut self, value: bool) {
        SETTINGS_STATE.with_borrow_mut(|s| s.override_rounding_rate = value);
    }

    fn rounding_rate(&self) -> u8 {
        SETTINGS_STATE.with(|s| s.borrow().rounding_rate)
    }

    fn set_rounding_rate(&mut self, value: u8) {
        SETTINGS_STATE.with_borrow_mut(|s| s.rounding_rate = value);
    }

    fn read_from_file(&mut self) -> Result<(), Box<dyn Error>> {
        match SETTINGS_STATE.with(|s| s.borrow().read_from_file_result.clone()) {
            Ok(()) => Ok(()),
            Err(e) => Err(e.to_string().into()),
        }
    }

    fn write_to_file(&mut self) -> Result<(), Box<dyn Error>> {
        match SETTINGS_STATE.with(|s| s.borrow().write_to_file_result.clone()) {
            Ok(()) => Ok(()),
            Err(e) => Err(e.to_string().into()),
        }
    }

    fn set_system_path_finder(&mut self, _path_finder: Box<dyn PathFinder>) {}
}

#[derive(Clone)]
pub struct SettingsState {
    pub main_window_x: i32,
    pub main_window_y: i32,
    pub midi_input_device: String,
    pub midi_output_device: String,
    pub osc_listening_port: u16,
    pub pitch_table: u8,
    pub override_rounding_initial: bool,
    pub override_rounding_rate: bool,
    pub rounding_rate: u8,

    read_from_file_result: Result<(), Arc<dyn Error>>,
    write_to_file_result: Result<(), Arc<dyn Error>>,
}

impl SettingsState {
    pub fn new() -> Self {
        SettingsState {
            main_window_x: 0,
            main_window_y: 0,
            midi_input_device: String::new(),
            midi_output_device: String::new(),
            osc_listening_port: 0,
            pitch_table: 0,
            override_rounding_initial: true,
            override_rounding_rate: true,
            rounding_rate: 127,

            read_from_file_result: Ok(()),
            write_to_file_result: Ok(()),
        }
    }
}

thread_local! {
    static SETTINGS_STATE: RefCell<SettingsState> = RefCell::new(SettingsState::new());
}
