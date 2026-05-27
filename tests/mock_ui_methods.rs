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
            s.focus_port_strategy = Some(port_strategy.clone_box());
        });
    }

    #[allow(dead_code)]
    fn get_selected_port_index(&self, port_strategy: &dyn PortStrategy) -> usize {
        UI_STATE.with_borrow_mut(|s| {
            s.get_selected_port_index_count += 1;
            s.get_selected_port_index_port_strategy = Some(port_strategy.clone_box());
        });
        0
    }

    #[allow(dead_code)]
    fn set_selected_port_index(&self, index: usize, port_strategy: &dyn PortStrategy) {
        UI_STATE.with_borrow_mut(|s| {
            s.set_selected_port_index_count += 1;
            s.set_selected_port_index_index = Some(index);
            s.set_selected_port_index_port_strategy = Some(port_strategy.clone_box());
        });
    }

    #[allow(dead_code)]
    fn set_devices_model(&self, device_names: &Vec<String>, port_strategy: &dyn PortStrategy) {
        UI_STATE.with_borrow_mut(|s| {
            s.set_devices_model_count += 1;
            s.set_devices_model_device_names = Some(device_names.clone());
            s.set_devices_model_port_strategy = Some(port_strategy.clone_box());
        });
    }

    #[allow(dead_code)]
    fn show_connected_device_name(&self, name: &str, msg_type: MessageType,
                                  port_strategy: &dyn PortStrategy) {
        UI_STATE.with_borrow_mut(|s| {
            s.show_connected_device_name_count += 1;
            s.show_connected_device_name_name = Some(name.to_string());
            s.show_connected_device_name_msg_type = Some(msg_type);
            s.show_connected_device_name_port_strategy = Some(port_strategy.clone_box());
        });
    }

    #[allow(dead_code)]
    fn show_message(&self, msg: &str, msg_type: MessageType) {
        UI_STATE.with_borrow_mut(|s| {
            s.show_message_count += 1;
            s.show_message_msg = Some(msg.to_string());
            s.show_message_msg_type = Some(msg_type);
        });
    }

    #[allow(dead_code)]
    fn show_pitchgrid_status(&self, status: &str, msg_type: MessageType) {
        UI_STATE.with_borrow_mut(|s| {
            s.show_pitchgrid_status_count += 1;
            s.show_pitchgrid_status_msg = Some(status.to_string());
            s.show_pitchgrid_status_msg_type = Some(msg_type);
        });
    }

    #[allow(dead_code)]
    fn show_tuning(&self, tuning: FormattedTuningParams, is_root_freq_overridden: bool) {
        UI_STATE.with_borrow_mut(|s| {
            s.show_tuning_count += 1;
            s.show_tuning_formatted_tuning = Some(tuning);
            s.show_tuning_is_root_freq_overridden = Some(is_root_freq_overridden);
        });
    }

    #[allow(dead_code)]
    fn set_main_window_position(&self, x: i32, y: i32) {
        UI_STATE.with_borrow_mut(|s| {
            s.main_window_position_x = Some(x);
            s.main_window_position_y = Some(y);
        });
    }

    #[allow(dead_code)]
    fn set_override_rounding_initial(&self, value: bool) {
        UI_STATE.with_borrow_mut(|s| {
            s.override_rounding_initial = Some(value);
        });
    }

    #[allow(dead_code)]
    fn set_override_rounding_rate(&self, value: bool) {
        UI_STATE.with_borrow_mut(|s| {
            s.override_rounding_rate = Some(value);
        });
    }

    #[allow(dead_code)]
    fn set_rounding_rate(&self, rate: u8) {
        UI_STATE.with_borrow_mut(|s| {
            s.rounding_rate = Some(rate);
        });
    }

    #[allow(dead_code)]
    fn set_selected_osc_listening_port_index(&self, index: i32) {
        UI_STATE.with_borrow_mut(|s| {
            s.selected_osc_listening_port_index = Some(index);
        });
    }

    #[allow(dead_code)]
    fn set_selected_pitch_table_index(&self, index: i32) {
        UI_STATE.with_borrow_mut(|s| {
            s.selected_pitch_table_index = Some(index);
        });
    }
}

pub struct UiMethodsState {
    pub focus_port_count: u16,
    pub focus_port_strategy: Option<Box<dyn PortStrategy>>,

    pub get_selected_port_index_count: u16,
    pub get_selected_port_index_port_strategy: Option<Box<dyn PortStrategy>>,

    pub set_selected_port_index_count: u16,
    pub set_selected_port_index_index: Option<usize>,
    pub set_selected_port_index_port_strategy: Option<Box<dyn PortStrategy>>,

    pub set_devices_model_count: u16,
    pub set_devices_model_device_names: Option<Vec<String>>,
    pub set_devices_model_port_strategy: Option<Box<dyn PortStrategy>>,

