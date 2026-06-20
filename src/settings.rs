use std::error::Error;
use std::fs;
use std::path::PathBuf;
use serde::{Serialize, Deserialize};
use app_info::APP_TITLE;
use crate::i_settings::ISettings;
use crate::path_finder::{PathFinder, SystemPathFinder};
use log::trace;

// Application settings, serialised to file.
#[derive(Serialize, Deserialize, Debug)]
pub struct Settings {
    main_window_x: i32,
    main_window_y: i32,
    midi_input_device: String,
    midi_output_device: String,
    osc_listening_port: u16,
    pitch_table: u8,
    override_rounding_initial: bool,
    override_rounding_rate: bool,
    rounding_rate: u8,
    #[serde(skip, default = "default_path_finder")]
    system_path_finder: Box<dyn PathFinder>,
}
/// For public functions, see `impl ISettings for Settings`.
impl Settings {
    pub fn new() -> Self {
        Self {
            main_window_x: 0,
            main_window_y: 0,
            midi_input_device: String::new(),
            midi_output_device: String::new(),
            osc_listening_port: 0,
            pitch_table: 0,
            override_rounding_initial: true,
            override_rounding_rate: true,
            rounding_rate: 127,
            system_path_finder: default_path_finder(),
        }
    }

    fn get_app_config_folder_path(&self) -> Result<PathBuf, Box<dyn Error>> {
        Ok(self.system_path_finder.config_folder_path()?.join(APP_TITLE))
    }

    fn get_path(&self) -> Result<PathBuf, Box<dyn Error>> {
        Ok(self.get_app_config_folder_path()?.join(SETTINGS_FILE_NAME))
    }
}

impl ISettings for Settings {
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

    fn rounding_rate(&self) -> u8 {
        self.rounding_rate
    }

    fn set_rounding_rate(&mut self, value: u8) {
        self.rounding_rate = value;
    }

    fn read_from_file(&mut self) -> Result<(), Box<dyn Error>> {
        let path = self.get_path()?;
        let toml_str = match fs::read_to_string(&path) {
            Ok(s) => s,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(e) => {
                let msg = format!("Error reading settings file '{:?}': {}", path, e);
                return Err(Box::new(std::io::Error::new(e.kind(), msg)));
            }
        };
        // Deserialise settings.
        let settings = match toml::from_str::<Settings>(&toml_str) {
            Ok(s) => s,
            Err(_) => {
                // A parsing error will happen if there are new settings.
                // So, instead of returning an error, just return without restoring any settings.
                return Ok(());
            }
        };
        self.main_window_x = settings.main_window_x;
        self.main_window_y = settings.main_window_y;
        self.midi_input_device = settings.midi_input_device;
        self.midi_output_device = settings.midi_output_device;
        self.osc_listening_port = settings.osc_listening_port;
        self.pitch_table = settings.pitch_table;
        self.override_rounding_initial = settings.override_rounding_initial;
        self.override_rounding_rate = settings.override_rounding_rate;
        self.rounding_rate = settings.rounding_rate;
        trace!("read_from_file: self.midi_input_device = {}; self.midi_output_device = {}; \
            self.pitch_table = {:?};", self.midi_input_device, self.midi_output_device, self.pitch_table);
        Ok(())
    }

    fn write_to_file(&mut self) -> Result<(), Box<dyn Error>> {
        let path = self.get_path()?;
        // It is safe to unwrap as get_path() would have thrown an error if a parent folder
        // could not be specified.
        let parent_folder_path = path.parent().unwrap();
        if !parent_folder_path.try_exists()? {
            fs::create_dir(parent_folder_path)?;
        }
        // Serialise settings.
        let toml = toml::to_string(&self)?;
        if let Err(e) = fs::write(&path, toml) {
            return Err(
                Box::new(std::io::Error::new(e.kind(),
                format!("Error writing settings file '{:?}': {e}", path))));
        }
        trace!("write_to_file: self.midi_input_device = {}; self.midi_output_device = {}; \
            self.pitch_table = {:?};", self.midi_input_device, self.midi_output_device, self.pitch_table);
        Ok(())
    }

    /// Replaces the default system path finder for testing.
    fn set_system_path_finder(&mut self, path_finder: Box<dyn PathFinder>) {
        self.system_path_finder = path_finder;
    }
}

fn default_path_finder() -> Box<dyn PathFinder> {
    Box::new(SystemPathFinder::new())
}

const SETTINGS_FILE_NAME: &str = "toml";
