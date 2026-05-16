use std::cell::RefCell;
use pitchgrid_continuum::global::MessageType;
use pitchgrid_continuum::i_ui_methods::IUiMethods;
use pitchgrid_continuum::port_strategy::PortStrategy;
use pitchgrid_continuum::tuner::FormattedTuningParams;

/// Returns a clone of the current `UiMethodsState`.
pub fn ui_state() -> UiMethodsState {
    UI_STATE.with(|s| s.borrow().clone())
}

pub struct MockUiMethods {}

impl MockUiMethods {
    pub fn new() -> Self {
        UI_STATE.replace(UiMethodsState::new());
        MockUiMethods {}
    }
}

impl IUiMethods for MockUiMethods {
    #[allow(dead_code)]
    fn focus_port(&self, port_strategy: &dyn PortStrategy) {
        UI_STATE.with_borrow_mut(|s| {
            s.focus_port_count += 1;
            s.last_focus_port_port_strategy = Some(port_strategy.clone_box());
        });
    }

    #[allow(dead_code)]
    fn get_selected_port_index(&self, port_strategy: &dyn PortStrategy) -> usize {
        UI_STATE.with_borrow_mut(|s| {
            s.get_selected_port_index_count += 1;
            s.last_get_selected_port_index_port_strategy = Some(port_strategy.clone_box());
        });
        0
    }

    #[allow(dead_code)]
    fn set_selected_port_index(&self, index: usize, port_strategy: &dyn PortStrategy) {
        UI_STATE.with_borrow_mut(|s| {
            s.set_selected_port_index_count += 1;
            s.last_set_selected_port_index_index = Some(index);
            s.last_set_selected_port_index_port_strategy = Some(port_strategy.clone_box());
        });
    }

    #[allow(dead_code)]
    fn set_devices_model(&self, device_names: &Vec<String>, port_strategy: &dyn PortStrategy) {
        UI_STATE.with_borrow_mut(|s| {
            s.set_devices_model_count += 1;
            s.last_set_devices_model_device_names = Some(device_names.clone());
            s.last_set_devices_model_port_strategy = Some(port_strategy.clone_box());
        });
    }

    #[allow(dead_code)]
    fn show_connected_device_name(&self, name: &str, msg_type: MessageType,
                                  port_strategy: &dyn PortStrategy) {
        UI_STATE.with_borrow_mut(|s| {
            s.show_connected_device_name_count += 1;
            s.last_show_connected_device_name_name = Some(name.to_string());
            s.last_show_connected_device_name_msg_type = Some(msg_type);
            s.last_show_connected_device_name_port_strategy = Some(port_strategy.clone_box());
        });
    }

    #[allow(dead_code)]
    fn show_message(&self, msg: &str, msg_type: MessageType) {
        UI_STATE.with_borrow_mut(|s| {
            s.show_message_count += 1;
            s.last_show_message_msg = Some(msg.to_string());
            s.last_show_message_msg_type = Some(msg_type);
        });
    }

    #[allow(dead_code)]
    fn show_pitchgrid_status(&self, status: &str, msg_type: MessageType) {
        UI_STATE.with_borrow_mut(|s| {
            s.show_pitchgrid_status_count += 1;
            s.last_show_pitchgrid_status_status = Some(status.to_string());
            s.last_show_pitchgrid_status_msg_type = Some(msg_type);
        });
    }

    #[allow(dead_code)]
    fn show_tuning(&self, tuning: FormattedTuningParams, is_root_freq_overridden: bool) {
        UI_STATE.with_borrow_mut(|s| {
            s.show_tuning_count += 1;
            s.last_show_tuning_tuning = Some(tuning);
            s.last_show_tuning_is_root_freq_overridden = Some(is_root_freq_overridden);
        });
    }

    #[allow(dead_code)]
    fn set_main_window_position(&self, x: i32, y: i32) {
        UI_STATE.with_borrow_mut(|s| {
            s.set_main_window_position_count += 1;
            s.last_set_main_window_position_x = Some(x);
            s.last_set_main_window_position_y = Some(y);
        });
    }

    #[allow(dead_code)]
    fn set_override_rounding_initial(&self, value: bool) {
        UI_STATE.with_borrow_mut(|s| {
            s.set_override_rounding_initial_count += 1;
            s.last_set_override_rounding_initial_value = Some(value);
        });
    }

