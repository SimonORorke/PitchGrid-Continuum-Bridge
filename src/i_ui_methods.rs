use crate::global::MessageType;
use crate::device_strategy::DeviceStrategy;
use crate::tuning_params::FormattedTuningParams;

/// A trait that defines the interface for methods called by `Presenter`
/// to make changes to the UI.
///
/// The `I` prefix is borrowed from .Net standards, where `I` stands for `interface`,
/// which is the .Net equivalent of Rust's `trait`.
///
/// Rationale for the `I` prefix:
/// `UiMethods` describes the purpose of the struct of that name,
/// which implements `IUiMethods`. The `IUiMethods` trait exists only to allow `UiMethods` to be mocked.
/// Nor do we use `mockall` or similar to create the mocks:  so, for example, we don't need to
/// rename trait `IUiMethods` to `UiMethods` so that a mock called `MockUiMethods` can be automatically
/// generated. And a `T` prefix is avoided because `T` in Rust is a common type parameter.
///
/// Project-wide trait-naming convention (two tiers). This is the key justification for the
/// non-idiomatic `I` prefix, so the two tiers are kept distinct on purpose:
/// * **`I`-prefixed traits are service interfaces**: each abstracts a *single* concrete struct
///   (`ISettings`/`Settings`, `IOsc`/`Osc`, `IMidiManager`/`MidiManager`,
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
pub trait IUiMethods: Send + Sync {
    fn focus_device(&self, device_strategy: &dyn DeviceStrategy);
    fn get_selected_device_index(&self, device_strategy: &dyn DeviceStrategy) -> usize;
    fn set_selected_device_index(&self, index: usize, device_strategy: &dyn DeviceStrategy);
    fn set_devices_model(&self, device_names: &[String], device_strategy: &dyn DeviceStrategy);
    fn show_connected_device_name(&self, name: &str, msg_type: MessageType,
                                  device_strategy: &dyn DeviceStrategy);
    fn show_message(&self, msg: &str, msg_type: MessageType);
    fn show_pitchgrid_status(&self, status: &str, msg_type: MessageType);
    fn show_tuning(&self, tuning: FormattedTuningParams, is_root_freq_overridden: bool);
    fn set_main_window_position(&self, x: i32, y: i32);
    fn set_override_rounding_initial(&self, value: bool);
    fn set_override_rounding_rate(&self, value: bool);
    fn set_rounding_rate(&self, rate: u8);
    fn set_selected_osc_listening_port_index(&self, index: i32);
    fn set_selected_pitch_table_index(&self, index: i32);
}
