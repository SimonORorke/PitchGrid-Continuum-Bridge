use std::error::Error;
use crate::path_finder::PathFinder;

/// A trait that defines the interface for application settings.
///
/// For the The `I` prefix, see `ITuner`s doc comment.
pub trait ISettings: Send + Sync {
    fn main_window_x(&self) -> i32;
    fn set_main_window_x(&mut self, value: i32);
    fn main_window_y(&self) -> i32;
    fn set_main_window_y(&mut self, value: i32);
    fn midi_input_device(&self) -> &str;
    fn set_midi_input_device(&mut self, value: &str);
    fn midi_output_device(&self) -> &str;
    fn set_midi_output_device(&mut self, value: &str);
    fn osc_listening_port(&self) -> u16;
    fn set_osc_listening_port(&mut self, value: u16);
    fn pitch_table(&self) -> u8;
    fn set_pitch_table(&mut self, value: u8);
    fn override_rounding_initial(&self) -> bool;
    fn set_override_rounding_initial(&mut self, value: bool);
    fn override_rounding_rate(&self) -> bool;
    fn set_override_rounding_rate(&mut self, value: bool);
    fn rounding_rate(&self) -> u8;
    fn set_rounding_rate(&mut self, value: u8);
    fn read_from_file(&mut self) -> Result<(), Box<dyn Error>>;
    fn write_to_file(&mut self) -> Result<(), Box<dyn Error>>;
    fn set_system_path_finder(&mut self, path_finder: Box<dyn PathFinder>);
}
