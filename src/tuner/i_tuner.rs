use std::sync::Arc;
use crate::midi_sender::IMidiSender;
use crate::tuning_params::TuningParams;
use super::FormattedTuningParams;

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
    fn pitch_table_index(&self) -> usize;
}

pub type SharedTuner = Arc<dyn ITuner>;
