use simonor::{MidiDesktopSettings};

/// A trait that defines the interface for application settings.
///
/// For the `I` prefix, see `IUiMethods`s doc comment.
pub trait ISettings: MidiDesktopSettings + Send + Sync {
    fn osc_listening_port(&self) -> u16;
    fn set_osc_listening_port(&mut self, value: u16);
    fn pitch_table(&self) -> u8;
    fn set_pitch_table(&mut self, value: u8);
    fn override_rounding_initial(&self) -> bool;
    fn set_override_rounding_initial(&mut self, value: bool);
    fn override_rounding_rate(&self) -> bool;
    fn set_override_rounding_rate(&mut self, value: bool);
    fn root_freq_override_index(&self) -> usize;
    fn set_root_freq_override_index(&mut self, value: usize);
    fn rounding_rate(&self) -> u8;
    fn set_rounding_rate(&mut self, value: u8);
}
