use crate::global::MessageType;
use crate::device_strategy::DeviceStrategy;
use crate::tuning_params::FormattedTuningParams;

/// A trait that defines the interface for methods called by `Controller`
/// to make changes to the UI.
///
/// For the `I` prefix, see `ITuner`s doc comment.
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
