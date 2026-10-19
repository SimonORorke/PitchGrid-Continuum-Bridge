use std::error::Error;
use serde::{Serialize, Deserialize};
use app_info::APP_TITLE;
use crate::i_settings::ISettings;
use log::trace;
use simonor::{MidiDesktopSettings, PathFinder, SettingsFileIo};

/// Application settings, serialised to file.
#[derive(Serialize, Deserialize, Debug)]
pub struct Settings {
    auto_check_new_versions: bool,
    ignore_version: String,
    main_window_x: i32,
    main_window_y: i32,
    midi_input_device: String,
    midi_output_device: String,
    osc_listening_port: u16,
    pitch_table: u8,
    override_rounding_initial: bool,
    override_rounding_rate: bool,
    root_freq_override_index: u8,
    rounding_rate: u8,
    #[serde(skip, default = "create_settings_reader_writer")]
    io: SettingsFileIo,
}

impl Settings {
    pub fn new() -> Self {
        Self {
            auto_check_new_versions: true,
            ignore_version: String::new(),
            main_window_x: 0,
            main_window_y: 0,
            midi_input_device: String::new(),
            midi_output_device: String::new(),
            osc_listening_port: 0,
            pitch_table: 0,
            override_rounding_initial: true,
            override_rounding_rate: true,
            root_freq_override_index: 0,
            rounding_rate: 127,
            io: create_settings_reader_writer(),
        }
    }
}

impl MidiDesktopSettings for Settings {
    fn auto_check_new_versions(&self) -> bool { self.auto_check_new_versions }

    fn set_auto_check_new_versions(&mut self, value: bool) {
        self.auto_check_new_versions = value;
    }

    fn ignore_version(&self) -> &str { &self.ignore_version }

    fn set_ignore_version(&mut self, value: &str) { self.ignore_version = value.into() }

    fn main_window_x(&self) -> i32 {
        self.main_window_x
    }

    fn set_main_window_x(&mut self, value: i32) {
        self.main_window_x = value;
    }

    fn main_window_y(&self) -> i32 {
        self.main_window_y
    }

    fn set_main_window_y(&mut self, value: i32) {
        self.main_window_y = value;
    }

    fn midi_input_device(&self) -> &str {
        &self.midi_input_device
    }

    fn set_midi_input_device(&mut self, value: &str) {
        self.midi_input_device = value.into();
    }

    fn midi_output_device(&self) -> &str {
        &self.midi_output_device
    }

    fn set_midi_output_device(&mut self, value: &str) {
        self.midi_output_device = value.into();
    }

    fn read_from_file(&mut self) -> Result<(), Box<dyn Error>> {
        if let Some(settings) = self.io.read_from_file::<Settings>()? {
            self.auto_check_new_versions = settings.auto_check_new_versions;
            self.ignore_version = settings.ignore_version;
            self.main_window_x = settings.main_window_x;
            self.main_window_y = settings.main_window_y;
            self.midi_input_device = settings.midi_input_device;
            self.midi_output_device = settings.midi_output_device;
            self.osc_listening_port = settings.osc_listening_port;
            self.pitch_table = settings.pitch_table;
            self.override_rounding_initial = settings.override_rounding_initial;
            self.override_rounding_rate = settings.override_rounding_rate;
            self.root_freq_override_index = settings.root_freq_override_index;
            self.rounding_rate = settings.rounding_rate;
        }
        trace!("read_from_file: self.midi_input_device = {}; self.midi_output_device = {}; \
            self.pitch_table = {:?};", self.midi_input_device, self.midi_output_device, self.pitch_table);
        Ok(())
    }

    fn write_to_file(&mut self) -> Result<(), Box<dyn Error>> {
        self.io.write_to_file(&self)?;
        trace!("write_to_file: self.midi_input_device = {}; self.midi_output_device = {}; \
            self.pitch_table = {:?};", self.midi_input_device, self.midi_output_device, self.pitch_table);
        Ok(())
    }

    /// Replaces the default system path finder for testing.
    fn set_system_path_finder(&mut self, path_finder: Box<dyn PathFinder>) {
        self.io.set_system_path_finder(path_finder);
    }
}

impl ISettings for Settings {
    fn osc_listening_port(&self) -> u16 {
        self.osc_listening_port
    }

    fn set_osc_listening_port(&mut self, value: u16) {
        self.osc_listening_port = value;
    }

    fn pitch_table(&self) -> u8 {
        self.pitch_table
    }

    fn set_pitch_table(&mut self, value: u8) {
        self.pitch_table = value;
    }

    fn override_rounding_initial(&self) -> bool {
        self.override_rounding_initial
    }

    fn set_override_rounding_initial(&mut self, value: bool) {
        self.override_rounding_initial = value;
    }

    fn override_rounding_rate(&self) -> bool {
        self.override_rounding_rate
    }

    fn set_override_rounding_rate(&mut self, value: bool) {
        self.override_rounding_rate = value;
    }

    fn root_freq_override_index(&self) -> usize {
        self.root_freq_override_index as usize
    }

    fn set_root_freq_override_index(&mut self, value: usize) {
        self.root_freq_override_index = value as u8;
    }

    fn rounding_rate(&self) -> u8 {
        self.rounding_rate
    }

    fn set_rounding_rate(&mut self, value: u8) {
        self.rounding_rate = value;
    }
}

fn create_settings_reader_writer() -> SettingsFileIo {
    SettingsFileIo::new(APP_TITLE.to_string(), SETTINGS_FILE_NAME.to_string())
}

const SETTINGS_FILE_NAME: &str = "Settings.toml";
