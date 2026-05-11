use std::error::Error;
use std::fs;
use std::path::PathBuf;
use serde::{Serialize, Deserialize};
use app_info::APP_TITLE;

#[derive(Serialize, Deserialize, Debug)]
pub struct Settings {
    pub main_window_x: i32,
    pub main_window_y: i32,
    pub midi_input_device: String,
    pub midi_output_device: String,
    pub osc_listening_port: u16,
    pub pitch_table: u8,
    pub override_rounding_initial: bool,
    pub override_rounding_rate: bool,
    pub rounding_rate: u8,
}

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
        }
    }

    fn get_app_config_folder_path(&self) -> Result<PathBuf, Box<dyn Error>> {
        if let Some(base_dirs) = directories::BaseDirs::new() {
            let config_folder_path = base_dirs.config_dir().to_path_buf();
            return Ok(config_folder_path.join(APP_TITLE));
        }
        Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::NotFound, NO_SETTINGS_FOLDER_PATH)))
    }

    fn get_path(&self) -> Result<PathBuf, Box<dyn Error>> {
        Ok(self.get_app_config_folder_path()?.join("Settings.toml"))
    }

    pub fn read_from_file(&mut self) -> Result<(), Box<dyn Error>> {
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
        // println!("Settings.read_from_file: self.midi_input_device = {}; self.midi_output_device = {}; \
        // self.pitch_table = {:?};", self.midi_input_device, self.midi_output_device, self.pitch_table);
        Ok(())
    }

    pub fn write_to_file(&mut self) -> Result<(), Box<dyn Error>> {
        let path = self.get_path()?;
        // It is safe to unwrap as get_path() would have thrown an error if a parent folder
        // could not be specified.
        let parent_folder_path = path.parent().unwrap();
        if !parent_folder_path.exists() {
            fs::create_dir(parent_folder_path)?;
        }
        // Serialise settings.
        let toml = toml::to_string(&self)?;
        if let Err(e) = fs::write(&path, toml) {
            return Err(
                Box::new(std::io::Error::new(e.kind(),
                format!("Error writing settings file '{:?}': {e}", path))));
        }
        // println!("Settings.write_to_file: self.midi_input_device = {}; self.midi_output_device = {}; \
        // self.pitch_table = {:?};", self.midi_input_device, self.midi_output_device, self.pitch_table);
        Ok(())
    }
}

const NO_SETTINGS_FOLDER_PATH: &str = "A valid settings folder path cannot be specified.";