    pub show_connected_device_name_count: u16,
    pub show_connected_device_name_name: Option<String>,
    pub show_connected_device_name_msg_type: Option<MessageType>,
    pub show_connected_device_name_port_strategy: Option<Box<dyn PortStrategy>>,

    pub show_message_count: u16,
    pub show_message_msg: Option<String>,
    pub show_message_msg_type: Option<MessageType>,

    pub show_pitchgrid_status_count: u16,
    pub show_pitchgrid_status_msg: Option<String>,
    pub show_pitchgrid_status_msg_type: Option<MessageType>,

    pub show_tuning_count: u16,
    pub show_tuning_formatted_tuning: Option<FormattedTuningParams>,
    pub show_tuning_is_root_freq_overridden: Option<bool>,

    pub main_window_position_x: Option<i32>,
    pub main_window_position_y: Option<i32>,

    pub override_rounding_initial: Option<bool>,
    pub override_rounding_rate: Option<bool>,
    pub rounding_rate: Option<u8>,
    pub selected_osc_listening_port_index: Option<i32>,
    pub selected_pitch_table_index: Option<i32>,
}

impl UiMethodsState {
    pub fn new() -> Self {
        UiMethodsState {
            focus_port_count: 0,
            focus_port_strategy: None,

            get_selected_port_index_count: 0,
            get_selected_port_index_port_strategy: None,

            set_selected_port_index_count: 0,
            set_selected_port_index_index: None,
            set_selected_port_index_port_strategy: None,

            set_devices_model_count: 0,
            set_devices_model_device_names: None,
            set_devices_model_port_strategy: None,

            show_connected_device_name_count: 0,
            show_connected_device_name_name: None,
            show_connected_device_name_msg_type: None,
            show_connected_device_name_port_strategy: None,

            show_message_count: 0,
            show_message_msg: None,
            show_message_msg_type: None,

            show_pitchgrid_status_count: 0,
            show_pitchgrid_status_msg: None,
            show_pitchgrid_status_msg_type: None,

            show_tuning_count: 0,
            show_tuning_formatted_tuning: None,
            show_tuning_is_root_freq_overridden: None,

            main_window_position_x: None,
            main_window_position_y: None,
            override_rounding_initial: None,
            override_rounding_rate: None,
            rounding_rate: None,
            selected_osc_listening_port_index: None,
            selected_pitch_table_index: None,
        }
    }
}

impl Clone for UiMethodsState {
    fn clone(&self) -> Self {
        UiMethodsState {
            focus_port_count: self.focus_port_count,
            focus_port_strategy: self.focus_port_strategy.as_ref().map(|s| s.clone_box()),

            get_selected_port_index_count: self.get_selected_port_index_count,
            get_selected_port_index_port_strategy: self.get_selected_port_index_port_strategy.as_ref().map(|s| s.clone_box()),

            set_selected_port_index_count: self.set_selected_port_index_count,
            set_selected_port_index_index: self.set_selected_port_index_index,
            set_selected_port_index_port_strategy: self.set_selected_port_index_port_strategy.as_ref().map(|s| s.clone_box()),

            set_devices_model_count: self.set_devices_model_count,
            set_devices_model_device_names: self.set_devices_model_device_names.clone(),
            set_devices_model_port_strategy: self.set_devices_model_port_strategy.as_ref().map(|s| s.clone_box()),

            show_connected_device_name_count: self.show_connected_device_name_count,
            show_connected_device_name_name: self.show_connected_device_name_name.clone(),
            show_connected_device_name_msg_type: self.show_connected_device_name_msg_type.clone(),
            show_connected_device_name_port_strategy: self.show_connected_device_name_port_strategy.as_ref().map(|s| s.clone_box()),

            show_message_count: self.show_message_count,
            show_message_msg: self.show_message_msg.clone(),
            show_message_msg_type: self.show_message_msg_type.clone(),

            show_pitchgrid_status_count: self.show_pitchgrid_status_count,
            show_pitchgrid_status_msg: self.show_pitchgrid_status_msg.clone(),
            show_pitchgrid_status_msg_type: self.show_pitchgrid_status_msg_type.clone(),

            show_tuning_count: self.show_tuning_count,
            show_tuning_formatted_tuning: self.show_tuning_formatted_tuning.clone(),
            show_tuning_is_root_freq_overridden: self.show_tuning_is_root_freq_overridden,

            main_window_position_x: self.main_window_position_x,
            main_window_position_y: self.main_window_position_y,
            override_rounding_initial: self.override_rounding_initial,
            override_rounding_rate: self.override_rounding_rate,
            rounding_rate: self.rounding_rate,
            selected_osc_listening_port_index: self.selected_osc_listening_port_index,
            selected_pitch_table_index: self.selected_pitch_table_index,
        }
    }
}

thread_local! {
    static UI_STATE: RefCell<UiMethodsState> = RefCell::new(UiMethodsState::new());
}
