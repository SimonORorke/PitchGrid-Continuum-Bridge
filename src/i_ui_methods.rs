use crate::global::MessageType;
use crate::port_strategy::PortStrategy;
use crate::tuning_params::FormattedTuningParams;

/// A trait that defines the interface for methods called by `Controller`
/// to make changes to the UI.
///
/// For the The `I` prefix, see `ITuner`s doc comment.
pub trait IUiMethods: Send + Sync {
    fn focus_port(&self, port_strategy: &dyn PortStrategy);
    fn get_selected_port_index(&self, port_strategy: &dyn PortStrategy) -> usize;
    fn set_selected_port_index(&self, index: usize, port_strategy: &dyn PortStrategy);
    fn set_devices_model(&self, device_names: &Vec<String>, port_strategy: &dyn PortStrategy);
    fn show_connected_device_name(&self, name: &str, msg_type: MessageType,
                                  port_strategy: &dyn PortStrategy);
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