    #[allow(dead_code)]
    fn set_override_rounding_rate(&self, value: bool) {
        UI_STATE.with_borrow_mut(|s| {
            s.set_override_rounding_rate_count += 1;
            s.last_set_override_rounding_rate_value = Some(value);
        });
    }

    #[allow(dead_code)]
    fn set_rounding_rate(&self, rate: u8) {
        UI_STATE.with_borrow_mut(|s| {
            s.set_rounding_rate_count += 1;
            s.last_set_rounding_rate_rate = Some(rate);
        });
    }

    #[allow(dead_code)]
    fn set_selected_osc_listening_port_index(&self, index: i32) {
        UI_STATE.with_borrow_mut(|s| {
            s.set_selected_osc_listening_port_index_count += 1;
            s.last_set_selected_osc_listening_port_index_index = Some(index);
        });
    }

    #[allow(dead_code)]
    fn set_selected_pitch_table_index(&self, index: i32) {
        UI_STATE.with_borrow_mut(|s| {
            s.set_selected_pitch_table_index_count += 1;
            s.last_set_selected_pitch_table_index_index = Some(index);
        });
    }
}

pub struct UiMethodsState {
    pub focus_port_count: u16,
    pub last_focus_port_port_strategy: Option<Box<dyn PortStrategy>>,

    pub get_selected_port_index_count: u16,
    pub last_get_selected_port_index_port_strategy: Option<Box<dyn PortStrategy>>,

    pub set_selected_port_index_count: u16,
    pub last_set_selected_port_index_index: Option<usize>,
    pub last_set_selected_port_index_port_strategy: Option<Box<dyn PortStrategy>>,

    pub set_devices_model_count: u16,
    pub last_set_devices_model_device_names: Option<Vec<String>>,
    pub last_set_devices_model_port_strategy: Option<Box<dyn PortStrategy>>,

    pub show_connected_device_name_count: u16,
    pub last_show_connected_device_name_name: Option<String>,
    pub last_show_connected_device_name_msg_type: Option<MessageType>,
    pub last_show_connected_device_name_port_strategy: Option<Box<dyn PortStrategy>>,

    pub show_message_count: u16,
    pub last_show_message_msg: Option<String>,
    pub last_show_message_msg_type: Option<MessageType>,

    pub show_pitchgrid_status_count: u16,
    pub last_show_pitchgrid_status_status: Option<String>,
    pub last_show_pitchgrid_status_msg_type: Option<MessageType>,

    pub show_tuning_count: u16,
    pub last_show_tuning_tuning: Option<FormattedTuningParams>,
    pub last_show_tuning_is_root_freq_overridden: Option<bool>,

    pub set_main_window_position_count: u16,
    pub last_set_main_window_position_x: Option<i32>,
    pub last_set_main_window_position_y: Option<i32>,

    pub set_override_rounding_initial_count: u16,
    pub last_set_override_rounding_initial_value: Option<bool>,

    pub set_override_rounding_rate_count: u16,
    pub last_set_override_rounding_rate_value: Option<bool>,

    pub set_rounding_rate_count: u16,
    pub last_set_rounding_rate_rate: Option<u8>,

    pub set_selected_osc_listening_port_index_count: u16,
    pub last_set_selected_osc_listening_port_index_index: Option<i32>,

    pub set_selected_pitch_table_index_count: u16,
    pub last_set_selected_pitch_table_index_index: Option<i32>,
}

