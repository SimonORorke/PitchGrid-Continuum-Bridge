use std::sync::Arc;
use crate::i_continuum_protocol::TuningUpdateSignaller;
use crate::midi_sender::IMidiSender;
use crate::tuning_params::TuningParams;
use crate::tuning_params::FormattedTuningParams;

/// A trait that defines the interface for tuning a Continuum from PitchGrid parameters.
///
/// The `I` prefix is borrowed from .Net standards, where `I` stands for `interface`,
/// which is the .Net equivalent of Rust's `trait`.
///
/// Rationale for the `I` prefix:
/// `Tuner` describes the purpose of the struct of that name,
/// which implements `ITuner`. The `ITuner` trait exists only to allow `Tuner` to be mocked.
/// Nor do we use `mockall` or similar to create the mocks:  so, for example, we don't need to
/// rename trait `ITuner` to `Tuner` so that a mock called `MockTuner` can be automatically
/// generated. And a `T` prefix is avoided because `T` in Rust is a common type parameter.
pub trait ITuner: Send + Sync {
    fn init(&self, pitch_table: u8);
    fn on_tuning_received(&self, params: TuningParams);
    fn has_data(&self) -> bool;
    fn remove_data(&self);
    fn send_current_preset_update(&self) -> bool;
    fn formatted_tuning_params(&self) -> FormattedTuningParams;
    fn is_root_freq_overridden(&self) -> bool;
    fn set_root_freq_override_note_no(&self, index: usize, send_tuning: bool);
    fn set_override_rounding_initial(&self, value: bool);
    fn set_override_rounding_rate(&self, value: bool);
    fn set_rounding_rate(&self, rate: u8);
    fn set_pitch_table(&self, pitch_table: u8);
    fn on_tuning_updated(&self);
    fn set_midi_sender(&self, sender: Box<dyn IMidiSender>);
    fn set_tuning_signaller(&self, signaller: Arc<dyn TuningUpdateSignaller>);
    fn pitch_table_index(&self) -> usize;
}

pub type SharedTuner = Arc<dyn ITuner>;
