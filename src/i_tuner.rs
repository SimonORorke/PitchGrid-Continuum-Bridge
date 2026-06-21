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
///
/// Project-wide trait-naming convention (two tiers). This is the key justification for the
/// non-idiomatic `I` prefix, so the two tiers are kept distinct on purpose:
/// * **`I`-prefixed traits are service interfaces**: each abstracts a *single* concrete struct
///   (`ITuner`/`Tuner`, `ISettings`/`Settings`, `IOsc`/`Osc`, `IMidiManager`/`MidiManager`,
///   `IContinuumProtocol`/`ContinuumProtocol`, `IUiMethods`/`UiMethods`, `IMidiSender`/`MidiSender`)
///   so it can be injected and mocked. The struct keeps the descriptive name; the trait takes the
///   `I` prefix.
/// * **Role-named traits with no prefix are seams / callbacks**: named for what they do rather than
///   for one concrete type, and may have several implementors — e.g. `MidiInputListener`,
///   `ContinuumProtocolListener`, `TuningUpdateSignaller`.
///
/// So an `I` prefix vs a bare role-name is a deliberate signal of which kind of trait it is. Don't
/// "de-prefix" the service interfaces to match Rust's usual no-`I` convention — that would erase the
/// distinction.
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