impl UiMethodsState {
    pub fn new() -> Self {
        UiMethodsState {
            focus_port_count: 0,
            last_focus_port_port_strategy: None,

            get_selected_port_index_count: 0,
            last_get_selected_port_index_port_strategy: None,

            set_selected_port_index_count: 0,
            last_set_selected_port_index_index: None,
            last_set_selected_port_index_port_strategy: None,

            set_devices_model_count: 0,
            last_set_devices_model_device_names: None,
            last_set_devices_model_port_strategy: None,

            show_connected_device_name_count: 0,
            last_show_connected_device_name_name: None,
            last_show_connected_device_name_msg_type: None,
            last_show_connected_device_name_port_strategy: None,

            show_message_count: 0,
            last_show_message_msg: None,
            last_show_message_msg_type: None,

            show_pitchgrid_status_count: 0,
            last_show_pitchgrid_status_status: None,
            last_show_pitchgrid_status_msg_type: None,

            show_tuning_count: 0,
            last_show_tuning_tuning: None,
            last_show_tuning_is_root_freq_overridden: None,

            set_main_window_position_count: 0,
            last_set_main_window_position_x: None,
            last_set_main_window_position_y: None,

            set_override_rounding_initial_count: 0,
            last_set_override_rounding_initial_value: None,

            set_override_rounding_rate_count: 0,
            last_set_override_rounding_rate_value: None,

            set_rounding_rate_count: 0,
            last_set_rounding_rate_rate: None,

            set_selected_osc_listening_port_index_count: 0,
            last_set_selected_osc_listening_port_index_index: None,

            set_selected_pitch_table_index_count: 0,
            last_set_selected_pitch_table_index_index: None,
        }
    }
}

impl Clone for UiMethodsState {
    fn clone(&self) -> Self {
        UiMethodsState {
            focus_port_count: self.focus_port_count,
            last_focus_port_port_strategy: self.last_focus_port_port_strategy.as_ref().map(|s| s.clone_box()),

            get_selected_port_index_count: self.get_selected_port_index_count,
            last_get_selected_port_index_port_strategy: self.last_get_selected_port_index_port_strategy.as_ref().map(|s| s.clone_box()),

            set_selected_port_index_count: self.set_selected_port_index_count,
            last_set_selected_port_index_index: self.last_set_selected_port_index_index,
            last_set_selected_port_index_port_strategy: self.last_set_selected_port_index_port_strategy.as_ref().map(|s| s.clone_box()),

            set_devices_model_count: self.set_devices_model_count,
            last_set_devices_model_device_names: self.last_set_devices_model_device_names.clone(),
            last_set_devices_model_port_strategy: self.last_set_devices_model_port_strategy.as_ref().map(|s| s.clone_box()),

            show_connected_device_name_count: self.show_connected_device_name_count,
            last_show_connected_device_name_name: self.last_show_connected_device_name_name.clone(),
            last_show_connected_device_name_msg_type: self.last_show_connected_device_name_msg_type.clone(),
            last_show_connected_device_name_port_strategy: self.last_show_connected_device_name_port_strategy.as_ref().map(|s| s.clone_box()),

            show_message_count: self.show_message_count,
            last_show_message_msg: self.last_show_message_msg.clone(),
            last_show_message_msg_type: self.last_show_message_msg_type.clone(),

            show_pitchgrid_status_count: self.show_pitchgrid_status_count,
            last_show_pitchgrid_status_status: self.last_show_pitchgrid_status_status.clone(),
            last_show_pitchgrid_status_msg_type: self.last_show_pitchgrid_status_msg_type.clone(),

            show_tuning_count: self.show_tuning_count,
            last_show_tuning_tuning: self.last_show_tuning_tuning.clone(),
            last_show_tuning_is_root_freq_overridden: self.last_show_tuning_is_root_freq_overridden,

            set_main_window_position_count: self.set_main_window_position_count,
            last_set_main_window_position_x: self.last_set_main_window_position_x,
            last_set_main_window_position_y: self.last_set_main_window_position_y,

            set_override_rounding_initial_count: self.set_override_rounding_initial_count,
            last_set_override_rounding_initial_value: self.last_set_override_rounding_initial_value,

            set_override_rounding_rate_count: self.set_override_rounding_rate_count,
            last_set_override_rounding_rate_value: self.last_set_override_rounding_rate_value,

            set_rounding_rate_count: self.set_rounding_rate_count,
            last_set_rounding_rate_rate: self.last_set_rounding_rate_rate,

            set_selected_osc_listening_port_index_count: self.set_selected_osc_listening_port_index_count,
            last_set_selected_osc_listening_port_index_index: self.last_set_selected_osc_listening_port_index_index,

            set_selected_pitch_table_index_count: self.set_selected_pitch_table_index_count,
            last_set_selected_pitch_table_index_index: self.last_set_selected_pitch_table_index_index,
        }
    }
}

thread_local! {
    static UI_STATE: RefCell<UiMethodsState> = RefCell::new(UiMethodsState::new());
}
