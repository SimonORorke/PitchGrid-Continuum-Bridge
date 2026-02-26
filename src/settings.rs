use std::error::Error;
use std::fs;
use serde::{Serialize, Deserialize};
use crate::global::APP_TITLE;

#[derive(Serialize, Deserialize, Debug)]
pub struct Settings {
    pub midi_input_port: String,
    pub midi_output_port: String,
    pub pitch_table: i32,
}

impl Settings {
    pub fn new() -> Self {
        Self {
            midi_input_port: String::new(),
            midi_output_port: String::new(),
            pitch_table: 0,
        }
    }

    fn get_app_config_folder_path(&self) -> String {
        let mut path = String::new();
        if let Some(base_dirs) = directories::BaseDirs::new() {
            let config_folder_path = base_dirs.config_dir().to_str().unwrap().to_string();
            path =
                format!("{}/{}", config_folder_path, APP_TITLE);
        }
        path
    }

    fn get_path(&self) -> String {
        format!("{}/Settings.toml", self.get_app_config_folder_path())
    }

    pub fn read_from_file(&mut self) -> Result<(), Box<dyn Error>> {
        let path = self.get_path();
        let toml_str = match fs::read_to_string(&path) {
            Ok(s) => s,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(e) => {
                let msg = format!("Error reading settings file '{}': {}", path.clone(), e);
                return Err(Box::new(std::io::Error::new(e.kind(), msg)));
            }
        };
        // Deserialise settings.
        let settings = toml::from_str::<Settings>(&toml_str)
            .map_err(|e| {
                let msg = format!("Error parsing settings file '{}': {}", path.clone(), e);
                std::io::Error::new(std::io::ErrorKind::InvalidData, msg)
            })?;
        self.midi_input_port = settings.midi_input_port;
        self.midi_output_port = settings.midi_output_port;
        self.pitch_table = settings.pitch_table;
        // println!("Settings.read_from_file: self.midi_input_port = {}", self.midi_input_port);
        Ok(())
    }

    pub fn write_to_file(&mut self) -> Result<(), Box<dyn Error>> {
        let path = self.get_path();
        if path.is_empty() {
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::NotFound, "Settings path is empty")));
        }
        let app_config_folder_path = self.get_app_config_folder_path();
        let app_config_folder_exists = fs::exists(&app_config_folder_path)?;
        if !app_config_folder_exists {
            fs::create_dir(app_config_folder_path)?;
        }
        // Serialise settings.
        let toml = toml::to_string(&self)?;
        if let Err(e) = fs::write(&path, toml) {
            return Err(
                Box::new(std::io::Error::new(e.kind(),
                format!("Error writing settings file '{path}': {e}"))));
        }
        // println!("Settings.write_to_file:");
        // println!("    self.midi_input_port = {}", self.midi_input_port);
        // println!("    self.midi_output_port = {}", self.midi_output_port);
        Ok(())
    }
}