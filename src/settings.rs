use std::error::Error;
use serde::{Serialize, Deserialize};
use crate::global;

#[derive(Serialize, Deserialize, Debug)]
pub struct Settings {
    pub midi_output_port: String,
}

impl Settings {
    pub fn new() -> Self {
        Self {
            midi_output_port: String::new(),
        }
    }

    fn get_path(&self) -> String {
        let mut path = String::new();
        if let Some(base_dirs) = directories::BaseDirs::new() {
            let config_dir = base_dirs.config_dir().to_str().unwrap().to_string();
            path = format!("{}/{}/Settings.toml", config_dir, global::APP_TITLE);
        }
        path
    }

    pub fn read_from_file(&mut self) -> Result<(), Box<dyn Error>> {
        let path = self.get_path();
        let toml_str = match std::fs::read_to_string(&path) {
            Ok(s) => s,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(e) => {
                let msg = format!("Error reading settings file '{}': {}", path.clone(), e);
                return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, msg)));
            }
        };
        let settings = toml::from_str::<Settings>(&toml_str)
            .map_err(|e| {
                let msg = format!("Error parsing settings file '{}': {}", path.clone(), e);
                std::io::Error::new(std::io::ErrorKind::InvalidData, msg)
            })?;

        self.midi_output_port = settings.midi_output_port;
        Ok(())
    }

    pub fn write_to_file(&self) -> Result<(), Box<dyn Error>> {
        let path = self.get_path();
        if path.is_empty() {
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::NotFound, "Settings path is empty")));
        }
        let toml = toml::to_string(&self)?;
        std::fs::write(path, toml)?;
        Ok(())
    }
}